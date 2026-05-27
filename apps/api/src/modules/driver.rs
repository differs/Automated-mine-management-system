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
        .route("/", get(list_drivers).post(create_driver))
        .route("/{driver_id}", get(get_driver))
        .route("/import", post(import_drivers))
}

#[derive(Deserialize)]
pub struct DriverListQuery {
    pub keyword: Option<String>,
    pub status: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateDriverRequest {
    pub name: String,
    pub phone: String,
    pub license_plate: String,
    pub vehicle_type: String,
    pub capacity_ton: f64,
}

#[derive(Deserialize)]
pub struct ImportDriversRequest {
    pub source: String,
    pub total_rows: usize,
}

#[derive(Serialize)]
pub struct DriverSummary {
    pub id: Uuid,
    pub name: String,
    pub phone: String,
    pub license_plate: String,
    pub vehicle_type: String,
    pub status: String,
}

#[derive(Serialize)]
pub struct DriverDetail {
    pub id: Uuid,
    pub name: String,
    pub phone: String,
    pub license_plate: String,
    pub vehicle_type: String,
    pub capacity_ton: f64,
    pub status: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct ImportDriversResponse {
    pub accepted: bool,
    pub source: String,
    pub total_rows: usize,
}

async fn list_drivers(
    State(state): State<AppState>,
    Query(query): Query<DriverListQuery>,
) -> Json<Vec<DriverSummary>> {
    let _pool = &state.db;
    let status = query.status.unwrap_or_else(|| "idle".to_string());
    let keyword = query.keyword.unwrap_or_else(|| "sample".to_string());

    Json(vec![DriverSummary {
        id: Uuid::new_v4(),
        name: format!("{keyword} driver"),
        phone: "13800000000".to_string(),
        license_plate: "贵A12345".to_string(),
        vehicle_type: "dump_truck".to_string(),
        status,
    }])
}

async fn create_driver(
    Json(payload): Json<CreateDriverRequest>,
) -> Result<Json<DriverDetail>, ApiError> {
    if payload.name.trim().is_empty() || payload.phone.trim().is_empty() {
        return Err(ApiError::bad_request("driver name and phone are required"));
    }

    Ok(Json(DriverDetail {
        id: Uuid::new_v4(),
        name: payload.name,
        phone: payload.phone,
        license_plate: payload.license_plate,
        vehicle_type: payload.vehicle_type,
        capacity_ton: payload.capacity_ton,
        status: "idle".to_string(),
        updated_at: Utc::now(),
    }))
}

async fn get_driver(Path(driver_id): Path<Uuid>) -> Json<DriverDetail> {
    Json(DriverDetail {
        id: driver_id,
        name: "Sample Driver".to_string(),
        phone: "13800000000".to_string(),
        license_plate: "贵A12345".to_string(),
        vehicle_type: "dump_truck".to_string(),
        capacity_ton: 35.0,
        status: "idle".to_string(),
        updated_at: Utc::now(),
    })
}

async fn import_drivers(
    Json(payload): Json<ImportDriversRequest>,
) -> Result<Json<ImportDriversResponse>, ApiError> {
    if payload.total_rows == 0 {
        return Err(ApiError::bad_request("total_rows must be greater than zero"));
    }

    Ok(Json(ImportDriversResponse {
        accepted: true,
        source: payload.source,
        total_rows: payload.total_rows,
    }))
}
