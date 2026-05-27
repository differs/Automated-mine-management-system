use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, QueryBuilder, Row};
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_waybills).post(create_waybill))
        .route("/{waybill_id}", get(get_waybill))
        .route("/{waybill_id}/dispatch", post(dispatch_waybill))
        .route("/{waybill_id}/arrive", post(arrive_waybill))
        .route("/{waybill_id}/cancel", post(cancel_waybill))
}

#[derive(Deserialize)]
pub struct WaybillListQuery {
    pub status: Option<String>,
    pub pit_id: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct CreateWaybillRequest {
    pub driver_id: Uuid,
    pub pit_id: Uuid,
    pub estimated_weight_ton: Option<f64>,
}

#[derive(Deserialize)]
pub struct DispatchWaybillRequest {
    pub dispatcher_id: Uuid,
}

#[derive(Deserialize)]
pub struct ArriveWaybillRequest {
    pub arrival_source: String,
}

#[derive(Deserialize)]
pub struct CancelWaybillRequest {
    pub cancelled_by: Uuid,
    pub reason: String,
}

#[derive(Serialize)]
pub struct WaybillSummary {
    pub id: Uuid,
    pub serial_no: String,
    pub driver_id: Uuid,
    pub pit_id: Uuid,
    pub status: String,
    pub dispatch_time: Option<DateTime<Utc>>,
}

#[derive(FromRow)]
struct WaybillSummaryRow {
    id: Uuid,
    serial_no: String,
    driver_id: Uuid,
    pit_id: Uuid,
    status: String,
    dispatch_time: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
pub struct WaybillDetail {
    pub id: Uuid,
    pub serial_no: String,
    pub driver_id: Uuid,
    pub pit_id: Uuid,
    pub status: String,
    pub queue_number: Option<i32>,
    pub estimated_weight_ton: Option<f64>,
    pub actual_weight_ton: Option<f64>,
    pub dispatch_time: Option<DateTime<Utc>>,
    pub arrive_time: Option<DateTime<Utc>>,
}

#[derive(FromRow)]
struct WaybillDetailRow {
    id: Uuid,
    serial_no: String,
    driver_id: Uuid,
    pit_id: Uuid,
    status: String,
    queue_number: Option<i32>,
    estimated_weight_ton: Option<f64>,
    actual_weight_ton: Option<f64>,
    dispatch_time: Option<DateTime<Utc>>,
    arrive_time: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
pub struct WaybillActionResponse {
    pub id: Uuid,
    pub status: String,
    pub at: DateTime<Utc>,
}

async fn list_waybills(
    State(state): State<AppState>,
    Query(query): Query<WaybillListQuery>,
) -> Result<Json<Vec<WaybillSummary>>, ApiError> {
    let mut qb = QueryBuilder::new(
        "SELECT id, serial_no, driver_id, pit_id, status::text AS status, dispatch_time \
         FROM waybills WHERE 1=1",
    );

    if let Some(status) = query.status.as_deref() {
        qb.push(" AND status::text = ").push_bind(status);
    }

    if let Some(pit_id) = query.pit_id {
        qb.push(" AND pit_id = ").push_bind(pit_id);
    }

    qb.push(" ORDER BY created_at DESC LIMIT 100");

    let rows = qb
        .build_query_as::<WaybillSummaryRow>()
        .fetch_all(&state.db)
        .await
        .map_err(|err| ApiError::internal(format!("failed to list waybills: {err}")))?;

    Ok(Json(
        rows.into_iter()
            .map(|row| WaybillSummary {
                id: row.id,
                serial_no: row.serial_no,
                driver_id: row.driver_id,
                pit_id: row.pit_id,
                status: row.status,
                dispatch_time: row.dispatch_time,
            })
            .collect(),
    ))
}

async fn create_waybill(
    State(state): State<AppState>,
    Json(payload): Json<CreateWaybillRequest>,
) -> Result<Json<WaybillDetail>, ApiError> {
    ensure_driver_and_pit_exist(&state, payload.driver_id, payload.pit_id).await?;

    let serial_no = build_waybill_serial_no();

    let row = sqlx::query_as::<_, WaybillDetailRow>(
        "INSERT INTO waybills (serial_no, driver_id, pit_id, estimated_weight_ton) \
         VALUES ($1, $2, $3, $4) \
         RETURNING id, serial_no, driver_id, pit_id, status::text AS status, queue_number, \
         estimated_weight_ton::double precision AS estimated_weight_ton, \
         actual_weight_ton::double precision AS actual_weight_ton, dispatch_time, arrive_time",
    )
    .bind(&serial_no)
    .bind(payload.driver_id)
    .bind(payload.pit_id)
    .bind(payload.estimated_weight_ton)
    .fetch_one(&state.db)
    .await
    .map_err(map_waybill_write_error)?;

    Ok(Json(map_waybill_detail(row)))
}

async fn get_waybill(
    State(state): State<AppState>,
    Path(waybill_id): Path<Uuid>,
) -> Result<Json<WaybillDetail>, ApiError> {
    let row = sqlx::query_as::<_, WaybillDetailRow>(
        "SELECT id, serial_no, driver_id, pit_id, status::text AS status, queue_number, \
         estimated_weight_ton::double precision AS estimated_weight_ton, \
         actual_weight_ton::double precision AS actual_weight_ton, dispatch_time, arrive_time \
         FROM waybills WHERE id = $1",
    )
    .bind(waybill_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| ApiError::internal(format!("failed to fetch waybill: {err}")))?;

    let row = row.ok_or_else(|| ApiError::not_found("waybill not found"))?;

    Ok(Json(map_waybill_detail(row)))
}

async fn dispatch_waybill(
    State(state): State<AppState>,
    Path(waybill_id): Path<Uuid>,
    Json(payload): Json<DispatchWaybillRequest>,
) -> Result<Json<WaybillActionResponse>, ApiError> {
    if payload.dispatcher_id.is_nil() {
        return Err(ApiError::bad_request("dispatcher_id is required"));
    }

    let now = Utc::now();
    let row = sqlx::query(
        "UPDATE waybills SET status = 'dispatched', dispatch_time = $2, updated_at = $2, \
         version = version + 1 \
         WHERE id = $1 AND status = 'pending_dispatch' \
         RETURNING id, status::text AS status, dispatch_time",
    )
    .bind(waybill_id)
    .bind(now)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| ApiError::internal(format!("failed to dispatch waybill: {err}")))?;

