use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Processing state of a video.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VideoStatus {
    /// Upload received, awaiting transcode.
    Pending,
    /// FFmpeg job running.
    Processing,
    /// Ready to serve.
    Ready,
    /// Transcode failed.
    Failed,
}

/// Who may see a video.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VideoVisibility {
    /// Visible to everyone, appears in listings and search.
    #[default]
    Public,
    /// Viewable by anyone with the URL; hidden from listings and search.
    Unlisted,
    /// Only the channel owner.
    Private,
}

impl VideoVisibility {
    /// Parse an API value; unknown -> None.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "public" => Some(Self::Public),
            "unlisted" => Some(Self::Unlisted),
            "private" => Some(Self::Private),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Unlisted => "unlisted",
            Self::Private => "private",
        }
    }
}

/// A video.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Video {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub thumbnail_url: Option<String>,
    pub object_key: String,
    pub tags: Vec<String>,
    pub status: VideoStatus,
    pub visibility: VideoVisibility,
    pub duration_secs: Option<i32>,
    pub file_size: Option<i64>,
    pub view_count: i64,
    pub like_count: i64,
    pub dislike_count: i64,
    pub published_at: Option<DateTime<Utc>>,
    /// When a scheduled video becomes visible (NULL = not scheduled).
    pub publish_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}
/// One chapter parsed from a video description (YouTube-style timestamps).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Chapter {
    /// Start offset in seconds from the beginning of the video.
    pub start_secs: i64,
    pub title: String,
}

/// Search sort order.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SearchSort {
    #[default]
    Relevance,
    Date,
    Views,
}
impl SearchSort {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "relevance" => Some(Self::Relevance),
            "date" => Some(Self::Date),
            "views" => Some(Self::Views),
            _ => None,
        }
    }
}

/// Search duration filter.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DurationFilter {
    #[default]
    Any,
    Short,
    Medium,
    Long,
}
impl DurationFilter {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "any" => Some(Self::Any),
            "short" => Some(Self::Short),
            "medium" => Some(Self::Medium),
            "long" => Some(Self::Long),
            _ => None,
        }
    }
}

/// Search upload-date recency filter.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum UploadDateFilter {
    #[default]
    Any,
    Hour,
    Day,
    Week,
    Month,
    Year,
}
impl UploadDateFilter {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "any" => Some(Self::Any),
            "hour" => Some(Self::Hour),
            "day" => Some(Self::Day),
            "week" => Some(Self::Week),
            "month" => Some(Self::Month),
            "year" => Some(Self::Year),
            _ => None,
        }
    }
}

/// Parse chapters out of a video description.
///
/// Rules (mirroring YouTube):
/// - A chapter line starts at the beginning of the line with a timestamp
///   `M:SS` or `H:MM:SS` followed by whitespace and a non-empty title.
/// - The first chapter must start at 0:00; if the description's first
///   timestamp is later, a 0:00 chapter titled after the video is prepended.
/// - Start times must be strictly increasing; a chapter shorter than 10 s
///   (except the last) is dropped.
/// - Timestamps beyond the video duration (when known) are ignored.
/// - At most 50 chapters; fewer than 2 results in an empty list.
pub fn parse_chapters(
    description: Option<&str>,
    video_title: &str,
    duration_secs: Option<i64>,
) -> Vec<Chapter> {
    let Some(desc) = description else { return Vec::new() };
    let mut found: Vec<(i64, String)> = Vec::new();
    for line in desc.lines() {
        if let Some((secs, title)) = parse_timestamp_line(line) {
            if let Some(d) = duration_secs {
                if secs > d {
                    continue;
                }
            }
            found.push((secs, title));
            if found.len() >= 50 {
                break;
            }
        }
    }
    if found.is_empty() {
        return Vec::new();
    }
    if found[0].0 != 0 {
        found.insert(0, (0, video_title.trim().to_string()));
    }
    let mut out: Vec<(i64, String)> = Vec::new();
    for (i, (secs, title)) in found.iter().enumerate() {
        let secs = *secs;
        if out.is_empty() {
            out.push((secs, title.clone()));
            continue;
        }
        let prev = out.last().unwrap().0;
        if secs <= prev {
            continue; // must be strictly increasing
        }
        if i == found.len() - 1 {
            out.push((secs, title.clone())); // last chapter: keep any length
        } else if secs - prev >= 10 {
            out.push((secs, title.clone()));
        }
        // else: shorter than 10 s before the next chapter — dropped
    }
    if out.len() < 2 {
        return Vec::new();
    }
    out.into_iter()
        .map(|(start_secs, title)| Chapter { start_secs, title })
        .collect()
}

/// Parse one line: leading timestamp + whitespace + non-empty title.
fn parse_timestamp_line(line: &str) -> Option<(i64, String)> {
    let (time_part, title_part) = line.split_once(' ')?;
    let title = title_part.trim();
    if title.is_empty() {
        return None;
    }
    let parts: Vec<&str> = time_part.split(':').collect();
    if parts.len() < 2 || parts.len() > 3 {
        return None;
    }
    let (h, m, s) = match parts.len() {
        2 => ("0", parts[0], parts[1]),
        3 => (parts[0], parts[1], parts[2]),
        _ => unreachable!(),
    };
    let h: i64 = h.parse().ok()?;
    let m: i64 = m.parse().ok()?;
    let s: i64 = s.parse().ok()?;
    if h > 999 || m > 59 || s > 59 {
        return None;
    }
    Some((h * 3600 + m * 60 + s, title.to_string()))
}

