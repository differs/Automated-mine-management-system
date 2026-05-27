use std::net::SocketAddr;

use api::{app, config, state};
use anyhow::Context;
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = config::AppConfig::from_env();

    tracing_subscriber::fmt()
        .with_env_filter(config.rust_log.clone())
        .init();

    let state = state::AppState::bootstrap(config.clone()).await?;
    let app = app::build_router(state);

    let addr: SocketAddr = format!("{}:{}", config.host, config.port)
        .parse()
        .context("invalid bind address")?;

    let listener = TcpListener::bind(addr).await?;
    info!("api listening on http://{}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
