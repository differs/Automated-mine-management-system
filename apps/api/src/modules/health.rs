use axum::Json;
use serde::Serialize;

#[derive(Serialize)]
pub struct HealthResponse {
    service: &'static str,
    status: &'static str,
}

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        service: "automated-mine-management-api",
        status: "ok",
    })
}