/// One transcode variant of a video (an entry of `videos.streams`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stream {
    /// Display label, e.g. "1080p", "720p" (or "default" for legacy single-stream videos).
    pub quality: String,
    pub width: i32,
    pub height: i32,
    /// Object key in the media bucket (signed on demand by the API).
    pub object_key: String,
    pub size_bytes: i64,
}

/// Fields set when a video row is first created (upload started).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoCreate {
    pub channel_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub object_key: String,
    pub tags: Vec<String>,
    #[serde(default)]
    pub visibility: VideoVisibility,
    /// Schedule: publish at this time instead of immediately (future only).
    #[serde(default)]
    pub publish_at: Option<DateTime<Utc>>,
}

/// Fields updated by the worker after transcode.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoFinalize {
    pub status: VideoStatus,
    pub duration_secs: Option<i32>,
    pub file_size: Option<i64>,
    pub thumbnail_url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn video_status_roundtrip() {
        for v in [
            VideoStatus::Pending,
            VideoStatus::Processing,
            VideoStatus::Ready,
            VideoStatus::Failed,
        ] {
            let s = serde_json::to_string(&v).unwrap();
            let b: VideoStatus = serde_json::from_str(&s).unwrap();
            assert_eq!(v, b);
        }
    }

    #[test]
    fn chapters_basic_parse() {
        let desc = "Some intro text\n0:00 Intro\n1:30 The main part\n5:00 Conclusion\nThanks for watching";
        let ch = parse_chapters(Some(desc), "My Video", None);
        assert_eq!(ch.len(), 3);
        assert_eq!(ch[0].start_secs, 0);
        assert_eq!(ch[0].title, "Intro");
        assert_eq!(ch[1].start_secs, 90);
        assert_eq!(ch[1].title, "The main part");
        assert_eq!(ch[2].start_secs, 300);
        assert_eq!(ch[2].title, "Conclusion");
    }

    #[test]
    fn chapters_prepends_zero_when_missing() {
        let desc = "0:30 Start\n2:00 End";
        let ch = parse_chapters(Some(desc), "My Video", None);
        assert_eq!(ch.len(), 3);
        assert_eq!(ch[0].start_secs, 0);
        assert_eq!(ch[0].title, "My Video");
        assert_eq!(ch[1].start_secs, 30);
        assert_eq!(ch[2].start_secs, 120);
    }

    #[test]
    fn chapters_hours_format() {
        let desc = "0:00 Intro\n1:00:00 One hour in";
        let ch = parse_chapters(Some(desc), "V", None);
        assert_eq!(ch.len(), 2);
        assert_eq!(ch[1].start_secs, 3600);
    }

    #[test]
    fn chapters_drops_short_and_out_of_order() {
        // 0:00 -> 0:05 (5s, dropped) -> 0:20 (kept) -> 0:10 (out of order, dropped)
        let desc = "0:00 A\n0:05 B\n0:20 C\n0:10 D";
        let ch = parse_chapters(Some(desc), "V", None);
        let secs: Vec<i64> = ch.iter().map(|c| c.start_secs).collect();
        assert_eq!(secs, vec![0, 20]);
    }

    #[test]
    fn chapters_keeps_last_even_if_short() {
        let desc = "0:00 A\n10:00 B\n10:05 C";
        let ch = parse_chapters(Some(desc), "V", None);
        let secs: Vec<i64> = ch.iter().map(|c| c.start_secs).collect();
        assert_eq!(secs, vec![0, 600, 605]);
    }

    #[test]
    fn chapters_ignores_beyond_duration() {
        let desc = "0:00 A\n5:00 B\n99:00 Too long";
        let ch = parse_chapters(Some(desc), "V", Some(300));
        let secs: Vec<i64> = ch.iter().map(|c| c.start_secs).collect();
        assert_eq!(secs, vec![0, 300]);
    }

    #[test]
    fn chapters_empty_when_fewer_than_two() {
        assert!(parse_chapters(Some("0:00 Only one"), "V", None).is_empty());
        assert!(parse_chapters(Some("no timestamps here"), "V", None).is_empty());
        assert!(parse_chapters(None, "V", None).is_empty());
    }

    #[test]
    fn chapters_ignores_non_chapter_lines() {
        // Timestamp not at line start, or with no title, must be ignored.
        let desc = "check out 1:30 later\n0:00\n0:00 Real chapter\n3:00 Next";
        let ch = parse_chapters(Some(desc), "V", None);
        let secs: Vec<i64> = ch.iter().map(|c| c.start_secs).collect();
        assert_eq!(secs, vec![0, 180]);
    }

    #[test]
    fn video_serializes() {
        let v = Video {
            id: Uuid::new_v4(),
            channel_id: Uuid::new_v4(),
            title: "T".into(),
            description: None,
            thumbnail_url: None,
            object_key: "videos/x".into(),
            tags: vec!["a".into()],
            status: VideoStatus::Pending,
            visibility: VideoVisibility::Public,
            duration_secs: None,
            file_size: None,
            view_count: 0,
            like_count: 0,
            dislike_count: 0,
            published_at: None,
            publish_at: None,
            created_at: Utc::now(),
        };
        let s = serde_json::to_string(&v).unwrap();
        assert!(s.contains("\"channelId\""));
        assert!(s.contains("\"viewCount\""));
        assert!(s.contains("\"objectKey\""));
        assert!(s.contains("\"tags\""));
        assert!(s.contains("\"visibility\""));
        assert!(s.contains("\"public\""));
    }
}
