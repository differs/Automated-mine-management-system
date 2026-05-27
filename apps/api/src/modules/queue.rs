use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
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
) -> Json<Vec<QueueEntry>> {
    let _pool = &state.db;

    Json(vec![
        QueueEntry {
            waybill_id: Uuid::new_v4(),
            driver_id: Uuid::new_v4(),
            queue_position: 1,
            entered_at: Utc::now(),
        },
        QueueEntry {
            waybill_id: pit_id,
            driver_id: Uuid::new_v4(),
            queue_position: 2,
            entered_at: Utc::now(),
        },
    ])
}

async fn join_queue(
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

    Ok(Json(QueueActionResponse {
        waybill_id,
        status: "queueing".to_string(),
        queue_position: Some(5),
        at: Utc::now(),
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
