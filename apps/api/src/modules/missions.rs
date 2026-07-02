use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{error::ApiError, pagination::{Pagination, PagedResponse}, state::AppState};

/// 无人矿卡任务模块
///
/// 为无人驾驶矿卡/自动驾驶系统提供标准化调度接口。
/// 无人驾驶中控系统通过此模块获取任务、上报状态。
///
/// 对接方式:
///   - 无人驾驶中控调用 POST /api/v1/missions 获取新任务
///   - 无人驾驶中控调用 POST /api/v1/missions/:id/status 上报状态
///   - 无人驾驶中控调用 GET /api/v1/missions/active 查询当前任务
///
/// 与现有 waybill 模块的关系:
///   - waybill 是"业务运单"（谁、去哪、拉什么）
///   - mission 是"执行任务"（无人车具体的作业指令）
///   - 一个 waybill 可对应多个 mission（如多车次）
pub fn router() -> Router<AppState> {
    Router::new()
        // 无人驾驶中控：拉取待执行任务
        .route("/pending", get(list_pending_missions))
        // 无人驾驶中控：领取任务
        .route("/{mission_id}/claim", post(claim_mission))
        // 无人驾驶中控：上报任务状态
        .route("/{mission_id}/status", post(report_mission_status))
        // 无人驾驶中控：完成任务
        .route("/{mission_id}/complete", post(complete_mission))
        // 后台/调度：创建无人车任务（由自动调度或人工创建）
        .route("/", post(create_mission))
        // 后台/调度：查看任务列表
        .route("/", get(list_all_missions))
}

// ─── 数据模型 ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "mission_status", rename_all = "lowercase")]
pub enum MissionStatus {
    Pending,      // 待执行
    Claimed,      // 已被无人车领取
    InProgress,   // 执行中
    Completed,    // 已完成
    Failed,       // 执行失败
    Cancelled,    // 已取消
}

