use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_pits).post(create_pit))
        .route("/{pit_id}", get(get_pit))
}

#[derive(Deserialize)]
pub struct CreatePitRequest {
    pub name: String,
    pub code: Option<String>,
    pub location_text: Option<String>,
    pub queue_capacity: Option<i32>,
}

#[derive(Serialize)]
pub struct PitSummary {
    pub id: Uuid,
    pub name: String,
    pub code: String,
    pub current_queue_count: i32,
    pub avg_wait_minutes: i32,
    pub is_active: bool,
}

#[derive(Serialize)]
pub struct PitDetail {
    pub id: Uuid,
    pub name: String,
    pub code: String,
    pub location_text: Option<String>,
    pub queue_capacity: Option<i32>,
    pub current_queue_count: i32,
    pub avg_wait_minutes: i32,
    pub is_active: bool,
}

async fn list_pits(State(state): State<AppState>) -> Json<Vec<PitSummary>> {
    let _pool = &state.db;

    Json(vec![PitSummary {
        id: Uuid::new_v4(),
        name: "1号坑".to_string(),
        code: "PIT-001".to_string(),
        current_queue_count: 4,
        avg_wait_minutes: 18,
        is_active: true,
    }])
}

async fn create_pit(Json(payload): Json<CreatePitRequest>) -> Result<Json<PitDetail>, ApiError> {
    if payload.name.trim().is_empty() {
        return Err(ApiError::bad_request("pit name is required"));
    }

    Ok(Json(PitDetail {
        id: Uuid::new_v4(),
        name: payload.name,
        code: payload.code.unwrap_or_else(|| "PIT-AUTO".to_string()),
        location_text: payload.location_text,
        queue_capacity: payload.queue_capacity,
        current_queue_count: 0,
        avg_wait_minutes: 0,
        is_active: true,
    }))
}

async fn get_pit(Path(pit_id): Path<Uuid>) -> Json<PitDetail> {
    Json(PitDetail {
        id: pit_id,
        name: "1号坑".to_string(),
        code: "PIT-001".to_string(),
        location_text: Some("贵州矿区东侧".to_string()),
        queue_capacity: Some(20),
        current_queue_count: 4,
        avg_wait_minutes: 18,
        is_active: true,
    })
}
