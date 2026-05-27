use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
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

#[derive(Serialize)]
pub struct WaybillActionResponse {
    pub id: Uuid,
    pub status: String,
    pub at: DateTime<Utc>,
}

async fn list_waybills(
    State(state): State<AppState>,
    Query(query): Query<WaybillListQuery>,
) -> Json<Vec<WaybillSummary>> {
    let _pool = &state.db;
    let status = query.status.unwrap_or_else(|| "pending_dispatch".to_string());

    Json(vec![WaybillSummary {
        id: query.pit_id.unwrap_or_else(Uuid::new_v4),
        serial_no: "WB-20260528-0001".to_string(),
        driver_id: Uuid::new_v4(),
        pit_id: Uuid::new_v4(),
        status,
        dispatch_time: Some(Utc::now()),
    }])
}

async fn create_waybill(
    Json(payload): Json<CreateWaybillRequest>,
) -> Json<WaybillDetail> {
    Json(WaybillDetail {
        id: Uuid::new_v4(),
        serial_no: format!("WB-{}", Utc::now().format("%Y%m%d%H%M%S")),
        driver_id: payload.driver_id,
        pit_id: payload.pit_id,
        status: "pending_dispatch".to_string(),
        queue_number: None,
        estimated_weight_ton: payload.estimated_weight_ton,
        actual_weight_ton: None,
        dispatch_time: None,
        arrive_time: None,
    })
}

async fn get_waybill(Path(waybill_id): Path<Uuid>) -> Json<WaybillDetail> {
    Json(WaybillDetail {
        id: waybill_id,
        serial_no: "WB-20260528-0001".to_string(),
        driver_id: Uuid::new_v4(),
        pit_id: Uuid::new_v4(),
        status: "dispatched".to_string(),
        queue_number: Some(3),
        estimated_weight_ton: Some(32.5),
        actual_weight_ton: None,
        dispatch_time: Some(Utc::now()),
        arrive_time: None,
    })
}

async fn dispatch_waybill(
    Path(waybill_id): Path<Uuid>,
    Json(payload): Json<DispatchWaybillRequest>,
) -> Result<Json<WaybillActionResponse>, ApiError> {
    if payload.dispatcher_id.is_nil() {
        return Err(ApiError::bad_request("dispatcher_id is required"));
    }

    Ok(Json(WaybillActionResponse {
        id: waybill_id,
        status: "dispatched".to_string(),
        at: Utc::now(),
    }))
}

async fn arrive_waybill(
    Path(waybill_id): Path<Uuid>,
    Json(payload): Json<ArriveWaybillRequest>,
) -> Result<Json<WaybillActionResponse>, ApiError> {
    if payload.arrival_source.trim().is_empty() {
        return Err(ApiError::bad_request("arrival_source is required"));
    }

    Ok(Json(WaybillActionResponse {
        id: waybill_id,
        status: "arrived".to_string(),
        at: Utc::now(),
    }))
}

async fn cancel_waybill(
    Path(waybill_id): Path<Uuid>,
    Json(payload): Json<CancelWaybillRequest>,
) -> Result<Json<WaybillActionResponse>, ApiError> {
    if payload.cancelled_by.is_nil() || payload.reason.trim().is_empty() {
        return Err(ApiError::bad_request("cancelled_by and reason are required"));
    }

    Ok(Json(WaybillActionResponse {
        id: waybill_id,
        status: "cancelled".to_string(),
        at: Utc::now(),
    }))
}