#[derive(Debug, Deserialize)]
struct CompleteMissionRequest {
    result: Option<String>,
    actual_weight_ton: Option<f64>,
    error_message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateMissionRequest {
    /// 关联运单ID（可选，无人车独立任务时可为空）
    pub waybill_id: Option<Uuid>,
    /// 车辆ID（无人车标识）
    pub vehicle_id: String,
    /// 任务类型: loading / hauling / dumping
    pub mission_type: String,
    /// 起点坑口
    pub source_pit_id: Uuid,
    /// 目的地（卸货点/排土场）
    pub destination: String,
    /// 优先级（越高越优先）
    pub priority: i32,
    /// 预计载重(吨)
    pub estimated_weight_ton: Option<f64>,
    /// 任务参数（无人驾驶系统自定义）
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MissionStatusReport {
    /// 无人驾驶系统上报的状态
    pub status: String,
    /// 当前位置(经度)
    pub position_lng: Option<f64>,
    /// 当前位置(纬度)
    pub position_lat: Option<f64>,
    /// 当前载重(吨)
    pub payload_weight: Option<f64>,
    /// 电量/油量(%)
    pub battery_level: Option<f32>,
    /// 错误信息（失败时）
    pub error_message: Option<String>,
    /// 预计完成时间
    pub estimated_completion: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MissionResponse {
    pub id: Uuid,
    pub waybill_id: Option<Uuid>,
    pub vehicle_id: String,
    pub mission_type: String,
    pub source_pit_id: Uuid,
    pub destination: String,
    pub priority: i32,
    pub status: String,
    pub estimated_weight_ton: Option<f64>,
    pub params: Option<serde_json::Value>,
    pub claimed_by: Option<String>,
    pub claimed_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct MissionRow {
    id: Uuid,
    waybill_id: Option<Uuid>,
    vehicle_id: String,
    mission_type: String,
    source_pit_id: Uuid,
    destination: String,
    priority: i32,
    status: String,
    estimated_weight_ton: Option<f64>,
    params: Option<serde_json::Value>,
    claimed_by: Option<String>,
    claimed_at: Option<DateTime<Utc>>,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

// ─── API 实现 ──────────────────────────────────────────────────────────────

/// 无人驾驶中控拉取待执行任务（轮询接口）
async fn list_pending_missions(
    State(state): State<AppState>,
) -> Result<Json<Vec<MissionResponse>>, ApiError> {
    let rows = sqlx::query_as::<_, MissionRow>(
        r#"
        SELECT id, waybill_id, vehicle_id, mission_type, source_pit_id, destination,
               priority, status::text AS status, estimated_weight_ton, params,
               claimed_by, claimed_at, started_at, completed_at, created_at
        FROM missions
        WHERE status = 'pending'
        ORDER BY priority DESC, created_at ASC
        LIMIT 50
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("failed to fetch pending missions: {e}")))?;

    Ok(Json(rows.into_iter().map(map_mission).collect()))
}

#[derive(Deserialize)]
struct ClaimMissionRequest {
    vehicle_id: String,
}

/// 无人驾驶中控领取任务
async fn claim_mission(
    State(state): State<AppState>,
    Path(mission_id): Path<Uuid>,
    Json(claim): Json<ClaimMissionRequest>,
) -> Result<Json<MissionResponse>, ApiError> {
    if claim.vehicle_id.trim().is_empty() {
        return Err(ApiError::bad_request("vehicle_id is required"));
    }

    let now = Utc::now();
    let row = sqlx::query_as::<_, MissionRow>(
        r#"
        UPDATE missions
        SET status = 'claimed', claimed_by = $2, claimed_at = $3, updated_at = $3
        WHERE id = $1 AND status = 'pending'
        RETURNING id, waybill_id, vehicle_id, mission_type, source_pit_id, destination,
                  priority, status::text AS status, estimated_weight_ton, params,
                  claimed_by, claimed_at, started_at, completed_at, created_at
        "#,
    )
    .bind(mission_id)
    .bind(&claim.vehicle_id)
    .bind(now)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("failed to claim mission: {e}")))?;

    let row = row.ok_or_else(|| ApiError::conflict("mission not found or already claimed"))?;

    Ok(Json(map_mission(row)))
}

/// 无人驾驶中控上报任务状态（位置、载重、电量等）
async fn report_mission_status(
    State(state): State<AppState>,
    Path(mission_id): Path<Uuid>,
    Json(report): Json<MissionStatusReport>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // 记录状态日志
    sqlx::query(
        r#"
        INSERT INTO mission_status_logs
            (mission_id, status, position_lng, position_lat, payload_weight,
             battery_level, error_message, estimated_completion)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
    .bind(mission_id)
    .bind(&report.status)
    .bind(report.position_lng)
    .bind(report.position_lat)
    .bind(report.payload_weight)
    .bind(report.battery_level)
    .bind(&report.error_message)
    .bind(report.estimated_completion)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("failed to log mission status: {e}")))?;

    // 如果状态是 in_progress，更新 started_at
    if report.status == "in_progress" {
        sqlx::query(
            "UPDATE missions SET status = 'in_progress', started_at = COALESCE(started_at, $2), \
             updated_at = $2 WHERE id = $1 AND status = 'claimed'",
        )
        .bind(mission_id)
        .bind(Utc::now())
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("failed to update mission status: {e}")))?;
    }

    Ok(Json(serde_json::json!({
        "mission_id": mission_id,
        "status": report.status,
        "received_at": Utc::now(),
    })))
}

