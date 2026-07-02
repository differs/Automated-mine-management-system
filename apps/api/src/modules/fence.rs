use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};

/// 电子围栏模块
///
/// 功能:
///   1. 坑口围栏管理（圆形/多边形）
///   2. 司机位置上报
///   3. 围栏自动判定 → 到场/离场事件
///   4. 车辆轨迹存储
///
/// 数据流:
///   司机App GPS → POST /api/v1/fence/report
///                 → 服务端围栏判定
///                 → 进入围栏 → 自动到场
///                 → 离开围栏 → 记录离场时间
pub fn router() -> Router<AppState> {
    Router::new()
        // 围栏管理
        .route("/fences", get(list_fences).post(create_fence))
        .route("/fences/{fence_id}", get(get_fence)
            .post(update_fence)
            .delete(delete_fence))
        .route("/fences/pit/{pit_id}", get(get_fences_by_pit))
        // 位置上报
        .route("/report", post(report_location))
        .route("/report/batch", post(report_location_batch))
        // 轨迹查询
        .route("/trail/{driver_id}", get(get_driver_trail))
        // 围栏事件
        .route("/events/{fence_id}", get(get_fence_events))
}

// ─── 数据模型 ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct FenceResponse {
    pub id: Uuid,
    pub pit_id: Uuid,
    pub name: String,
    pub fence_type: String,    // arrival / geofence / restricted
    pub shape: String,         // circle / polygon
    pub center_lat: f64,
    pub center_lng: f64,
    pub radius_meters: f64,
    pub polygon_points: Option<Vec<Point2D>>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Point2D {
    pub lat: f64,
    pub lng: f64,
}

#[derive(Debug, Deserialize)]
pub struct CreateFenceRequest {
    pub pit_id: Uuid,
    pub name: String,
    pub fence_type: String,
    pub shape: String,         // circle / polygon
    pub center_lat: f64,
    pub center_lng: f64,
    pub radius_meters: f64,
    pub polygon_points: Option<Vec<Point2D>>,
}

#[derive(Debug, Deserialize)]
pub struct LocationReport {
    pub driver_id: Uuid,
    pub lat: f64,
    pub lng: f64,
    pub accuracy: Option<f64>,
    pub speed: Option<f64>,
    pub bearing: Option<f64>,
    pub reported_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct BatchLocationReport {
    pub driver_id: Uuid,
    pub points: Vec<LocationReport>,
}

#[derive(Debug, Serialize)]
pub struct LocationResponse {
    pub accepted: bool,
    pub fence_events: Vec<FenceEvent>,
}

#[derive(Debug, Serialize)]
pub struct FenceEvent {
    pub fence_id: Uuid,
    pub fence_name: String,
    pub event_type: String,  // enter / exit / inside / outside
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct TrailPoint {
    pub lat: f64,
    pub lng: f64,
    pub speed: Option<f64>,
    pub reported_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct FenceEventLog {
    pub id: Uuid,
    pub driver_id: Uuid,
    pub fence_id: Uuid,
    pub event_type: String,
    pub lat: f64,
    pub lng: f64,
    pub occurred_at: DateTime<Utc>,
}

// ─── API 实现 ──────────────────────────────────────────────────────────────

/// 列表所有围栏
async fn list_fences(
    State(state): State<AppState>,
) -> Result<Json<Vec<FenceResponse>>, ApiError> {
    let rows = sqlx::query(
        r#"SELECT id, pit_id, name, fence_type, shape, center_lat, center_lng,
                  radius_meters, polygon_points, is_active, created_at
           FROM geo_fences ORDER BY created_at DESC"#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("failed to list fences: {e}")))?;

    let fences = rows.iter().map(row_to_fence).collect::<Result<Vec<_>, _>>()
        .map_err(|e| ApiError::internal(format!("row mapping: {e}")))?;

    Ok(Json(fences))
}

/// 创建围栏
async fn create_fence(
    State(state): State<AppState>,
    Json(req): Json<CreateFenceRequest>,
) -> Result<Json<FenceResponse>, ApiError> {
    let polygon_json = req.polygon_points.as_ref()
        .map(|p| serde_json::to_value(p).unwrap_or_default())
        .unwrap_or(serde_json::Value::Null);

    let row = sqlx::query(
        r#"INSERT INTO geo_fences (pit_id, name, fence_type, shape, center_lat, center_lng,
            radius_meters, polygon_points)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
           RETURNING id, pit_id, name, fence_type, shape, center_lat, center_lng,
                     radius_meters, polygon_points, is_active, created_at"#,
    )
    .bind(req.pit_id)
    .bind(&req.name)
    .bind(&req.fence_type)
    .bind(&req.shape)
    .bind(req.center_lat)
    .bind(req.center_lng)
    .bind(req.radius_meters)
    .bind(&polygon_json)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("failed to create fence: {e}")))?;

