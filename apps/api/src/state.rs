use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::config::AppConfig;

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub db: PgPool,
}

impl AppState {
    pub async fn bootstrap(config: AppConfig) -> anyhow::Result<Self> {
        let db = PgPoolOptions::new()
            .max_connections(10)
            .connect_lazy(&config.database_url)?;

        Ok(Self { config, db })
    }
}
