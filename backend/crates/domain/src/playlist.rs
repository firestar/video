//! Playlist entity types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A user-owned playlist.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub description: String,
    pub is_public: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Number of videos in the playlist (populated by list queries).
    #[serde(default)]
    pub video_count: i64,
}

/// Request body for POST /api/playlists.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistCreate {
    pub title: String,
    pub description: Option<String>,
    pub is_public: Option<bool>,
}

/// Request body for PATCH /api/playlists/{id}.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistUpdate {
    pub title: Option<String>,
    pub description: Option<String>,
    pub is_public: Option<bool>,
}

/// Request body for POST /api/playlists/{id}/items.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistItemAdd {
    pub video_id: Uuid,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playlist_serializes() {
        let p = Playlist {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            title: "T".into(),
            description: "d".into(),
            is_public: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            video_count: 0,
        };
        let j = serde_json::to_string(&p).unwrap();
        assert!(j.contains("\"userId\""));
        assert!(j.contains("\"videoCount\""));
    }

    #[test]
    fn playlist_create_deserializes() {
        let r: PlaylistCreate =
            serde_json::from_str(r#"{"title":"My list","isPublic":true}"#).unwrap();
        assert_eq!(r.title, "My list");
        assert_eq!(r.is_public, Some(true));
    }
}
