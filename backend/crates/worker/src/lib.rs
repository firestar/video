//! Worker crate: FFmpeg transcode pipeline.
//!
//! Connects to Postgres + Redis + R2, BRPOPs transcode jobs off the
//! `transcode` list, runs the full FFmpeg pipeline (ffprobe → transcode →
//! thumbnail), uploads outputs to R2, and finalizes the DB row.

mod config;
mod jobs;
mod queue;

pub use config::WorkerConfig;

use std::sync::Arc;

use db::{create_pool, Pool};
use redis::Client as RedisClient;
use storage::Storage;
use tracing::{info, warn};

/// Redis list name the API pushes jobs onto.
pub const TRANSCODE_QUEUE: &str = "transcode";

/// Shared worker state.
#[derive(Clone)]
pub struct WorkerState {
    pub config: Arc<WorkerConfig>,
    pub db: Pool,
    pub redis: RedisClient,
    pub storage: Storage,
}

impl WorkerState {
    pub async fn new(config: WorkerConfig) -> anyhow::Result<Self> {
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

/// Run the worker loop: BRPOP from Redis `transcode` list, process jobs.
pub async fn run(state: WorkerState) -> anyhow::Result<()> {
    info!(
        ffmpeg = %state.config.ffmpeg_path,
        ffprobe = %state.config.ffprobe_path,
        max_retries = state.config.max_retries,
        "worker starting"
    );
    let queue = queue::JobQueue::new(&state.redis, TRANSCODE_QUEUE)
        .await?
        .with_timeout(queue::POP_TIMEOUT_SECS);
    queue
        .publish("worker:ready", "worker online")
        .await?;

    loop {
        match queue.pop().await {
            Ok(Some(job)) => {
                info!(job = %job, "popped job");
                if let Err(e) = jobs::process(&state, &queue, &job).await {
                    warn!(job = %job, error = ?e, "job failed (already marked)");
                }
            }
            Ok(None) => {
                // BRPOP timeout — no job in the window; loop continues.
            }
            Err(e) => {
                warn!(error = ?e, "queue pop error; retrying in 2s");
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }
    }
}
