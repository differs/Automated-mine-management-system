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
    Router::new().route("/waybills/{waybill_id}", post(create_weigh_record))
}

#[derive(Deserialize)]
pub struct CreateWeighRecordRequest {
    pub operator_id: Uuid,
    pub gross_weight_ton: f64,
    pub tare_weight_ton: Option<f64>,
    pub net_weight_ton: f64,
    pub source: Option<String>,
    pub note: Option<String>,
}

#[derive(Serialize)]
pub struct WeighingActionResponse {
    pub waybill_id: Uuid,
    pub status: String,
    pub net_weight_ton: f64,
    pub completed_at: DateTime<Utc>,
}

async fn create_weigh_record(
    State(state): State<AppState>,
    Path(waybill_id): Path<Uuid>,
    Json(payload): Json<CreateWeighRecordRequest>,
) -> Result<Json<WeighingActionResponse>, ApiError> {
    if payload.operator_id.is_nil() {
        return Err(ApiError::bad_request("operator_id is required"));
    }

    if payload.gross_weight_ton < 0.0 || payload.net_weight_ton < 0.0 {
        return Err(ApiError::bad_request("weight values must be non-negative"));
    }

    if let Some(tare_weight_ton) = payload.tare_weight_ton
        && tare_weight_ton < 0.0
    {
        return Err(ApiError::bad_request("tare_weight_ton must be non-negative"));
    }

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|err| ApiError::internal(format!("failed to begin weighing transaction: {err}")))?;

    let status_row = sqlx::query("SELECT status::text AS status FROM waybills WHERE id = $1 FOR UPDATE")
        .bind(waybill_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|err| ApiError::internal(format!("failed to lock waybill for weighing: {err}")))?;

    let Some(status_row) = status_row else {
        return Err(ApiError::not_found("waybill not found"));
    };

    let status: String = status_row.get("status");
    if status != "loaded" {
        return Err(ApiError::conflict("only loaded waybills can create weigh records"));
    }

    let now = Utc::now();
    let source = payload
        .source
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("manual");

    sqlx::query(
        "INSERT INTO weigh_records (waybill_id, gross_weight_ton, tare_weight_ton, net_weight_ton, \
         weigh_time, operator_id, source, note) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(waybill_id)
    .bind(payload.gross_weight_ton)
    .bind(payload.tare_weight_ton)
    .bind(payload.net_weight_ton)
    .bind(now)
    .bind(payload.operator_id)
    .bind(source)
    .bind(payload.note.as_deref().map(str::trim))
    .execute(&mut *tx)
    .await
    .map_err(|err| {
        if let sqlx::Error::Database(db_err) = &err
            && db_err.is_unique_violation()
        {
            return ApiError::conflict("weigh record already exists for this waybill");
        }
        ApiError::internal(format!("failed to create weigh record: {err}"))
    })?;

    let updated = sqlx::query(
        "UPDATE waybills SET status = 'completed', weigh_start_time = $2, actual_weight_ton = $3, \
         completed_time = $2, updated_at = $2, version = version + 1 \
         WHERE id = $1 RETURNING id, status::text AS status, actual_weight_ton::double precision AS actual_weight_ton, completed_time",
    )
    .bind(waybill_id)
    .bind(now)
    .bind(payload.net_weight_ton)
    .fetch_one(&mut *tx)
    .await
    .map_err(|err| ApiError::internal(format!("failed to complete waybill after weighing: {err}")))?;

    tx.commit()
        .await
        .map_err(|err| ApiError::internal(format!("failed to commit weighing: {err}")))?;

    Ok(Json(WeighingActionResponse {
        waybill_id: updated.get("id"),
        status: updated.get("status"),
        net_weight_ton: updated.get("actual_weight_ton"),
        completed_at: updated.get("completed_time"),
    }))
}
