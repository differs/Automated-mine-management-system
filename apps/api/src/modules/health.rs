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

/// OpenAPI 文档 JSON
pub async fn openapi_doc() -> axum::response::Response {
    let doc = include_str!("../../../../docs/openapi.json");
    axum::response::Response::builder()
        .header("content-type", "application/json; charset=utf-8")
        .body(axum::body::Body::from(doc))
        .unwrap()
}
