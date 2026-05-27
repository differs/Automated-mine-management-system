use axum::{Json, Router, routing::get};
use serde::Serialize;
use std::net::SocketAddr;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;

#[derive(Serialize)]
struct HealthResponse {
    service: &'static str,
    status: &'static str,
}

#[derive(Serialize)]
struct OverviewResponse {
    product: &'static str,
    phase: &'static str,
    dispatch_flow: [&'static str; 6],
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "api=debug,tower_http=info".to_string()),
        )
        .init();

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/v1/overview", get(overview))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("api listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        service: "automated-mine-management-api",
        status: "ok",
    })
}

async fn overview() -> Json<OverviewResponse> {
    Json(OverviewResponse {
        product: "Automated Mine Management System",
        phase: "mvp-foundation",
        dispatch_flow: [
            "dispatch",
            "arrival",
            "queue",
            "loading",
            "weighing",
            "completion",
        ],
    })
}
