//! Application configuration loaded from env via figment.

use figment::{providers, Figment};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Address to bind, e.g. "0.0.0.0:8080".
    pub listen: String,
    /// Postgres connection string.
    pub database_url: String,
    /// Redis connection string.
    pub redis_url: String,
    /// JWT secret.
    pub jwt_secret: String,
    /// S3 region.
    pub s3_region: String,
    /// S3 endpoint (for R2/localstack). Empty = AWS default.
    pub s3_endpoint: Option<String>,
    /// S3 bucket.
    pub s3_bucket: String,
    pub s3_access_key: String,
    pub s3_secret_key: String,
    /// Public base URL for S3 objects (CDN).
    pub s3_base_url: Option<String>,
    /// Log level.
    pub log_level: String,
    /// CORS allowed origins (comma-separated).
    pub cors_origins: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listen: "0.0.0.0:8080".into(),
            database_url: "postgres://postgres:postgres@localhost:5432/youtube".into(),
            redis_url: "redis://localhost:6379".into(),
            jwt_secret: "change-me-in-production".into(),
            s3_region: "us-east-1".into(),
            s3_endpoint: None,
            s3_bucket: "videos".into(),
            s3_access_key: "minio".into(),
            s3_secret_key: "minio123".into(),
            s3_base_url: None,
            log_level: "info".into(),
            cors_origins: "*".into(),
        }
    }
}

impl Config {
    /// Load config from env vars (figment).
    pub fn load() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();
        let fig = Figment::from(providers::Serialized::defaults(Config::default()))
            .merge(providers::Env::prefixed("YT_").split(".::"));
        fig.extract()
            .map_err(|e| anyhow::anyhow!("config error: {e}"))
    }
}
