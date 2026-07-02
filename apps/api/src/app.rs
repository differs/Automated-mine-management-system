use axum::{Router, middleware, routing::get};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{
    middleware::{auth::require_auth, rate_limit::create_rate_limiter},
    modules::{
        alert, auth, dashboard, dispatch, driver, fence, health, loading, missions, offline, pit,
        queue, scale, system_config, waybill, weighing, ws,
    },
    state::AppState,
};

pub fn build_router(state: AppState) -> Router {
    // ── 受保护路由（需要 JWT 认证）────────────────────────────────
    let protected_routes = Router::new()
        .nest("/drivers", driver::router())
        .nest("/pits", pit::router())
        .nest("/waybills", waybill::router())
        .nest("/queue", queue::router())
        .nest("/loading", loading::router())
        .nest("/weighing", weighing::router())
        .nest("/dashboard", dashboard::router())
        .nest("/fence", fence::router())
        .nest("/system", system_config::router())
        .nest("/alerts", alert::router())
        .nest("/missions", missions::router())
        .nest("/offline", offline::router())
        .nest("/scale", scale::router())
        .nest("/dispatch", dispatch::router())
        .nest("/ws", ws::router())
        .layer(middleware::from_fn(require_auth));

    // ── 注入 jwt_secret 到 request extensions ─────────────────────
    let jwt_secret = state.config.jwt_secret.clone();
    let redis_for_rate_limit = state.redis.clone();

    // ── 组装路由 ─────────────────────────────────────────────────
    Router::new()
        .route("/health", get(health::health))
        .route("/docs/openapi.json", get(health::openapi_doc))
        .nest("/api/v1/auth", auth::router())
        .nest("/api/v1", protected_routes)
        // jwt_secret 注入层（auth 中间件需要从 extensions 读取）
        .layer(middleware::from_fn({
            let secret = jwt_secret.clone();
            move |mut req: axum::http::Request<axum::body::Body>,
                  next: axum::middleware::Next| {
                let secret = secret.clone();
                async move {
                    req.extensions_mut().insert(secret);
                    Ok::<_, std::convert::Infallible>(next.run(req).await)
                }
            }
        }))
        // Redis 速率限制（每 60 秒 100 次）
        .layer(middleware::from_fn(create_rate_limiter(redis_for_rate_limit)))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
