//! Job queue over Redis (LIST-based, Phase 1 simple).
//!
//! Producer: the API pushes `{"videoId": <uuid>, "objectKey": <key>}` onto
//! the `transcode` list. Worker BRPOPs with a 30s timeout.

use redis::{Client, RedisResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Default blocking timeout for `pop` (seconds).
pub const POP_TIMEOUT_SECS: i64 = 30;

/// A transcode job descriptor.
///
/// Matches the payload the API produces (`backend/crates/api/src/routes.rs`):
/// `video_id` = the video row id, `object_key` = the source file key in R2.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscodeJob {
    pub video_id: Uuid,
    pub object_key: String,
}

impl std::fmt::Display for TranscodeJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "video={} src={}", self.video_id, self.object_key)
    }
}

/// A thin Redis LIST-based job queue.
pub struct JobQueue {
    client: Client,
    list: String,
    /// Blocking timeout in seconds for `pop`.
    timeout_secs: i64,
}

impl JobQueue {
    pub async fn new(client: &Client, list: &str) -> RedisResult<Self> {
        Ok(Self {
            client: client.clone(),
            list: list.to_string(),
            timeout_secs: POP_TIMEOUT_SECS,
        })
    }

    /// Override the pop timeout (seconds).
    pub fn with_timeout(mut self, secs: i64) -> Self {
        self.timeout_secs = secs.max(0);
        self
    }

    /// Pop a job from the queue (blocking BRPOP, `timeout_secs`).
    ///
    /// Returns `None` on timeout (no job within the window).
    pub async fn pop(&self) -> anyhow::Result<Option<TranscodeJob>> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let result: Option<String> = redis::cmd("BRPOP")
            .arg(&self.list)
            .arg(self.timeout_secs)
            .query_async(&mut conn)
            .await?;
        match result {
            Some(payload) => Ok(Some(serde_json::from_str(&payload)?)),
            None => Ok(None),
        }
    }

    /// Publish a message to a channel (fire-and-forget).
    pub async fn publish(&self, channel: &str, message: &str) -> anyhow::Result<()> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        redis::cmd("PUBLISH")
            .arg(channel)
            .arg(message)
            .query_async::<()>(&mut conn)
            .await?;
        Ok(())
    }
}