    let row = row.ok_or_else(|| {
        ApiError::conflict("waybill can only be dispatched from pending_dispatch status")
    })?;

    Ok(Json(WaybillActionResponse {
        id: row.get("id"),
        status: row.get("status"),
        at: row.get("dispatch_time"),
    }))
}

async fn arrive_waybill(
    State(state): State<AppState>,
    Path(waybill_id): Path<Uuid>,
    Json(payload): Json<ArriveWaybillRequest>,
) -> Result<Json<WaybillActionResponse>, ApiError> {
    if payload.arrival_source.trim().is_empty() {
        return Err(ApiError::bad_request("arrival_source is required"));
    }

    let now = Utc::now();
    let row = sqlx::query(
        "UPDATE waybills SET status = 'arrived', arrive_time = $2, updated_at = $2, \
         version = version + 1 \
         WHERE id = $1 AND status = 'dispatched' \
         RETURNING id, status::text AS status, arrive_time",
    )
    .bind(waybill_id)
    .bind(now)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| ApiError::internal(format!("failed to mark waybill arrived: {err}")))?;

    let row = row.ok_or_else(|| {
        ApiError::conflict("waybill can only be marked arrived from dispatched status")
    })?;

    Ok(Json(WaybillActionResponse {
        id: row.get("id"),
        status: row.get("status"),
        at: row.get("arrive_time"),
    }))
}

async fn cancel_waybill(
    State(state): State<AppState>,
    Path(waybill_id): Path<Uuid>,
    Json(payload): Json<CancelWaybillRequest>,
) -> Result<Json<WaybillActionResponse>, ApiError> {
    if payload.cancelled_by.is_nil() || payload.reason.trim().is_empty() {
        return Err(ApiError::bad_request("cancelled_by and reason are required"));
    }

    let now = Utc::now();
    let row = sqlx::query(
        "UPDATE waybills SET status = 'cancelled', cancelled_by = $2, cancelled_reason = $3, \
         cancelled_time = $4, updated_at = $4, version = version + 1 \
         WHERE id = $1 AND status <> 'completed' AND status <> 'cancelled' \
         RETURNING id, status::text AS status, cancelled_time",
    )
    .bind(waybill_id)
    .bind(payload.cancelled_by)
    .bind(payload.reason.trim())
    .bind(now)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| ApiError::internal(format!("failed to cancel waybill: {err}")))?;

    let row = row.ok_or_else(|| {
        ApiError::conflict("completed or cancelled waybill cannot be cancelled again")
    })?;

    Ok(Json(WaybillActionResponse {
        id: row.get("id"),
        status: row.get("status"),
        at: row.get("cancelled_time"),
    }))
}

async fn ensure_driver_and_pit_exist(
    state: &AppState,
    driver_id: Uuid,
    pit_id: Uuid,
) -> Result<(), ApiError> {
    let driver_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM drivers WHERE id = $1 AND is_active = TRUE)",
    )
    .bind(driver_id)
    .fetch_one(&state.db)
    .await
    .map_err(|err| ApiError::internal(format!("failed to validate driver: {err}")))?;

    if !driver_exists {
        return Err(ApiError::bad_request("driver not found or inactive"));
    }

    let pit_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM pits WHERE id = $1 AND is_active = TRUE)",
    )
    .bind(pit_id)
    .fetch_one(&state.db)
    .await
    .map_err(|err| ApiError::internal(format!("failed to validate pit: {err}")))?;

    if !pit_exists {
        return Err(ApiError::bad_request("pit not found or inactive"));
    }

    Ok(())
}

fn build_waybill_serial_no() -> String {
    let suffix = Uuid::new_v4()
        .simple()
        .to_string()
        .chars()
        .take(6)
        .collect::<String>()
        .to_uppercase();
    format!("WB-{}-{suffix}", Utc::now().format("%Y%m%d%H%M%S"))
}

fn map_waybill_detail(row: WaybillDetailRow) -> WaybillDetail {
    WaybillDetail {
        id: row.id,
        serial_no: row.serial_no,
        driver_id: row.driver_id,
        pit_id: row.pit_id,
        status: row.status,
        queue_number: row.queue_number,
        estimated_weight_ton: row.estimated_weight_ton,
        actual_weight_ton: row.actual_weight_ton,
        dispatch_time: row.dispatch_time,
        arrive_time: row.arrive_time,
    }
}

fn map_waybill_write_error(err: sqlx::Error) -> ApiError {
    if let sqlx::Error::Database(db_err) = &err {
        if db_err.is_unique_violation() {
            return ApiError::conflict("driver already has an active waybill or serial number exists");
        }

        if db_err.message().contains("drivers") || db_err.message().contains("driver_id") {
            return ApiError::bad_request("invalid driver_id");
        }

        if db_err.message().contains("pits") || db_err.message().contains("pit_id") {
            return ApiError::bad_request("invalid pit_id");
        }
    }

    ApiError::internal(format!("failed to create waybill: {err}"))
}
