use api::config::AppConfig;
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = AppConfig::from_env();

    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&config.database_url)
        .await?;

    sqlx::migrate!("../../db/migrations").run(&pool).await?;

    println!("migrations applied");

    Ok(())
}