    row_to_fence(&row).map(Json).map_err(|e| ApiError::internal(format!("row mapping: {e}")))
}

/// 获取单个围栏
async fn get_fence(
    State(state): State<AppState>,
    Path(fence_id): Path<Uuid>,
) -> Result<Json<FenceResponse>, ApiError> {
    let row = sqlx::query(
        r#"SELECT id, pit_id, name, fence_type, shape, center_lat, center_lng,
                  radius_meters, polygon_points, is_active, created_at
           FROM geo_fences WHERE id = $1"#,
    )
    .bind(fence_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("failed to fetch fence: {e}")))?;

    let Some(row) = row else {
        return Err(ApiError::not_found("fence not found"));
    };

    row_to_fence(&row).map(Json).map_err(|e| ApiError::internal(format!("row mapping: {e}")))
}

/// 更新围栏
async fn update_fence(
    State(state): State<AppState>,
    Path(fence_id): Path<Uuid>,
    Json(req): Json<CreateFenceRequest>,
) -> Result<Json<FenceResponse>, ApiError> {
    let polygon_json = req.polygon_points.as_ref()
        .map(|p| serde_json::to_value(p).unwrap_or_default())
        .unwrap_or(serde_json::Value::Null);

    let row = sqlx::query(
        r#"UPDATE geo_fences SET pit_id=$2, name=$3, fence_type=$4, shape=$5,
            center_lat=$6, center_lng=$7, radius_meters=$8, polygon_points=$9,
            updated_at=NOW()
           WHERE id=$1
           RETURNING id, pit_id, name, fence_type, shape, center_lat, center_lng,
                     radius_meters, polygon_points, is_active, created_at"#,
    )
    .bind(fence_id)
    .bind(req.pit_id)
    .bind(&req.name)
    .bind(&req.fence_type)
    .bind(&req.shape)
    .bind(req.center_lat)
    .bind(req.center_lng)
    .bind(req.radius_meters)
    .bind(&polygon_json)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("failed to update fence: {e}")))?;

    let Some(row) = row else {
        return Err(ApiError::not_found("fence not found"));
    };

    row_to_fence(&row).map(Json).map_err(|e| ApiError::internal(format!("row mapping: {e}")))
}

/// 删除围栏
async fn delete_fence(
    State(state): State<AppState>,
    Path(fence_id): Path<Uuid>,
) -> Result<Json<()>, ApiError> {
    sqlx::query("DELETE FROM geo_fences WHERE id = $1")
        .bind(fence_id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("failed to delete fence: {e}")))?;

    Ok(Json(()))
}

/// 按坑口获取围栏
async fn get_fences_by_pit(
    State(state): State<AppState>,
    Path(pit_id): Path<Uuid>,
) -> Result<Json<Vec<FenceResponse>>, ApiError> {
    let rows = sqlx::query(
        r#"SELECT id, pit_id, name, fence_type, shape, center_lat, center_lng,
                  radius_meters, polygon_points, is_active, created_at
           FROM geo_fences WHERE pit_id = $1 ORDER BY created_at"#,
    )
    .bind(pit_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("failed to fetch pit fences: {e}")))?;

    let fences = rows.iter().map(row_to_fence).collect::<Result<Vec<_>, _>>()
        .map_err(|e| ApiError::internal(format!("row mapping: {e}")))?;

    Ok(Json(fences))
}

/// 上报单条位置
async fn report_location(
    State(state): State<AppState>,
    Json(loc): Json<LocationReport>,
) -> Result<Json<LocationResponse>, ApiError> {
    process_location(&state, &loc).await
}

