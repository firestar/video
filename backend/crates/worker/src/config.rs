//! Worker config.

use figment::{providers, Figment};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkerConfig {
    pub database_url: String,
    pub redis_url: String,
    pub s3_region: String,
    pub s3_endpoint: Option<String>,
    pub s3_bucket: String,
    pub s3_access_key: String,
    pub s3_secret_key: String,
    pub s3_base_url: Option<String>,
    pub log_level: String,
    /// Path to the FFmpeg binary.
    pub ffmpeg_path: String,
    /// Path to the ffprobe binary.
    pub ffprobe_path: String,
    /// Max attempts per job before marking failed.
    pub max_retries: u32,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            database_url: "postgres://postgres:postgres@localhost:5432/youtube".into(),
            redis_url: "redis://localhost:6379".into(),
            s3_region: "us-east-1".into(),
            s3_endpoint: None,
            s3_bucket: "videos".into(),
            s3_access_key: "minio".into(),
            s3_secret_key: "minio123".into(),
            s3_base_url: None,
            log_level: "info".into(),
            ffmpeg_path: "ffmpeg".into(),
            ffprobe_path: "ffprobe".into(),
            max_retries: 3,
        }
    }
}

impl WorkerConfig {
    pub fn load() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();
        let fig = Figment::from(providers::Serialized::defaults(WorkerConfig::default()))
            .merge(providers::Env::prefixed("YT_").split(".::"));
        fig.extract()
            .map_err(|e| anyhow::anyhow!("worker config error: {e}"))
    }
}
