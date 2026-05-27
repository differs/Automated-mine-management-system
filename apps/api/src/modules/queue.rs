use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row};
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/pits/{pit_id}", get(get_pit_queue))
        .route("/waybills/{waybill_id}/join", post(join_queue))
        .route("/waybills/{waybill_id}/call-next", post(call_next))
        .route("/waybills/{waybill_id}/leave", post(leave_queue))
}

#[derive(Deserialize)]
pub struct JoinQueueRequest {
    pub driver_id: Uuid,
    pub pit_id: Uuid,
    pub arrival_method: String,
}

#[derive(Deserialize)]
pub struct QueueActionRequest {
    pub operator_id: Uuid,
    pub reason: Option<String>,
}

#[derive(Serialize)]
pub struct QueueEntry {
    pub waybill_id: Uuid,
    pub driver_id: Uuid,
    pub queue_position: i32,
    pub entered_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct QueueEntryRow {
    waybill_id: Uuid,
    driver_id: Uuid,
    queue_position: i32,
    entered_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct QueueActionResponse {
    pub waybill_id: Uuid,
    pub status: String,
    pub queue_position: Option<i32>,
    pub at: DateTime<Utc>,
}

async fn get_pit_queue(
    State(state): State<AppState>,
    Path(pit_id): Path<Uuid>,
) -> Result<Json<Vec<QueueEntry>>, ApiError> {
    let rows = sqlx::query_as::<_, QueueEntryRow>(
        "SELECT waybill_id, driver_id, queue_position, enter_queue_time AS entered_at \
         FROM queue_logs WHERE pit_id = $1 AND exit_queue_time IS NULL \
         ORDER BY queue_position ASC, enter_queue_time ASC",
    )
    .bind(pit_id)
    .fetch_all(&state.db)
    .await
    .map_err(|err| ApiError::internal(format!("failed to load pit queue: {err}")))?;

    Ok(Json(
        rows.into_iter()
            .map(|row| QueueEntry {
                waybill_id: row.waybill_id,
                driver_id: row.driver_id,
                queue_position: row.queue_position,
                entered_at: row.entered_at,
            })
            .collect(),
    ))
}

async fn join_queue(
    State(state): State<AppState>,
    Path(waybill_id): Path<Uuid>,
    Json(payload): Json<JoinQueueRequest>,
) -> Result<Json<QueueActionResponse>, ApiError> {
    if payload.driver_id.is_nil()
        || payload.pit_id.is_nil()
        || payload.arrival_method.trim().is_empty()
    {
        return Err(ApiError::bad_request(
            "driver_id, pit_id and arrival_method are required",
        ));
    }

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|err| ApiError::internal(format!("failed to begin queue transaction: {err}")))?;

    let waybill_row = sqlx::query(
        "SELECT driver_id, pit_id, status::text AS status FROM waybills WHERE id = $1 FOR UPDATE",
    )
    .bind(waybill_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|err| ApiError::internal(format!("failed to lock waybill: {err}")))?;

    let Some(waybill_row) = waybill_row else {
        return Err(ApiError::not_found("waybill not found"));
    };

    let current_driver_id: Uuid = waybill_row.get("driver_id");
    let current_pit_id: Uuid = waybill_row.get("pit_id");
    let current_status: String = waybill_row.get("status");

    if current_driver_id != payload.driver_id || current_pit_id != payload.pit_id {
        return Err(ApiError::conflict("waybill driver_id or pit_id does not match join request"));
    }

    if current_status == "queueing" {
        return Err(ApiError::conflict("waybill is already in queue"));
    }

    if current_status != "arrived" {
        return Err(ApiError::conflict("only arrived waybills can join queue"));
    }

    let pit_active = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM pits WHERE id = $1 AND is_active = TRUE)",
    )
    .bind(payload.pit_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|err| ApiError::internal(format!("failed to validate pit status: {err}")))?;

    if !pit_active {
        return Err(ApiError::bad_request("pit not found or inactive"));
    }

    let queue_position: i32 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(queue_position), 0) + 1 FROM queue_logs \
         WHERE pit_id = $1 AND exit_queue_time IS NULL",
    )
    .bind(payload.pit_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|err| ApiError::internal(format!("failed to calculate queue position: {err}")))?;

    let now = Utc::now();

    sqlx::query(
        "INSERT INTO queue_logs (pit_id, driver_id, waybill_id, enter_queue_time, queue_position) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(payload.pit_id)
    .bind(payload.driver_id)
    .bind(waybill_id)
    .bind(now)
    .bind(queue_position)
    .execute(&mut *tx)
    .await
    .map_err(|err| ApiError::internal(format!("failed to create queue log: {err}")))?;

    sqlx::query(
        "UPDATE waybills SET status = 'queueing', queue_enter_time = $2, queue_number = $3, \
         updated_at = $2, version = version + 1 WHERE id = $1",
    )
    .bind(waybill_id)
    .bind(now)
    .bind(queue_position)
    .execute(&mut *tx)
    .await
    .map_err(|err| ApiError::internal(format!("failed to update waybill queue status: {err}")))?;

    sqlx::query(
        "UPDATE pits SET current_queue_count = $2, updated_at = $3 WHERE id = $1",
    )
    .bind(payload.pit_id)
    .bind(queue_position)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|err| ApiError::internal(format!("failed to refresh pit queue count: {err}")))?;

    tx.commit()
        .await
        .map_err(|err| ApiError::internal(format!("failed to commit queue join: {err}")))?;

    Ok(Json(QueueActionResponse {
        waybill_id,
        status: "queueing".to_string(),
        queue_position: Some(queue_position),
        at: now,
    }))
}

async fn call_next(
    Path(waybill_id): Path<Uuid>,
    Json(payload): Json<QueueActionRequest>,
) -> Result<Json<QueueActionResponse>, ApiError> {
    if payload.operator_id.is_nil() {
        return Err(ApiError::bad_request("operator_id is required"));
    }

    Ok(Json(QueueActionResponse {
        waybill_id,
        status: "loading".to_string(),
        queue_position: Some(1),
        at: Utc::now(),
    }))
}

async fn leave_queue(
    Path(waybill_id): Path<Uuid>,
    Json(payload): Json<QueueActionRequest>,
) -> Result<Json<QueueActionResponse>, ApiError> {
    if payload.operator_id.is_nil() && payload.reason.as_deref().unwrap_or("").trim().is_empty() {
        return Err(ApiError::bad_request(
            "operator_id or manual leave reason is required",
        ));
    }

    Ok(Json(QueueActionResponse {
        waybill_id,
        status: "left_queue".to_string(),
        queue_position: None,
        at: Utc::now(),
    }))
}
