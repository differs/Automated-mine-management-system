use axum::{Json, Router, extract::State, routing::get};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/overview", get(overview))
        .route("/pit-efficiency", get(pit_efficiency))
        .route("/driver-ranking", get(driver_ranking))
}

#[derive(Serialize, Deserialize)]
pub struct OverviewResponse {
    pub today_total_waybills: i64,
    pub today_completed: i64,
    pub today_cancelled: i64,
    pub in_progress: i64,
    pub today_total_tonnage: f64,
    pub pit_summaries: Vec<PitSummary>,
    pub date: String,
}

#[derive(Serialize, Deserialize)]
pub struct PitSummary {
    pub pit_id: Uuid,
    pub pit_name: String,
    pub current_queue: i32,
    pub avg_wait_minutes: i32,
    pub today_trips: i64,
    pub today_tonnage: f64,
}

#[derive(Serialize)]
pub struct PitEfficiencyRow {
    pub pit_id: Uuid,
    pub pit_name: String,
    pub today_trips: i64,
    pub today_tonnage: f64,
    pub avg_wait_minutes: f64,
    pub avg_loading_minutes: f64,
}

#[derive(Serialize)]
pub struct DriverRankingRow {
    pub driver_id: Uuid,
    pub driver_name: String,
    pub license_plate: String,
    pub today_trips: i64,
    pub today_tonnage: f64,
}

async fn overview(
    State(state): State<AppState>,
) -> Result<Json<OverviewResponse>, ApiError> {
    // ── Redis 缓存读取（30秒 TTL）─────────────────────────────────
    let cache_key = "dashboard:overview";
    if let Ok(cached) = state.redis.clone().get::<_, String>(cache_key).await {
        if let Ok(response) = serde_json::from_str::<OverviewResponse>(&cached) {
            return Ok(Json(response));
        }
    }

    let today = chrono::Utc::now().date_naive();

    // Count today's waybills by status
    let count_rows = sqlx::query(
        "SELECT status::text AS status, COUNT(*)::bigint AS cnt \
         FROM waybills WHERE created_at::date = $1 \
         GROUP BY status",
    )
    .bind(today)
    .fetch_all(&state.db)
    .await
    .map_err(|err| ApiError::internal(format!("failed to query overview: {err}")))?;

    let mut total = 0i64;
    let mut completed = 0i64;
    let mut cancelled = 0i64;
    let mut in_progress = 0i64;

    for row in &count_rows {
        let status: String = row.get("status");
        let cnt: i64 = row.get("cnt");
        total += cnt;
        match status.as_str() {
            "completed" => completed += cnt,
            "cancelled" => cancelled += cnt,
            _ => in_progress += cnt,
        }
    }

    // Total tonnage today
    let tonnage: Option<f64> = sqlx::query_scalar(
        "SELECT SUM(actual_weight_ton)::double precision \
         FROM waybills WHERE status = 'completed' AND completed_time::date = $1",
    )
    .bind(today)
    .fetch_one(&state.db)
    .await
    .map_err(|err| ApiError::internal(format!("failed to query tonnage: {err}")))?;
    let today_total_tonnage = tonnage.unwrap_or(0.0);

    // Pit summaries
    let pit_rows = sqlx::query(
        "SELECT p.id AS pit_id, p.name AS pit_name, \
         p.current_queue_count AS current_queue, \
         p.avg_wait_minutes AS avg_wait_minutes, \
         COALESCE(t.today_trips, 0)::bigint AS today_trips, \
         COALESCE(t.today_tonnage, 0.0)::double precision AS today_tonnage \
         FROM pits p \
         LEFT JOIN LATERAL ( \
           SELECT COUNT(*)::bigint AS today_trips, \
                  COALESCE(SUM(w.actual_weight_ton), 0.0)::double precision AS today_tonnage \
           FROM waybills w \
           WHERE w.pit_id = p.id AND w.completed_time::date = $1 AND w.status = 'completed' \
         ) t ON TRUE \
         WHERE p.is_active = TRUE \
         ORDER BY p.name",
    )
    .bind(today)
    .fetch_all(&state.db)
    .await
    .map_err(|err| ApiError::internal(format!("failed to query pit summaries: {err}")))?;

    let pit_summaries: Vec<PitSummary> = pit_rows.iter().map(|row| {
        PitSummary {
            pit_id: row.get("pit_id"),
            pit_name: row.get("pit_name"),
            current_queue: row.get("current_queue"),
            avg_wait_minutes: row.get("avg_wait_minutes"),
            today_trips: row.get("today_trips"),
            today_tonnage: row.get("today_tonnage"),
        }
    }).collect();

    let response = OverviewResponse {
        today_total_waybills: total,
        today_completed: completed,
        today_cancelled: cancelled,
        in_progress,
        today_total_tonnage,
        pit_summaries,
        date: today.to_string(),
    };

    // ── Redis 缓存写入（30秒 TTL）─────────────────────────────────
    if let Ok(json) = serde_json::to_string(&response) {
        let _: Result<(), _> = state.redis.clone().set_ex(cache_key, &json, 30).await;
    }

    Ok(Json(response))
}

