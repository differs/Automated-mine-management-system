use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
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

#[derive(FromRow)]
struct PitSummaryRow {
    id: Uuid,
    name: String,
    code: Option<String>,
    current_queue_count: i32,
    avg_wait_minutes: i32,
    is_active: bool,
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

#[derive(FromRow)]
struct PitDetailRow {
    id: Uuid,
    name: String,
    code: Option<String>,
    location_text: Option<String>,
    queue_capacity: Option<i32>,
    current_queue_count: i32,
    avg_wait_minutes: i32,
    is_active: bool,
}

async fn list_pits(State(state): State<AppState>) -> Result<Json<Vec<PitSummary>>, ApiError> {
    let rows = sqlx::query_as::<_, PitSummaryRow>(
        "SELECT id, name, code, current_queue_count, avg_wait_minutes, is_active \
         FROM pits ORDER BY created_at DESC LIMIT 100",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|err| ApiError::internal(format!("failed to list pits: {err}")))?;

    Ok(Json(
        rows.into_iter()
            .map(|row| PitSummary {
                id: row.id,
                name: row.name,
                code: row.code.unwrap_or_default(),
                current_queue_count: row.current_queue_count,
                avg_wait_minutes: row.avg_wait_minutes,
                is_active: row.is_active,
            })
            .collect(),
    ))
}

async fn create_pit(
    State(state): State<AppState>,
    Json(payload): Json<CreatePitRequest>,
) -> Result<Json<PitDetail>, ApiError> {
    if payload.name.trim().is_empty() {
        return Err(ApiError::bad_request("pit name is required"));
    }

    let row = sqlx::query_as::<_, PitDetailRow>(
        "INSERT INTO pits (name, code, location_text, queue_capacity) \
         VALUES ($1, $2, $3, $4) \
         RETURNING id, name, code, location_text, queue_capacity, \
         current_queue_count, avg_wait_minutes, is_active",
    )
    .bind(payload.name.trim())
    .bind(payload.code.as_deref().map(str::trim))
    .bind(payload.location_text.as_deref().map(str::trim))
    .bind(payload.queue_capacity)
    .fetch_one(&state.db)
    .await
    .map_err(|err| {
        if let sqlx::Error::Database(db_err) = &err
            && db_err.is_unique_violation()
        {
            return ApiError::conflict("pit name or code already exists");
        }
        ApiError::internal(format!("failed to create pit: {err}"))
    })?;

    Ok(Json(PitDetail {
        id: row.id,
        name: row.name,
        code: row.code.unwrap_or_default(),
        location_text: row.location_text,
        queue_capacity: row.queue_capacity,
        current_queue_count: row.current_queue_count,
        avg_wait_minutes: row.avg_wait_minutes,
        is_active: row.is_active,
    }))
}

async fn get_pit(
    State(state): State<AppState>,
    Path(pit_id): Path<Uuid>,
) -> Result<Json<PitDetail>, ApiError> {
    let row = sqlx::query_as::<_, PitDetailRow>(
        "SELECT id, name, code, location_text, queue_capacity, \
         current_queue_count, avg_wait_minutes, is_active \
         FROM pits WHERE id = $1",
    )
    .bind(pit_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| ApiError::internal(format!("failed to fetch pit: {err}")))?;

    let row = row.ok_or_else(|| ApiError::not_found("pit not found"))?;

    Ok(Json(PitDetail {
        id: row.id,
        name: row.name,
        code: row.code.unwrap_or_default(),
        location_text: row.location_text,
        queue_capacity: row.queue_capacity,
        current_queue_count: row.current_queue_count,
        avg_wait_minutes: row.avg_wait_minutes,
        is_active: row.is_active,
    }))
}
