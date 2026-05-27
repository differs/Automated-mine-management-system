use axum::{Router, routing::get};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{
    modules::{auth, driver, pit, queue, waybill},
    state::AppState,
};

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(crate::modules::health::health))
        .nest("/api/v1/auth", auth::router())
        .nest("/api/v1/drivers", driver::router())
        .nest("/api/v1/pits", pit::router())
        .nest("/api/v1/waybills", waybill::router())
        .nest("/api/v1/queue", queue::router())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