/// 无人驾驶中控完成任务
async fn complete_mission(
    State(state): State<AppState>,
    Path(mission_id): Path<Uuid>,
    Json(complete): Json<CompleteMissionRequest>,
) -> Result<Json<MissionResponse>, ApiError> {
    let result = complete.result.as_deref().unwrap_or("completed");
    let new_status = if result == "failed" { "failed" } else { "completed" };

    let now = Utc::now();
    let row = sqlx::query_as::<_, MissionRow>(
        r#"
        UPDATE missions
        SET status = $2, completed_at = $3, updated_at = $3,
            actual_weight_ton = COALESCE($4, actual_weight_ton),
            error_message = $5
        WHERE id = $1 AND status IN ('claimed', 'in_progress')
        RETURNING id, waybill_id, vehicle_id, mission_type, source_pit_id, destination,
                  priority, status::text AS status, estimated_weight_ton, params,
                  claimed_by, claimed_at, started_at, completed_at, created_at
        "#,
    )
    .bind(mission_id)
    .bind(new_status)
    .bind(now)
    .bind(complete.actual_weight_ton)
    .bind(&complete.error_message)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("failed to complete mission: {e}")))?;

    let row = row.ok_or_else(|| ApiError::conflict("mission not found or already completed"))?;

    // 如果关联了 waybill，自动更新 waybill 状态
    if let Some(wb_id) = row.waybill_id {
        if new_status == "completed" {
            let _ = sqlx::query(
                "UPDATE waybills SET status = 'completed', completed_at = $2, \
                 actual_weight_ton = COALESCE($3, actual_weight_ton), \
                 updated_at = $2, version = version + 1 \
                 WHERE id = $1 AND status IN ('loaded', 'weighing')",
            )
            .bind(wb_id)
            .bind(now)
            .bind(complete.actual_weight_ton)
            .execute(&state.db)
            .await;
        }
    }

    Ok(Json(map_mission(row)))
}

/// 创建无人车任务（调度后台/自动调度系统调用）
async fn create_mission(
    State(state): State<AppState>,
    Json(req): Json<CreateMissionRequest>,
) -> Result<Json<MissionResponse>, ApiError> {
    let row = sqlx::query_as::<_, MissionRow>(
        r#"
        INSERT INTO missions
            (waybill_id, vehicle_id, mission_type, source_pit_id, destination,
             priority, estimated_weight_ton, params)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id, waybill_id, vehicle_id, mission_type, source_pit_id, destination,
                  priority, status::text AS status, estimated_weight_ton, params,
                  claimed_by, claimed_at, started_at, completed_at, created_at
        "#,
    )
    .bind(req.waybill_id)
    .bind(&req.vehicle_id)
    .bind(&req.mission_type)
    .bind(req.source_pit_id)
    .bind(&req.destination)
    .bind(req.priority)
    .bind(req.estimated_weight_ton)
    .bind(req.params)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("failed to create mission: {e}")))?;

    Ok(Json(map_mission(row)))
}

/// 查看所有任务（后台管理）
async fn list_all_missions(
    State(state): State<AppState>,
    axum::extract::Query(query): axum::extract::Query<Pagination>,
) -> Result<Json<PagedResponse<MissionResponse>>, ApiError> {
    let (offset, limit) = query.offset_limit();

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*)::bigint FROM missions")
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("failed to count missions: {e}")))?;

    let rows = sqlx::query_as::<_, MissionRow>(
        r#"
        SELECT id, waybill_id, vehicle_id, mission_type, source_pit_id, destination,
               priority, status::text AS status, estimated_weight_ton, params,
               claimed_by, claimed_at, started_at, completed_at, created_at
        FROM missions
        ORDER BY created_at DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("failed to list missions: {e}")))?;

    let data: Vec<MissionResponse> = rows.into_iter().map(map_mission).collect();

    Ok(Json(PagedResponse::new(
        data,
        total,
        query.page(),
        query.page_size(),
    )))
}

fn map_mission(row: MissionRow) -> MissionResponse {
    MissionResponse {
        id: row.id,
        waybill_id: row.waybill_id,
        vehicle_id: row.vehicle_id,
        mission_type: row.mission_type,
        source_pit_id: row.source_pit_id,
        destination: row.destination,
        priority: row.priority,
        status: row.status,
        estimated_weight_ton: row.estimated_weight_ton,
        params: row.params,
        claimed_by: row.claimed_by,
        claimed_at: row.claimed_at,
        started_at: row.started_at,
        completed_at: row.completed_at,
        created_at: row.created_at,
    }
}
