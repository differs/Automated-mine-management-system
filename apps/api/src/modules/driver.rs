use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, QueryBuilder};
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

#[derive(FromRow)]
struct DriverSummaryRow {
    id: Uuid,
    name: String,
    phone: String,
    license_plate: String,
    vehicle_type: String,
    status: String,
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

#[derive(FromRow)]
struct DriverDetailRow {
    id: Uuid,
    name: String,
    phone: String,
    license_plate: String,
    vehicle_type: String,
    capacity_ton: f64,
    status: String,
    updated_at: DateTime<Utc>,
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
) -> Result<Json<Vec<DriverSummary>>, ApiError> {
    let mut qb = QueryBuilder::new(
        "SELECT id, name, phone, license_plate, vehicle_type::text AS vehicle_type, \
         status::text AS status FROM drivers WHERE 1=1",
    );

    if let Some(keyword) = query.keyword.as_deref() {
        let keyword = format!("%{}%", keyword.trim());
        qb.push(" AND (name ILIKE ")
            .push_bind(keyword.clone())
            .push(" OR phone ILIKE ")
            .push_bind(keyword.clone())
            .push(" OR license_plate ILIKE ")
            .push_bind(keyword)
            .push(")");
    }

    if let Some(status) = query.status.as_deref() {
        qb.push(" AND status::text = ").push_bind(status);
    }

    qb.push(" ORDER BY created_at DESC LIMIT 100");

    let rows = qb
        .build_query_as::<DriverSummaryRow>()
        .fetch_all(&state.db)
        .await
        .map_err(|err| ApiError::internal(format!("failed to list drivers: {err}")))?;

    Ok(Json(
        rows.into_iter()
            .map(|row| DriverSummary {
                id: row.id,
                name: row.name,
                phone: row.phone,
                license_plate: row.license_plate,
                vehicle_type: row.vehicle_type,
                status: row.status,
            })
            .collect(),
    ))
}

async fn create_driver(
    State(state): State<AppState>,
    Json(payload): Json<CreateDriverRequest>,
) -> Result<Json<DriverDetail>, ApiError> {
    if payload.name.trim().is_empty() || payload.phone.trim().is_empty() {
        return Err(ApiError::bad_request("driver name and phone are required"));
    }

    let row = sqlx::query_as::<_, DriverDetailRow>(
        "INSERT INTO drivers (name, phone, license_plate, vehicle_type, capacity_ton) \
         VALUES ($1, $2, $3, $4::vehicle_type, $5) \
         RETURNING id, name, phone, license_plate, vehicle_type::text AS vehicle_type, \
         capacity_ton::double precision AS capacity_ton, status::text AS status, updated_at",
    )
    .bind(payload.name.trim())
    .bind(payload.phone.trim())
    .bind(payload.license_plate.trim())
    .bind(payload.vehicle_type.trim())
    .bind(payload.capacity_ton)
    .fetch_one(&state.db)
    .await
    .map_err(map_driver_write_error)?;

    Ok(Json(DriverDetail {
        id: row.id,
        name: row.name,
        phone: row.phone,
        license_plate: row.license_plate,
        vehicle_type: row.vehicle_type,
        capacity_ton: row.capacity_ton,
        status: row.status,
        updated_at: row.updated_at,
    }))
}

async fn get_driver(
    State(state): State<AppState>,
    Path(driver_id): Path<Uuid>,
) -> Result<Json<DriverDetail>, ApiError> {
    let row = sqlx::query_as::<_, DriverDetailRow>(
        "SELECT id, name, phone, license_plate, vehicle_type::text AS vehicle_type, \
         capacity_ton::double precision AS capacity_ton, status::text AS status, updated_at \
         FROM drivers WHERE id = $1",
    )
    .bind(driver_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| ApiError::internal(format!("failed to fetch driver: {err}")))?;

    let row = row.ok_or_else(|| ApiError::not_found("driver not found"))?;

    Ok(Json(DriverDetail {
        id: row.id,
        name: row.name,
        phone: row.phone,
        license_plate: row.license_plate,
        vehicle_type: row.vehicle_type,
        capacity_ton: row.capacity_ton,
        status: row.status,
        updated_at: row.updated_at,
    }))
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

fn map_driver_write_error(err: sqlx::Error) -> ApiError {
    if let sqlx::Error::Database(db_err) = &err {
        if db_err.is_unique_violation() {
            return ApiError::conflict("driver phone or license plate already exists");
        }

        if db_err.message().contains("vehicle_type") {
            return ApiError::bad_request("invalid vehicle_type");
        }
    }

    ApiError::internal(format!("failed to create driver: {err}"))
}
