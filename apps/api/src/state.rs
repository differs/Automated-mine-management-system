use std::sync::Arc;

use redis::aio::ConnectionManager;
use sqlx::{PgPool, postgres::PgPoolOptions};
use tokio::sync::RwLock;

use crate::{config::{AppConfig, DispatchMode}, modules::ws::{EventTx, create_event_channel}};

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub db: PgPool,
    pub redis: ConnectionManager,
    pub ws_tx: EventTx,
    /// 运行时可变的调度模式（多请求共享，读多写少）
    pub dispatch_mode: Arc<RwLock<DispatchMode>>,
}

impl AppState {
    pub async fn bootstrap(config: AppConfig) -> anyhow::Result<Self> {
        let db = PgPoolOptions::new()
            .max_connections(10)
            .connect(&config.database_url)
            .await?;

        sqlx::migrate!("../../db/migrations").run(&db).await?;

        let redis_client = redis::Client::open(config.redis_url.as_str())?;
        let redis = ConnectionManager::new(redis_client).await?;

        let ws_tx = create_event_channel();
        let dispatch_mode = Arc::new(RwLock::new(config.dispatch_mode.clone()));

        Ok(Self { config, db, redis, ws_tx, dispatch_mode })
    }

    pub fn from_parts(config: AppConfig, db: PgPool, redis: ConnectionManager, ws_tx: EventTx) -> Self {
        let dispatch_mode = Arc::new(RwLock::new(config.dispatch_mode.clone()));
        Self { config, db, redis, ws_tx, dispatch_mode }
    }

    /// 创建测试用 AppState（不需要真实 Redis 连接）
    #[cfg(test)]
    pub async fn for_test(config: AppConfig, db: PgPool) -> Self {
        let redis_client = redis::Client::open(config.redis_url.as_str())
            .expect("failed to create redis client for test");
        let redis = ConnectionManager::new(redis_client)
            .await
            .expect("failed to connect redis for test");
        let ws_tx = create_event_channel();
        let dispatch_mode = Arc::new(RwLock::new(config.dispatch_mode.clone()));
        Self { config, db, redis, ws_tx, dispatch_mode }
    }
}
