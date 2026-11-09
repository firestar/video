//! Worker crate: FFmpeg transcode pipeline.
//!
//! Connects to Postgres + Redis + R2, BRPOPs transcode jobs off the
//! `transcode` list, runs the full FFmpeg pipeline (ffprobe → transcode →
//! thumbnail), uploads outputs to R2, and finalizes the DB row.

mod config;
mod jobs;
mod queue;
mod recovery;

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

    // Schedule tick: every 30s, publish scheduled videos whose time arrived.
    let schedule_db = state.db.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            match db::videos::release_due(&schedule_db).await {
                Ok(n) if n > 0 => info!(released = n, "published scheduled videos"),
                Ok(_) => {}
                Err(e) => warn!(error = ?e, "schedule tick failed"),
            }
        }
    });

    // Startup recovery: re-enqueue videos stuck in `processing` — their job
    // was BRPOP'd by a worker instance that died mid-pipeline, so the Redis
    // list no longer holds it. Duplicates (a job still in the list at death)
    // are skipped at pop time by the status guard in jobs::process.
    match db::videos::find_processing(&state.db).await {
        Ok(rows) if !rows.is_empty() => {
            for job in recovery::jobs(&rows) {
                if let Err(e) = queue.enqueue(&job).await {
                    warn!(video_id = %job.video_id, error = ?e, "recovery enqueue failed");
                }
            }
            info!(recovered = rows.len(), "re-enqueued stuck processing videos");
        }
        Ok(_) => {}
        Err(e) => warn!(error = ?e, "startup recovery scan failed"),
    }
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