/// 批量上报位置（离线缓存后批量提交）
async fn report_location_batch(
    State(state): State<AppState>,
    Json(batch): Json<BatchLocationReport>,
) -> Result<Json<Vec<LocationResponse>>, ApiError> {
    let mut results = Vec::with_capacity(batch.points.len());
    for point in batch.points {
        let resp = process_location(&state, &point).await?;
        results.push(resp.0);
    }
    Ok(Json(results))
}

/// 获取司机轨迹
async fn get_driver_trail(
    State(state): State<AppState>,
    Path(driver_id): Path<Uuid>,
) -> Result<Json<Vec<TrailPoint>>, ApiError> {
    let rows = sqlx::query(
        r#"SELECT lat, lng, speed, reported_at
           FROM location_reports
           WHERE driver_id = $1
           ORDER BY reported_at DESC LIMIT 500"#,
    )
    .bind(driver_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("failed to fetch trail: {e}")))?;

    let trail = rows.iter().map(|row| TrailPoint {
        lat: row.get("lat"),
        lng: row.get("lng"),
        speed: row.get("speed"),
        reported_at: row.get("reported_at"),
    }).collect();

    Ok(Json(trail))
}

/// 获取围栏事件日志
async fn get_fence_events(
    State(state): State<AppState>,
    Path(fence_id): Path<Uuid>,
) -> Result<Json<Vec<FenceEventLog>>, ApiError> {
    let rows = sqlx::query(
        r#"SELECT id, driver_id, fence_id, event_type, lat, lng, occurred_at
           FROM fence_event_logs
           WHERE fence_id = $1
           ORDER BY occurred_at DESC LIMIT 100"#,
    )
    .bind(fence_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("failed to fetch fence events: {e}")))?;

    let events = rows.iter().map(|row| FenceEventLog {
        id: row.get("id"),
        driver_id: row.get("driver_id"),
        fence_id: row.get("fence_id"),
        event_type: row.get("event_type"),
        lat: row.get("lat"),
        lng: row.get("lng"),
        occurred_at: row.get("occurred_at"),
    }).collect();

    Ok(Json(events))
}

// ─── 核心逻辑 ──────────────────────────────────────────────────────────────

/// 处理一条位置上报
async fn process_location(
    state: &AppState,
    loc: &LocationReport,
) -> Result<Json<LocationResponse>, ApiError> {
    // 1. 存储位置
    sqlx::query(
        r#"INSERT INTO location_reports (driver_id, lat, lng, accuracy, speed, bearing, reported_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
    )
    .bind(loc.driver_id)
    .bind(loc.lat)
    .bind(loc.lng)
    .bind(loc.accuracy.unwrap_or(0.0))
    .bind(loc.speed.unwrap_or(0.0))
    .bind(loc.bearing)
    .bind(loc.reported_at)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("failed to store location: {e}")))?;

    // 2. 获取该司机相关的活跃围栏
    let fences = sqlx::query(
        r#"SELECT gf.* FROM geo_fences gf
           JOIN pits p ON gf.pit_id = p.id
           JOIN waybills w ON w.pit_id = p.id
           WHERE w.driver_id = $1 AND w.status IN ('dispatched', 'arrived')
           AND gf.is_active = true"#,
    )
    .bind(loc.driver_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("failed to fetch relevant fences: {e}")))?;

    let mut events = Vec::new();
    let now = Utc::now();

    for fence in &fences {
        let fence_id: Uuid = fence.get("id");
        let pit_id: Uuid = fence.get("pit_id");
        let name: String = fence.get("name");
        let fence_type: String = fence.get("fence_type");
        let _shape: String = fence.get("shape");
        let center_lat: f64 = fence.get("center_lat");
        let center_lng: f64 = fence.get("center_lng");
        let radius: f64 = fence.get("radius_meters");

        // 计算距离
        let distance = haversine_distance(loc.lat, loc.lng, center_lat, center_lng);
        let inside = distance <= radius;

        // 获取上次状态
        let prev_inside = sqlx::query_scalar::<_, bool>(
            r#"SELECT inside FROM driver_fence_states
               WHERE driver_id = $1 AND fence_id = $2"#,
        )
        .bind(loc.driver_id)
        .bind(fence_id)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten()
        .unwrap_or(false);

        if inside && !prev_inside {
            // 进入围栏
            sqlx::query(
                r#"INSERT INTO driver_fence_states (driver_id, fence_id, inside, entered_at, updated_at)
                   VALUES ($1, $2, true, $3, $3)
                   ON CONFLICT (driver_id, fence_id) DO UPDATE SET inside=true, entered_at=$3, updated_at=$3"#,
            )
            .bind(loc.driver_id)
            .bind(fence_id)
            .bind(now)
            .execute(&state.db)
            .await
            .ok();

            // 记录事件
            sqlx::query(
                r#"INSERT INTO fence_event_logs (driver_id, fence_id, event_type, lat, lng, occurred_at)
                   VALUES ($1, $2, 'enter', $3, $4, $5)"#,
            )
            .bind(loc.driver_id)
            .bind(fence_id)
            .bind(loc.lat)
            .bind(loc.lng)
            .bind(now)
            .execute(&state.db)
            .await
            .ok();

            events.push(FenceEvent {
                fence_id,
                fence_name: name.clone(),
                event_type: "enter".into(),
                occurred_at: now,
            });

            // 如果围栏类型是 arrival，自动到场
            if fence_type == "arrival" {
                let _ = auto_arrive(state, loc.driver_id, pit_id).await;
            }
        } else if !inside && prev_inside {
            // 离开围栏
            sqlx::query(
                r#"UPDATE driver_fence_states SET inside=false, updated_at=$3
                   WHERE driver_id=$1 AND fence_id=$2"#,
            )
            .bind(loc.driver_id)
            .bind(fence_id)
            .bind(now)
            .execute(&state.db)
            .await
            .ok();

            sqlx::query(
                r#"INSERT INTO fence_event_logs (driver_id, fence_id, event_type, lat, lng, occurred_at)
                   VALUES ($1, $2, 'exit', $3, $4, $5)"#,
            )
            .bind(loc.driver_id)
            .bind(fence_id)
            .bind(loc.lat)
            .bind(loc.lng)
            .bind(now)
            .execute(&state.db)
            .await
            .ok();

            events.push(FenceEvent {
                fence_id,
                fence_name: name,
                event_type: "exit".into(),
                occurred_at: now,
            });
        }
    }

    Ok(Json(LocationResponse {
        accepted: true,
        fence_events: events,
    }))
}