async fn pit_efficiency(
    State(state): State<AppState>,
) -> Result<Json<Vec<PitEfficiencyRow>>, ApiError> {
    let today = chrono::Utc::now().date_naive();

    let rows = sqlx::query(
        "SELECT p.id AS pit_id, p.name AS pit_name, \
         COALESCE(t.today_trips, 0)::bigint AS today_trips, \
         COALESCE(t.today_tonnage, 0.0)::double precision AS today_tonnage, \
         COALESCE(t.avg_wait, 0.0)::double precision AS avg_wait_minutes, \
         COALESCE(t.avg_load, 0.0)::double precision AS avg_loading_minutes \
         FROM pits p \
         LEFT JOIN LATERAL ( \
           SELECT \
             COUNT(*)::bigint AS today_trips, \
             COALESCE(SUM(w.actual_weight_ton), 0.0) AS today_tonnage, \
             AVG(EXTRACT(EPOCH FROM (w.load_start_time - w.queue_enter_time)) / 60.0) AS avg_wait, \
             AVG(EXTRACT(EPOCH FROM (w.load_end_time - w.load_start_time)) / 60.0) AS avg_load \
           FROM waybills w \
           WHERE w.pit_id = p.id \
             AND w.completed_time::date = $1 \
             AND w.status = 'completed' \
         ) t ON TRUE \
         WHERE p.is_active = TRUE \
         ORDER BY today_trips DESC",
    )
    .bind(today)
    .fetch_all(&state.db)
    .await
    .map_err(|err| ApiError::internal(format!("failed to query pit efficiency: {err}")))?;

    let result: Vec<PitEfficiencyRow> = rows.iter().map(|row| {
        PitEfficiencyRow {
            pit_id: row.get("pit_id"),
            pit_name: row.get("pit_name"),
            today_trips: row.get("today_trips"),
            today_tonnage: row.get("today_tonnage"),
            avg_wait_minutes: row.get("avg_wait_minutes"),
            avg_loading_minutes: row.get("avg_loading_minutes"),
        }
    }).collect();

    Ok(Json(result))
}

async fn driver_ranking(
    State(state): State<AppState>,
) -> Result<Json<Vec<DriverRankingRow>>, ApiError> {
    let today = chrono::Utc::now().date_naive();

    let rows = sqlx::query(
        "SELECT d.id AS driver_id, d.name AS driver_name, d.license_plate, \
         COALESCE(t.today_trips, 0)::bigint AS today_trips, \
         COALESCE(t.today_tonnage, 0.0)::double precision AS today_tonnage \
         FROM drivers d \
         LEFT JOIN LATERAL ( \
           SELECT \
             COUNT(*)::bigint AS today_trips, \
             COALESCE(SUM(w.actual_weight_ton), 0.0) AS today_tonnage \
           FROM waybills w \
           WHERE w.driver_id = d.id \
             AND w.completed_time::date = $1 \
             AND w.status = 'completed' \
         ) t ON TRUE \
         WHERE d.is_active = TRUE \
         ORDER BY today_trips DESC \
         LIMIT 20",
    )
    .bind(today)
    .fetch_all(&state.db)
    .await
    .map_err(|err| ApiError::internal(format!("failed to query driver ranking: {err}")))?;

    let result: Vec<DriverRankingRow> = rows.iter().map(|row| {
        DriverRankingRow {
            driver_id: row.get("driver_id"),
            driver_name: row.get("driver_name"),
            license_plate: row.get("license_plate"),
            today_trips: row.get("today_trips"),
            today_tonnage: row.get("today_tonnage"),
        }
    }).collect();

    Ok(Json(result))
}
