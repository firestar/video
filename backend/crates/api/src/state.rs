//! Shared application state.

use std::sync::Arc;

use db::{create_pool, Pool};
use redis::Client as RedisClient;
use storage::Storage;

use crate::config::Config;

/// Shared state passed to all handlers.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: Pool,
    pub redis: RedisClient,
    pub storage: Storage,
}

impl AppState {
    /// Build state from config: connect to Postgres, Redis, S3.
    pub async fn new(config: Config) -> anyhow::Result<Self> {
        let db = create_pool(&config.database_url).await?;
        let redis = RedisClient::open(config.redis_url.as_str())
            .map_err(|e| anyhow::anyhow!("redis: {e}"))?;
        let storage = Storage::new(
            &config.s3_region,
            config.s3_endpoint.as_deref(),
            &config.s3_bucket,
            &config.s3_access_key,
            &config.s3_secret_key,
            config.s3_base_url.as_deref(),
        )
        .await?;
        Ok(Self {
            config: Arc::new(config),
            db,
            redis,
            storage,
        })
    }
}