/// 自动到场
async fn auto_arrive(
    state: &AppState,
    driver_id: Uuid,
    pit_id: Uuid,
) -> Result<(), ApiError> {
    let now = Utc::now();

    let result = sqlx::query(
        r#"UPDATE waybills SET status='arrived', arrive_time=$2, arrival_source='geo_fence',
            updated_at=$2, version=version+1
           WHERE driver_id=$1 AND pit_id=$3 AND status='dispatched'
           RETURNING id"#,
    )
    .bind(driver_id)
    .bind(now)
    .bind(pit_id)
    .execute(&state.db)
    .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => {
            tracing::info!("driver {driver_id} auto-arrived at pit {pit_id} via geo-fence");
            Ok(())
        }
        Ok(_) => Ok(()), // 没有待派单或已到场，不做处理
        Err(e) => Err(ApiError::internal(format!("auto-arrive failed: {e}"))),
    }
}

/// Haversine 距离计算（米）
fn haversine_distance(lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> f64 {
    const R: f64 = 6_371_000.0; // 地球半径（米）

    let dlat = (lat2 - lat1).to_radians();
    let dlng = (lng2 - lng1).to_radians();
    let a = (dlat / 2.0).sin().powi(2)
          + lat1.to_radians().cos()
          * lat2.to_radians().cos()
          * (dlng / 2.0).sin().powi(2);

    R * 2.0 * a.sqrt().asin()
}

fn row_to_fence(row: &sqlx::postgres::PgRow) -> Result<FenceResponse, sqlx::Error> {
    let polygon_json: Option<serde_json::Value> = row.try_get("polygon_points")?;
    let polygon_points = polygon_json
        .and_then(|v| serde_json::from_value::<Vec<Point2D>>(v).ok());

    Ok(FenceResponse {
        id: row.try_get("id")?,
        pit_id: row.try_get("pit_id")?,
        name: row.try_get("name")?,
        fence_type: row.try_get("fence_type")?,
        shape: row.try_get("shape")?,
        center_lat: row.try_get("center_lat")?,
        center_lng: row.try_get("center_lng")?,
        radius_meters: row.try_get("radius_meters")?,
        polygon_points,
        is_active: row.try_get("is_active")?,
        created_at: row.try_get("created_at")?,
    })
}
