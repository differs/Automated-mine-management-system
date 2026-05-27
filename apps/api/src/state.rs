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
            .connect(&config.database_url)
            .await?;

        sqlx::migrate!("../../db/migrations").run(&db).await?;

        Ok(Self { config, db })
    }

    pub fn from_parts(config: AppConfig, db: PgPool) -> Self {
        Self { config, db }
    }
}
