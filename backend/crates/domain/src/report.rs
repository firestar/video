use serde::Deserialize;
use uuid::Uuid;

/// Request body for POST /api/reports.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportCreate {
    /// "video" or "comment".
    pub target_type: String,
    pub target_id: Uuid,
    pub reason: String,
    pub detail: Option<String>,
}
