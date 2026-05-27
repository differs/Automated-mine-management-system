use axum::{
    Json, Router,
    extract::{Path, State},
    routing::post,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/waybills/{waybill_id}/start", post(start_loading))
        .route("/waybills/{waybill_id}/finish", post(finish_loading))
}

#[derive(Deserialize)]
pub struct StartLoadingRequest {
    pub operator_id: Uuid,
    pub loader_name: Option<String>,
    pub notes: Option<String>,
}

#[derive(Deserialize)]
pub struct FinishLoadingRequest {
    pub operator_id: Uuid,
    pub notes: Option<String>,
}

#[derive(Serialize)]
pub struct LoadingActionResponse {
    pub waybill_id: Uuid,
    pub status: String,
    pub at: DateTime<Utc>,
}

async fn start_loading(
    State(state): State<AppState>,
    Path(waybill_id): Path<Uuid>,
    Json(payload): Json<StartLoadingRequest>,
) -> Result<Json<LoadingActionResponse>, ApiError> {
    if payload.operator_id.is_nil() {
        return Err(ApiError::bad_request("operator_id is required"));
    }

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|err| ApiError::internal(format!("failed to begin loading transaction: {err}")))?;

    let status_row = sqlx::query("SELECT status::text AS status, pit_id FROM waybills WHERE id = $1 FOR UPDATE")
        .bind(waybill_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|err| ApiError::internal(format!("failed to lock waybill for loading start: {err}")))?;

    let Some(status_row) = status_row else {
        return Err(ApiError::not_found("waybill not found"));
    };

    let status: String = status_row.get("status");
    let pit_id: Uuid = status_row.get("pit_id");

    if status != "arrived" {
        return Err(ApiError::conflict("only arrived waybills can start loading"));
    }

    let now = Utc::now();

    sqlx::query(
        "INSERT INTO loading_records (waybill_id, pit_id, operator_id, start_time, loader_name, notes) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(waybill_id)
    .bind(pit_id)
    .bind(payload.operator_id)
    .bind(now)
    .bind(payload.loader_name.as_deref().map(str::trim))
    .bind(payload.notes.as_deref().map(str::trim))
    .execute(&mut *tx)
    .await
    .map_err(|err| {
        if let sqlx::Error::Database(db_err) = &err
            && db_err.is_unique_violation()
        {
            return ApiError::conflict("loading record already exists for this waybill");
        }
        ApiError::internal(format!("failed to create loading record: {err}"))
    })?;

    let updated = sqlx::query(
        "UPDATE waybills SET status = 'loading', load_start_time = $2, updated_at = $2, \
         version = version + 1 WHERE id = $1 \
         RETURNING id, status::text AS status, load_start_time",
    )
    .bind(waybill_id)
    .bind(now)
    .fetch_one(&mut *tx)
    .await
    .map_err(|err| ApiError::internal(format!("failed to update waybill loading status: {err}")))?;

    tx.commit()
        .await
        .map_err(|err| ApiError::internal(format!("failed to commit loading start: {err}")))?;

    Ok(Json(LoadingActionResponse {
        waybill_id: updated.get("id"),
        status: updated.get("status"),
        at: updated.get("load_start_time"),
    }))
}

async fn finish_loading(
    State(state): State<AppState>,
    Path(waybill_id): Path<Uuid>,
    Json(payload): Json<FinishLoadingRequest>,
) -> Result<Json<LoadingActionResponse>, ApiError> {
    if payload.operator_id.is_nil() {
        return Err(ApiError::bad_request("operator_id is required"));
    }

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|err| ApiError::internal(format!("failed to begin loading finish transaction: {err}")))?;

    let status_row = sqlx::query("SELECT status::text AS status FROM waybills WHERE id = $1 FOR UPDATE")
        .bind(waybill_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|err| ApiError::internal(format!("failed to lock waybill for loading finish: {err}")))?;

    let Some(status_row) = status_row else {
        return Err(ApiError::not_found("waybill not found"));
    };

    let status: String = status_row.get("status");
    if status != "loading" {
        return Err(ApiError::conflict("only loading waybills can finish loading"));
    }

    let now = Utc::now();

    let updated_loading = sqlx::query(
        "UPDATE loading_records SET end_time = $2, operator_id = $3, \
         notes = COALESCE($4, notes), updated_at = $2 \
         WHERE waybill_id = $1 AND end_time IS NULL \
         RETURNING waybill_id",
    )
    .bind(waybill_id)
    .bind(now)
    .bind(payload.operator_id)
    .bind(payload.notes.as_deref().map(str::trim))
    .fetch_optional(&mut *tx)
    .await
    .map_err(|err| ApiError::internal(format!("failed to update loading record: {err}")))?;

    if updated_loading.is_none() {
        return Err(ApiError::conflict("active loading record not found for this waybill"));
    }

    let updated = sqlx::query(
        "UPDATE waybills SET status = 'loaded', load_end_time = $2, updated_at = $2, \
         version = version + 1 WHERE id = $1 \
         RETURNING id, status::text AS status, load_end_time",
    )
    .bind(waybill_id)
    .bind(now)
    .fetch_one(&mut *tx)
    .await
    .map_err(|err| ApiError::internal(format!("failed to update waybill loaded status: {err}")))?;

    tx.commit()
        .await
        .map_err(|err| ApiError::internal(format!("failed to commit loading finish: {err}")))?;

    Ok(Json(LoadingActionResponse {
        waybill_id: updated.get("id"),
        status: updated.get("status"),
        at: updated.get("load_end_time"),
    }))
}
