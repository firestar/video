use serde_json::json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::{Extension, Json};
use uuid::Uuid;

use auth::encode_token;
use db::{analytics, channels, comments, history, likes, notifications, playlists, reports, subscriptions, subtitles, users, videos, watch_later};
use domain::{
    CommentCreate, CommentUpdate, DurationFilter, HistoryQuery, JwtClaims, LoginRequest,
    PlaylistCreate, PlaylistItemAdd, PlaylistUpdate, RegisterRequest, ReportCreate, SearchSort,
    SubscribeRequest, TokenResponse, UploadDateFilter, User, Video, VideoCreate, VideoStatus,
    VideoVisibility,
};

use crate::error::ApiError;

/// Channel PATCH body: update name / description / banner.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelUpdateBody {
    pub name: Option<String>,
    pub description: Option<String>,
    pub banner_url: Option<String>,
    pub avatar_url: Option<String>,
}

/// Video PATCH body: update title / description / tags (owner only).
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoUpdateBody {
    pub title: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub visibility: Option<String>,
    /// Schedule control: absent = unchanged, null = clear schedule (publish
    /// now if ready), timestamp = set a future schedule.
    pub publish_at: Option<Option<chrono::DateTime<chrono::Utc>>>,
}

/// Account PATCH body: update the signed-in user's profile.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeUpdateBody {
    pub name: Option<String>,
    pub email: Option<String>,
    /// None = leave unchanged; Some(s) = set (empty string clears it).
    pub bio: Option<String>,
}

/// Change-password body (authenticated).
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordBody {
    pub current_password: String,
    pub new_password: String,
}

/// Query params for GET /api/home/feed.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeFeedQuery {
    pub limit: Option<i64>,
}

/// Query params for GET /api/videos/{id}/related.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct RelatedQuery {
    pub limit: Option<i64>,
}


/// POST /api/videos with inline auth check (used because the route is
/// shared with GET /api/videos in the same router, so method-level
/// middleware can't be applied per-method in Axum 0.8).
/// POST /api/videos with inline auth check.
pub async fn create_video_with_auth(
    State(state): State<AppState>,
    req: axum::extract::Request,
) -> Result<impl axum::response::IntoResponse, ApiError> {
    let token = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_string())
        .ok_or(ApiError::Unauthorized("missing token".into()))?;
    let claims = auth::decode_token(&state.config.jwt_secret, &token)
        .map_err(|e| ApiError::Unauthorized(e.to_string()))?;
    let body: axum::body::Bytes = axum::body::to_bytes(req.into_body(), usize::MAX)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("body: {e}")))?;
    let req_json: serde_json::Value = serde_json::from_slice(&body)
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let title = req_json["title"].as_str().map(|s| s.to_string())
        .ok_or_else(|| ApiError::BadRequest("title required".into()))?;
    let description = req_json["description"].as_str().map(|s| s.to_string());
    // Accept both casings: presign returns `object_key`, and the upload page
    // + E2E seed echo it back as-is.
    let object_key = req_json
        .get("objectKey")
        .or_else(|| req_json.get("object_key"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .ok_or_else(|| ApiError::BadRequest("objectKey required".into()))?;
    let tags: Vec<String> = req_json["tags"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let visibility = req_json
        .get("visibility")
        .and_then(|v| v.as_str())
        .map(|s| {
            VideoVisibility::parse(s).ok_or_else(|| {
                ApiError::BadRequest("visibility must be public, unlisted, or private".into())
            })
        })
        .transpose()?
        .unwrap_or_default();

    let channel_row = channels::find_by_user(&state.db, claims.sub)
        .await?
        .ok_or(ApiError::NotFound)?;
    let publish_at = match req_json.get("publishAt") {
        None | Some(serde_json::Value::Null) => None,
        Some(v) => Some(
            v.as_str()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .ok_or_else(|| {
                    ApiError::BadRequest("publishAt must be an RFC3339 timestamp".into())
                })?,
        ),
    };
    let is_short = req_json.get("isShort").and_then(|v| v.as_bool()).unwrap_or(false);
    let vc = VideoCreate {
        channel_id: channel_row.id,
        title,
        description,
        object_key,
        tags,
        visibility,
        publish_at,
        is_short,
    };
    let row = videos::create(&state.db, &vc).await?;
    let video: domain::Video = row.into();

    let mut conn = state
        .redis
        .get_multiplexed_tokio_connection()
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("redis: {e}")))?;
    let msg = serde_json::json!({
        "video_id": video.id,
        "object_key": video.object_key,
    });
    redis::cmd("LPUSH")
        .arg("transcode")
        .arg(serde_json::to_string(&msg).unwrap())
        .query_async::<()>(&mut conn)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("redis lpush: {e}")))?;

    Ok((
        axum::http::StatusCode::CREATED,
        axum::Json(serde_json::json!({ "video": serde_json::to_value(&video).unwrap() })),
    ))
}
use crate::state::AppState;

// ---------- Public routes ----------

/// GET /health — liveness probe.
pub async fn health() -> impl axum::response::IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({ "status": "ok" })))
}

/// GET /api/health — app health (DB ping).
pub async fn api_health(State(state): State<AppState>) -> Result<Json<serde_json::Value>, ApiError> {
    let db_ok = db::pool::ping(&state.db)
        .await
        .map(|_| true)
        .unwrap_or(false);
    Ok(Json(serde_json::json!({
        "status": if db_ok { "ok" } else { "degraded" },
        "db": db_ok,
    })))
}

/// POST /api/auth/register
pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<TokenResponse>, ApiError> {
    if let Err(e) = req.validate() {
        return Err(ApiError::BadRequest(e));
    }
    // Friendly message for the only conflict left at sign-up: a duplicate
    // email. (Name collisions are fine now — the channel handle gets a
    // suffix instead.) The DB constraint still backstops the race.
    if users::find_by_email(&state.db, &req.email).await?.is_some() {
        return Err(ApiError::Conflict(
            "an account with this email already exists".into(),
        ));
    }
    let hash = auth::hash_password(&req.password)?;
    let row = users::create(&state.db, &req, &hash).await?;
    let user = User::from(row);

    let claims = JwtClaims {
        sub: user.id,
        email: user.email.clone(),
        name: user.name.clone(),
        channel: user.channel_name.clone(),
        exp: 0,
        iat: 0,
    };
    let token = encode_token(&state.config.jwt_secret, &claims)?;
    Ok(Json(TokenResponse {
        token,
        user,
    }))
}

/// POST /api/auth/login
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<TokenResponse>, ApiError> {
    let row = users::find_by_email(&state.db, &req.email)
        .await?
        .ok_or_else(|| ApiError::Unauthorized("invalid credentials".into()))?;
    if !auth::verify_password(&req.password, &row.password_hash)? {
        return Err(ApiError::Unauthorized("invalid credentials".into()));
    }
    let user = User::from(row);
    let claims = JwtClaims {
        sub: user.id,
        email: user.email.clone(),
        name: user.name.clone(),
        channel: user.channel_name.clone(),
        exp: 0,
        iat: 0,
    };
    let token = encode_token(&state.config.jwt_secret, &claims)?;
    Ok(Json(TokenResponse {
        token,
        user,
    }))
}

/// GET /api/videos — list ready (live) videos with channel info.
/// Query params: limit (default 20, max 100), offset (default 0),
/// sort = "newest" (default) | "trending".
pub async fn list_videos(
    State(state): State<AppState>,
    Query(q): Query<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = q
        .get("limit")
        .and_then(|v| v.as_i64())
        .unwrap_or(20)
        .clamp(1, 100);
    let offset = q
        .get("offset")
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
        .max(0);
    let sort = db::videos::VideoSort::from_str(
        q.get("sort")
            .and_then(|v| v.as_str())
            .unwrap_or("newest"),
    );

    let rows = videos::list_public(&state.db, limit, offset, sort).await?;

    // Batch-load channels for the page.
    let items = videos_with_channels(&state.db, &state.storage, &rows).await?;

    Ok(Json(serde_json::Value::Array(items)))
}

/// Serialize video rows, batch-joining each video's channel and the streamUrl slot.
async fn videos_with_channels(
    db: &db::Pool,
    storage: &storage::Storage,
    rows: &[db::videos::VideoRow],
) -> Result<Vec<serde_json::Value>, ApiError> {
    let channel_ids: Vec<Uuid> = rows.iter().map(|r| r.channel_id).collect();
    let channel_rows = channels::find_by_ids(db, &channel_ids).await?;
    let channel_map: std::collections::HashMap<Uuid, domain::Channel> = channel_rows
        .into_iter()
        .map(domain::Channel::from)
        .map(|c| (c.id, c))
        .collect();
    let mut items = Vec::with_capacity(rows.len());
    for row in rows.iter().cloned() {
        let video: Video = row.into();
        let mut v = serde_json::to_value(&video).unwrap();
        if let Some(map) = v.as_object_mut() {
            let channel = channel_map.get(&video.channel_id);
            map.insert(
                "channel".into(),
                channel
                    .map(|c| serde_json::to_value(c).unwrap())
                    .unwrap_or(serde_json::Value::Null),
            );
            map.insert("streamUrl".into(), serde_json::Value::Null);
            let thumbnail_key = format!("thumbnails/{}.jpg", video.id);
            let thumbnail_url = if storage.exists(&thumbnail_key).await.unwrap_or(false) {
                storage.signed_get(&thumbnail_key, std::time::Duration::from_secs(24 * 3600)).await.ok()
            } else {
                None
            };
            map.insert(
                "thumbnailUrl".into(),
                thumbnail_url.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null),
            );
        }
        items.push(v);
    }
    Ok(items)
}
/// GET /api/videos/{id} — fetch video + channel; signed stream URLs when live.
///
/// `streams` is the multi-quality ladder (highest first), each with a signed
/// `url`. `streamUrl` is the top quality (kept for backward compatibility).
/// Legacy videos without a `streams` array fall back to a single "default"
/// stream from the canonical `object_key`.
pub async fn get_video(
    State(state): State<AppState>,
    claims: Option<Extension<JwtClaims>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let row = videos::find_by_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound)?;
    let channel: Option<domain::Channel> =
        channels::find_by_id(&state.db, row.channel_id)
            .await?
            .map(Into::into);
    // Private videos are only visible to the channel owner. 404 (not 403)
    // so existence is not leaked to anonymous users.
    if row.visibility == "private" {
        let allowed = matches!(
            (claims.as_ref(), &channel),
            (Some(c), Some(ch)) if ch.user_id == c.sub
        );
        if !allowed {
            return Err(ApiError::NotFound);
        }
    }
    let streams_raw: Option<serde_json::Value> = row.streams.clone();
    let video: Video = row.into();

    let ttl = std::time::Duration::from_secs(24 * 3600);
    let (stream_url, streams_value) = if matches!(video.status, VideoStatus::Ready) {
        let parsed: Vec<domain::Stream> = streams_raw
            .as_ref()
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .filter(|v: &Vec<domain::Stream>| !v.is_empty())
            .unwrap_or_default();
        let list: Vec<domain::Stream> = if parsed.is_empty() {
            vec![domain::Stream {
                quality: "default".into(),
                width: 0,
                height: 0,
                object_key: video.object_key.clone(),
                size_bytes: video.file_size.unwrap_or(0),
            }]
        } else {
            parsed
        };
        let mut signed = Vec::with_capacity(list.len());
        for s in &list {
            let url = state
                .storage
                .signed_get(&s.object_key, ttl)
                .await
                .map_err(|e| ApiError::Internal(anyhow::anyhow!("storage: {e}")))?;
            signed.push(serde_json::json!({
                "quality": s.quality,
                "width": if s.width > 0 { serde_json::Value::from(s.width) } else { serde_json::Value::Null },
                "height": if s.height > 0 { serde_json::Value::from(s.height) } else { serde_json::Value::Null },
                "sizeBytes": s.size_bytes,
                "url": url,
            }));
        }
        let stream_url = signed.first().and_then(|v| v.get("url").cloned());
        (stream_url, serde_json::Value::Array(signed))
    } else {
        (None, serde_json::Value::Array(Vec::new()))
    };

    let thumbnail_key = format!("thumbnails/{}.jpg", video.id);
    let thumbnail_url = if state.storage.exists(&thumbnail_key).await.unwrap_or(false) {
        state.storage.signed_get(&thumbnail_key, ttl).await.ok()
    } else {
        None
    };

    let is_owner = matches!(
        (claims.as_ref(), &channel),
        (Some(c), Some(ch)) if ch.user_id == c.sub
    );
    let (has_liked, has_disliked, in_watch_later) = match claims {
        Some(Extension(c)) => (
            likes::has_liked(&state.db, c.sub, id).await.unwrap_or(false),
            likes::has_disliked(&state.db, c.sub, id).await.unwrap_or(false),
            watch_later::contains(&state.db, c.sub, id).await.unwrap_or(false),
        ),
        None => (false, false, false),
    };
    let chapters = domain::parse_chapters(
        video.description.as_deref(),
        &video.title,
        video.duration_secs.map(|d| d as i64),
    );
    let subtitle_items = signed_subtitles(&state, id).await?;
    Ok(Json(serde_json::json!({
        "video": serde_json::to_value(&video).unwrap(),
        "channel": channel,
        "streamUrl": stream_url,
        "streams": streams_value,
        "hasLiked": has_liked,
        "hasDisliked": has_disliked,
        "inWatchLater": in_watch_later,
        "chapters": chapters,
        "subtitles": subtitle_items,
        "isOwner": is_owner,
        "thumbnailUrl": thumbnail_url.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null),
    })))
}

/// GET /api/videos/{id}/related — same channel or shared tags, live only,
/// ordered by views. 404 if the video itself doesn't exist.
pub async fn get_related_videos(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(q): Query<RelatedQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Confirm the video exists (404 otherwise).
    let _ = videos::find_by_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound)?;

    let limit = q.limit.unwrap_or(10).clamp(1, 50);
    let rows = videos::find_related(&state.db, id, limit).await?;
    let items = videos_with_channels(&state.db, &state.storage, &rows).await?;

    Ok(Json(serde_json::Value::Array(items)))
}

/// GET /api/videos?channelId=... — list all videos (every status) for one
/// channel. Used by the owner's channel-management (studio) view. Public
/// (anyone can browse a channel's videos); the frontend gates the page.
pub async fn list_channel_videos(
    State(state): State<AppState>,
    Query(q): Query<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let channel_id = q
        .get("channelId")
        .or_else(|| q.get("channel_id"))
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<Uuid>().ok())
        .ok_or_else(|| ApiError::BadRequest("channelId is required and must be a valid UUID".into()))?;
    let limit = q.get("limit").and_then(|v| v.as_i64()).unwrap_or(50).clamp(1, 100);
    let offset = q.get("offset").and_then(|v| v.as_i64()).unwrap_or(0).max(0);
    let rows = videos::list_for_channel_all(&state.db, channel_id, limit, offset).await?;
    let items = videos_with_channels(&state.db, &state.storage, &rows).await?;
    Ok(Json(serde_json::Value::Array(items)))
}
/// GET /api/channels/{id}/analytics — Studio analytics for a channel (owner only).
///
/// Returns total views/subscribers, a 28-day daily series for each, and the
/// channel's top videos by view count.
pub async fn channel_analytics(
    State(state): State<AppState>,
    claims: Option<Extension<JwtClaims>>,
    Path(channel_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let claims = claims
        .map(|Extension(c)| c)
        .ok_or(ApiError::Unauthorized("missing token".into()))?;
    let channel = channels::find_by_id(&state.db, channel_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    if channel.user_id != claims.sub {
        return Err(ApiError::Forbidden);
    }
    let days: i32 = 28;
    let total_views = analytics::total_views(&state.db, channel_id).await?;
    let total_subscribers = analytics::total_subscribers(&state.db, channel_id).await?;
    let views_by_day = analytics::views_by_day(&state.db, channel_id, days).await?;
    let subscribers_by_day = analytics::subscribers_by_day(&state.db, channel_id, days).await?;
    let top_videos = analytics::top_videos(&state.db, channel_id, 5).await?;
    Ok(Json(serde_json::json!({
        "totalViews": total_views,
        "totalSubscribers": total_subscribers,
        "days": days,
        "viewsByDay": views_by_day,
        "subscribersByDay": subscribers_by_day,
        "topVideos": top_videos,
    })))
}


/// PATCH /api/videos/{id} — update title / description / tags. Owner of the
/// video's channel only.
pub async fn update_video(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
    Json(body): Json<VideoUpdateBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if let Some(v) = &body.visibility {
        if VideoVisibility::parse(v).is_none() {
            return Err(ApiError::BadRequest(
                "visibility must be public, unlisted, or private".into(),
            ));
        }
    }
    let row = videos::find_by_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound)?;
    // Only the channel owner may edit.
    let channel = channels::find_by_id(&state.db, row.channel_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    if channel.user_id != claims.sub {
        return Err(ApiError::Forbidden);
    }
    // Publishing now requires a ready video (streams must exist).
    if body.publish_at == Some(None) && row.status != "ready" {
        return Err(ApiError::BadRequest(
            "video is still processing — it will go live once ready".into(),
        ));
    }
    // Schedule: a timestamp sets a future schedule (not allowed once
    // published); an explicit null clears the schedule and publishes a ready
    // video immediately.
    if let Some(Some(t)) = &body.publish_at {
        if row.published_at.is_some() {
            return Err(ApiError::BadRequest(
                "already-published videos cannot be rescheduled".into(),
            ));
        }
        if *t <= chrono::Utc::now() {
            return Err(ApiError::BadRequest(
                "publishAt must be in the future (send null to publish now)".into(),
            ));
        }
    }
    // Persist the provided fields (None = leave unchanged).
    let updated_row = videos::update(
        &state.db,
        id,
        body.title.as_deref(),
        body.description.as_deref(),
        body.tags.as_deref(),
        body.visibility.as_deref(),
        body.publish_at,
    )
    .await?
    .ok_or(ApiError::NotFound)?;
    let updated_row = if body.publish_at == Some(None) {
        videos::publish_now(&state.db, id).await?.unwrap_or(updated_row)
    } else {
        updated_row
    };
    let video: domain::Video = updated_row.into();
    Ok(Json(serde_json::to_value(&video).unwrap()))
}

/// DELETE /api/videos/{id} — delete a video and clean up the S3 object,
/// thumbnail, and any referencing rows (likes, comments, history). Owner of
/// the video's channel only.
pub async fn delete_video(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let row = videos::find_by_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound)?;
    let channel = channels::find_by_id(&state.db, row.channel_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    if channel.user_id != claims.sub {
        return Err(ApiError::Forbidden);
    }
    let video: domain::Video = row.into();
    // Fetch subtitle object keys BEFORE the row delete (they cascade away).
    let subtitle_rows = subtitles::list_for_video(&state.db, id).await.unwrap_or_default();
    // Remove the DB row first: if it fails we return an error and the video
    // stays fully intact; if it succeeds, a failed storage delete only leaves
    // orphaned objects (reported in the response), never a broken listing.
    videos::delete(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound)?;
    // Delete the S3 objects (video file + thumbnail + subtitles, best-effort).
    let video_key = video.object_key.clone();
    let thumbnail_key = format!("thumbnails/{}.jpg", video.id);
    let video_deleted = if state.storage.exists(&video_key).await.unwrap_or(false) {
        state.storage.delete(&video_key).await.is_ok()
    } else {
        false
    };
    let thumbnail_deleted = if state.storage.exists(&thumbnail_key).await.unwrap_or(false) {
        state.storage.delete(&thumbnail_key).await.is_ok()
    } else {
        false
    };
    let mut subtitle_deleted: Vec<bool> = Vec::with_capacity(subtitle_rows.len());
    for r in &subtitle_rows {
        subtitle_deleted.push(state.storage.delete(&r.object_key).await.is_ok());
    }
    Ok(Json(serde_json::json!({
        "deleted": true,
        "video": serde_json::to_value(&video).unwrap(),
        "videoObjectDeleted": video_deleted,
        "thumbnailDeleted": thumbnail_deleted,
        "subtitlesDeleted": subtitle_deleted,
    })))
}

/// GET /api/comments/{videoId} — threaded list (top-level + replies),
/// batch-joined with the author's name + avatar.
pub async fn list_comments_threaded(
    State(state): State<AppState>,
    Path(video_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Confirm the video exists (404 otherwise).
    let _ = videos::find_by_id(&state.db, video_id)
        .await?
        .ok_or(ApiError::NotFound)?;

    let nodes = comments::list_for_video_threaded(&state.db, video_id, 200, 0).await?;

    // Collect all user ids (top-level + replies).
    let mut user_ids: Vec<Uuid> = Vec::new();
    for n in &nodes {
        user_ids.push(n.comment.user_id);
        for r in &n.replies {
            user_ids.push(r.user_id);
        }
    }
    let user_ids: Vec<Uuid> = user_ids
        .into_iter()
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    // Batch-load users.
    let user_rows = users::find_by_ids(&state.db, &user_ids).await?;
    let user_map: std::collections::HashMap<Uuid, (String, Option<String>)> =
        user_rows.into_iter().map(|u| (u.id, (u.name, u.avatar_url))).collect();

    let enrich = |c: &domain::Comment| -> serde_json::Value {
        let mut v = serde_json::to_value(c).unwrap();
        if let Some(obj) = v.as_object_mut() {
            let author = user_map.get(&c.user_id);
            obj.insert(
                "authorName".into(),
                serde_json::json!(author.map(|(n, _)| n.clone()).unwrap_or_else(|| "Unknown".to_string())),
            );
            obj.insert(
                "authorAvatar".into(),
                serde_json::json!(author.map(|(_, a)| a.clone())),
            );
        }
        v
    };

    let items: Vec<serde_json::Value> = nodes
        .iter()
        .map(|n| {
            let mut top = enrich(&n.comment);
            let replies: Vec<serde_json::Value> =
                n.replies.iter().map(enrich).collect();
            if let Some(obj) = top.as_object_mut() {
                obj.insert("replies".into(), serde_json::json!(replies));
            }
            top
        })
        .collect();

    Ok(Json(serde_json::json!({
        "comments": items,
        "count": items.len(),
    })))
}

/// POST /api/videos/{id}/comments — create a comment (top-level or a reply
/// to a top-level comment via `parentId`).
pub async fn create_comment(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
    Json(req): Json<CommentCreate>,
) -> Result<(StatusCode, Json<domain::Comment>), ApiError> {
    if let Err(e) = req.validate() {
        return Err(ApiError::BadRequest(e));
    }
    // The video must exist.
    let _ = videos::find_by_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound)?;
    let row = comments::create(&state.db, id, claims.sub, &req).await.map_err(|e| match e {
        domain::CommentError::ParentNotFound => ApiError::NotFound,
        domain::CommentError::WrongVideo => ApiError::BadRequest("reply must be on the same video".into()),
        domain::CommentError::NotTopLevel => ApiError::BadRequest("can only reply to top-level comments".into()),
        domain::CommentError::Db(e) => ApiError::Internal(anyhow::anyhow!("comment insert: {e}")),
    })?;
    // Best-effort: notify the video's channel owner (skipped when the commenter is the owner).
    let _ = notifications::notify_new_comment(&state.db, id, claims.sub).await;
    Ok((StatusCode::CREATED, Json(row.into())))
}

/// DELETE /api/comments/{id} — author or video owner only; replies cascade.
pub async fn delete_comment(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let comment = comments::find_by_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound)?;

    // Owner of the video may also delete any comment on it.
    if comment.user_id != claims.sub {
        let video = videos::find_by_id(&state.db, comment.video_id)
            .await?
            .ok_or(ApiError::NotFound)?;
        let channel = channels::find_by_id(&state.db, video.channel_id)
            .await?
            .ok_or(ApiError::NotFound)?;
        if channel.user_id != claims.sub {
            return Err(ApiError::Forbidden);
        }
    }

    let deleted = comments::delete(&state.db, id).await?;
    if !deleted {
        return Err(ApiError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}
/// PATCH /api/comments/{id} — edit your own comment.
pub async fn edit_comment(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
    Json(req): Json<CommentUpdate>,
) -> Result<Json<serde_json::Value>, ApiError> {
    req.validate().map_err(ApiError::BadRequest)?;
    let comment = comments::find_by_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound)?;
    if comment.user_id != claims.sub {
        return Err(ApiError::Forbidden);
    }
    let updated = comments::update_body(&state.db, id, req.body.trim()).await?;
    Ok(Json(serde_json::json!({ "comment": domain::Comment::from(updated) })))
}

/// POST /api/comments/{id}/pin — video owner pins a top-level comment.
pub async fn pin_comment(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let comment = comments::find_by_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound)?;
    if comment.parent_id.is_some() {
        return Err(ApiError::BadRequest("only top-level comments can be pinned".into()));
    }
    let video = videos::find_by_id(&state.db, comment.video_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    let channel = channels::find_by_id(&state.db, video.channel_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    if channel.user_id != claims.sub {
        return Err(ApiError::Forbidden);
    }
    comments::pin(&state.db, id, comment.video_id).await?;
    let updated = comments::find_by_id(&state.db, id).await?.ok_or(ApiError::NotFound)?;
    Ok(Json(serde_json::json!({ "comment": domain::Comment::from(updated) })))
}

/// DELETE /api/comments/{id}/pin — video owner unpins a comment.
pub async fn unpin_comment(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let comment = comments::find_by_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound)?;
    let video = videos::find_by_id(&state.db, comment.video_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    let channel = channels::find_by_id(&state.db, video.channel_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    if channel.user_id != claims.sub {
        return Err(ApiError::Forbidden);
    }
    comments::unpin(&state.db, id).await?;
    let updated = comments::find_by_id(&state.db, id).await?.ok_or(ApiError::NotFound)?;
    Ok(Json(serde_json::json!({ "comment": domain::Comment::from(updated) })))
}


/// POST /api/comments/{id}/like — toggle a like on a comment.
pub async fn toggle_comment_like(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let comment = comments::find_by_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound)?;
    let already = likes::has_comment_liked(&state.db, claims.sub, comment.video_id, id).await?;
    let liked = if already {
        let _ = likes::unlike(&state.db, claims.sub, comment.video_id, Some(id)).await?;
        false
    } else {
        let _ = likes::like(&state.db, claims.sub, comment.video_id, Some(id)).await?;
        true
    };
    Ok(Json(serde_json::json!({ "liked": liked })))
}

/// GET /api/search?q=...&limit=20
/// Full-text video search (title/description) + channel name match.
pub async fn search(
    State(state): State<AppState>,
    Query(q): Query<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let term = q
        .get("q")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if term.is_empty() {
        return Ok(Json(json!({ "videos": [], "channels": [] })));
    }
    let limit = q
        .get("limit")
        .and_then(|v| v.as_i64())
        .unwrap_or(20)
        .clamp(1, 100);
    let sort = q
        .get("sort")
        .and_then(|v| v.as_str())
        .and_then(SearchSort::parse)
        .unwrap_or_default();
    let duration = q
        .get("duration")
        .and_then(|v| v.as_str())
        .and_then(DurationFilter::parse)
        .unwrap_or_default();
    let upload_date = q
        .get("uploadDate")
        .and_then(|v| v.as_str())
        .and_then(UploadDateFilter::parse)
        .unwrap_or_default();
    let rtype = q.get("type").and_then(|v| v.as_str()).unwrap_or("all");

    let video_rows = if rtype == "channel" {
        Vec::new()
    } else {
        videos::search(&state.db, &term, limit, sort, duration, upload_date).await?
    };
    let channel_rows = if rtype == "video" {
        Vec::new()
    } else {
        channels::search(&state.db, &term, limit).await?
    };

    let vids = videos_with_channels(&state.db, &state.storage, &video_rows).await?;
    let chans: Vec<domain::Channel> = channel_rows.into_iter().map(Into::into).collect();

    Ok(Json(json!({
        "videos": vids,
        "channels": chans,
    })))
}

// ---------- Upload routes ----------

/// POST /api/uploads/presign — presigned PUT URL for direct-to-bucket upload.
pub async fn presign_upload(
    State(state): State<AppState>,
    req: axum::extract::Request,
) -> Result<Json<serde_json::Value>, ApiError> {
    let token = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_string())
        .ok_or(ApiError::Unauthorized("missing token".into()))?;
    let claims = auth::decode_token(&state.config.jwt_secret, &token)
        .map_err(|e| ApiError::Unauthorized(e.to_string()))?;
    let body: axum::body::Bytes = axum::body::to_bytes(req.into_body(), usize::MAX)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("body: {e}")))?;
    let req: serde_json::Value = serde_json::from_slice(&body)
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    let file_name = req
        .get("file_name")
        .or_else(|| req.get("fileName"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ApiError::BadRequest("file_name required".into()))?;
    let content_type = req
        .get("content_type")
        .or_else(|| req.get("contentType"))
        .and_then(|v| v.as_str());
    let size = req
        .get("size")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    if size <= 0 {
        return Err(ApiError::BadRequest("size must be > 0".into()));
    }
    let _ = file_name;
    let purpose = req
        .get("purpose")
        .and_then(|v| v.as_str())
        .unwrap_or("video");
    let object_key = match purpose {
        "avatar" => {
            if size > 5 * 1024 * 1024 {
                return Err(ApiError::BadRequest("avatar max 5 MB".into()));
            }
            if !matches!(
                content_type,
                Some("image/jpeg") | Some("image/png") | Some("image/webp")
            ) {
                return Err(ApiError::BadRequest(
                    "avatar must be image/jpeg, image/png or image/webp".into(),
                ));
            }
            format!("avatars/{}.jpg", Uuid::new_v4())
        }
        "banner" => {
            if size > 10 * 1024 * 1024 {
                return Err(ApiError::BadRequest("banner max 10 MB".into()));
            }
            if !matches!(
                content_type,
                Some("image/jpeg") | Some("image/png") | Some("image/webp")
            ) {
                return Err(ApiError::BadRequest(
                    "banner must be image/jpeg, image/png or image/webp".into(),
                ));
            }
            format!("banners/{}.jpg", Uuid::new_v4())
        }
        "thumbnail" => {
            if size > 5 * 1024 * 1024 {
                return Err(ApiError::BadRequest("thumbnail max 5 MB".into()));
            }
            if !matches!(
                content_type,
                Some("image/jpeg") | Some("image/png") | Some("image/webp")
            ) {
                return Err(ApiError::BadRequest(
                    "thumbnail must be image/jpeg, image/png or image/webp".into(),
                ));
            }
            let video_id = req
                .get("videoId")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
                .ok_or_else(|| ApiError::BadRequest("videoId required".into()))?;
            // Ownership: the video's channel must belong to the current user.
            let vrow = videos::find_by_id(&state.db, video_id)
                .await?
                .ok_or(ApiError::NotFound)?;
            let ch = channels::find_by_id(&state.db, vrow.channel_id)
                .await?
                .ok_or(ApiError::NotFound)?;
            if ch.user_id != claims.sub {
                return Err(ApiError::Forbidden);
            }
            format!("thumbnails/{}.jpg", video_id)
        }
        "subtitle" => {
            if size > 2 * 1024 * 1024 {
                return Err(ApiError::BadRequest("subtitle max 2 MB".into()));
            }
            let ext = file_name
                .rsplit('.')
                .next()
                .map(str::to_ascii_lowercase)
                .unwrap_or_default();
            let ok_type = matches!(
                content_type,
                Some("text/vtt") | Some("text/srt") | Some("text/plain")
            );
            if !matches!(ext.as_str(), "vtt" | "srt") || !ok_type {
                return Err(ApiError::BadRequest(
                    "subtitle must be a .vtt or .srt file (text/vtt or text/srt)".into(),
                ));
            }
            let video_id = req
                .get("videoId")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
                .ok_or_else(|| ApiError::BadRequest("videoId required".into()))?;
            let language = req
                .get("language")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| ApiError::BadRequest("language required".into()))?;
            if !valid_language_tag(language) {
                return Err(ApiError::BadRequest(
                    "language must be a BCP-47 tag like 'en' or 'pt-br'".into(),
                ));
            }
            // Ownership: the video's channel must belong to the current user.
            let vrow = videos::find_by_id(&state.db, video_id)
                .await?
                .ok_or(ApiError::NotFound)?;
            let ch = channels::find_by_id(&state.db, vrow.channel_id)
                .await?
                .ok_or(ApiError::NotFound)?;
            if ch.user_id != claims.sub {
                return Err(ApiError::Forbidden);
            }
            format!("subtitles/{}_{}.{}", video_id, normalize_language(language), ext)
        }
        _ => format!("videos/{}", Uuid::new_v4()),
    };
    let upload_url = state
        .storage
        .presign_put(
            &object_key,
            std::time::Duration::from_secs(15 * 60),
            content_type,
        )
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("storage: {e}")))?;
    Ok(Json(serde_json::json!({
        "upload_url": upload_url,
        "object_key": object_key,
    })))
}
// ---------------------------------------------------------------------------
// Subtitles (caption tracks)
// ---------------------------------------------------------------------------

/// Body for POST /api/videos/{id}/subtitles.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleCreate {
    pub language: String,
    pub object_key: String,
}

/// Loose BCP-47 validation: a 2-3 letter primary subtag plus at most one
/// 2-4 letter subtag (e.g. "en", "pt", "pt-br", "zh-hans").
fn valid_language_tag(tag: &str) -> bool {
    let parts: Vec<&str> = tag.split('-').collect();
    if parts.len() > 2 {
        return false;
    }
    let primary = parts[0];
    if !(2..=3).contains(&primary.len()) || !primary.chars().all(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    if parts.len() == 2 {
        let sub = parts[1];
        if !(2..=4).contains(&sub.len()) || !sub.chars().all(|c| c.is_ascii_alphabetic()) {
            return false;
        }
    }
    true
}

/// Normalize a language tag for storage/keys: trim + lowercase.
fn normalize_language(tag: &str) -> String {
    tag.trim().to_ascii_lowercase()
}

/// Convert an SRT document to WebVTT (the only format the browser's
/// `<track>` element supports). SRT cues: an optional numeric index line,
/// a timing line with millisecond commas, then text lines.
fn srt_to_vtt(srt: &str) -> String {
    let mut out = String::from("WEBVTT\n\n");
    for raw_block in srt.replace('\r', "").split("\n\n") {
        let block = raw_block.trim();
        if block.is_empty() {
            continue;
        }
        let mut lines: Vec<&str> = block.lines().collect();
        // Drop the numeric cue index line (SRT only).
        if let Some(first) = lines.first() {
            let t = first.trim();
            if !t.is_empty() && t.chars().all(|c| c.is_ascii_digit()) {
                lines.remove(0);
            }
        }
        for (i, line) in lines.iter().enumerate() {
            let t = line.trim_end();
            if i == 0 && t.contains("-->") {
                // Timing line: SRT uses ,ms; WebVTT uses .ms.
                out.push_str(t.replace(',', ".").trim());
            } else if !t.is_empty() {
                out.push_str(t);
            }
            out.push('\n');
        }
        out.push('\n');
    }
    out
}

/// Sign every track of a video for a response (24h TTL).
async fn signed_subtitles(
    state: &AppState,
    video_id: Uuid,
) -> Result<Vec<serde_json::Value>, ApiError> {
    let rows = subtitles::list_for_video(&state.db, video_id).await?;
    let ttl = std::time::Duration::from_secs(24 * 3600);
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        let url = state
            .storage
            .signed_get(&r.object_key, ttl)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("storage: {e}")))?;
        out.push(serde_json::json!({
            "language": r.language,
            "format": r.format,
            "url": url,
        }));
    }
    Ok(out)
}

/// POST /api/videos/{id}/subtitles — register an uploaded caption track
/// (owner only). The file must already be in storage (presign with purpose
/// "subtitle", then PUT). SRT objects are converted to WebVTT in place.
pub async fn register_subtitle(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
    Json(req): Json<SubtitleCreate>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let language = normalize_language(&req.language);
    if !valid_language_tag(&language) {
        return Err(ApiError::BadRequest(
            "language must be a BCP-47 tag like 'en' or 'pt-br'".into(),
        ));
    }
    let row = videos::find_by_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound)?;
    let channel = channels::find_by_id(&state.db, row.channel_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    if channel.user_id != claims.sub {
        return Err(ApiError::Forbidden);
    }
    if !state.storage.exists(&req.object_key).await.unwrap_or(false) {
        return Err(ApiError::BadRequest(
            "uploaded file not found in storage — presign and PUT first".into(),
        ));
    }
    // SRT → WebVTT: the browser's <track> element only supports VTT.
    let final_key = if req.object_key.ends_with(".srt") {
        let bytes = state
            .storage
            .download(&req.object_key)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("storage: {e}")))?;
        let text = String::from_utf8_lossy(&bytes);
        let vtt = srt_to_vtt(&text);
        let vtt_key = format!("subtitles/{}_{}.vtt", id, language);
        state
            .storage
            .upload(&vtt_key, vtt.into_bytes().into(), "text/vtt")
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("storage: {e}")))?;
        // Best-effort: remove the original SRT object.
        let _ = state.storage.delete(&req.object_key).await;
        vtt_key
    } else {
        req.object_key.clone()
    };
    let saved = subtitles::upsert(&state.db, id, &language, "vtt", &final_key).await?;
    let url = state
        .storage
        .signed_get(&saved.object_key, std::time::Duration::from_secs(24 * 3600))
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("storage: {e}")))?;
    Ok(Json(serde_json::json!({
        "subtitle": {
            "language": saved.language,
            "format": saved.format,
            "url": url,
        }
    })))
}

/// GET /api/videos/{id}/subtitles — public list of tracks with signed URLs.
/// Private videos are owner-only (404, matching get_video).
pub async fn list_subtitles(
    State(state): State<AppState>,
    claims: Option<Extension<JwtClaims>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let row = videos::find_by_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound)?;
    if row.visibility == "private" {
        let channel: Option<domain::Channel> = channels::find_by_id(&state.db, row.channel_id)
            .await?
            .map(Into::into);
        let allowed = matches!(
            (claims.as_ref(), &channel),
            (Some(c), Some(ch)) if ch.user_id == c.sub
        );
        if !allowed {
            return Err(ApiError::NotFound);
        }
    }
    let items = signed_subtitles(&state, id).await?;
    Ok(Json(serde_json::json!({ "subtitles": items })))
}

/// DELETE /api/videos/{id}/subtitles/{language} — remove a track (owner).
pub async fn delete_subtitle(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Path((id, language)): Path<(Uuid, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let row = videos::find_by_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound)?;
    let channel = channels::find_by_id(&state.db, row.channel_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    if channel.user_id != claims.sub {
        return Err(ApiError::Forbidden);
    }
    let language = normalize_language(&language);
    let removed = subtitles::delete(&state.db, id, &language).await?;
    if let Some(r) = &removed {
        let _ = state.storage.delete(&r.object_key).await;
    }
    Ok(Json(serde_json::json!({ "deleted": removed.is_some() })))
}

/// Part size for multipart uploads. Kept well under Cloudflare's 100 MB
/// request-body cap so every part PUT clears the proxy.
const MULTIPART_PART_SIZE: i64 = 50 * 1024 * 1024; // 50 MB
/// Max video size for the multipart path (matches the UI's 2 GB claim).
const MULTIPART_MAX_SIZE: i64 = 2 * 1024 * 1024 * 1024; // 2 GB
/// Presign TTL for each part (generous: a 2 GB upload can outlast 15 min).
const MULTIPART_PART_TTL_SECS: u64 = 60 * 60; // 60 min

/// Number of 50 MB parts needed to hold `size` bytes (>= 1).
fn num_parts_for(size: i64) -> i64 {
    (size + MULTIPART_PART_SIZE - 1) / MULTIPART_PART_SIZE
}

/// Decode the Bearer token from the request, returning the claims.
fn extract_claims(
    state: &AppState,
    req: &axum::extract::Request,
) -> Result<domain::JwtClaims, ApiError> {
    let token = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_string())
        .ok_or(ApiError::Unauthorized("missing token".into()))?;
    auth::decode_token(&state.config.jwt_secret, &token)
        .map_err(|e| ApiError::Unauthorized(e.to_string()))
}

/// Read the request body as a JSON value.
async fn read_json_body(req: axum::extract::Request) -> Result<serde_json::Value, ApiError> {
    let body: axum::body::Bytes = axum::body::to_bytes(req.into_body(), usize::MAX)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("body: {e}")))?;
    serde_json::from_slice(&body).map_err(|e| ApiError::BadRequest(e.to_string()))
}

/// POST /api/uploads/multipart/init — start a multipart upload and presign
/// every part. For files that would exceed Cloudflare's 100 MB single-body
/// cap, the browser uploads the parts individually (each < 100 MB) and then
/// calls /multipart/complete with the part ETags.
pub async fn multipart_init(
    State(state): State<AppState>,
    req: axum::extract::Request,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _claims = extract_claims(&state, &req)?;
    let body = read_json_body(req).await?;
    let content_type = body
        .get("content_type")
        .or_else(|| body.get("contentType"))
        .and_then(|v| v.as_str());
    let size = body.get("size").and_then(|v| v.as_i64()).unwrap_or(0);
    if size <= 0 {
        return Err(ApiError::BadRequest("size must be > 0".into()));
    }
    if size > MULTIPART_MAX_SIZE {
        return Err(ApiError::BadRequest("video max 2 GB".into()));
    }
    let object_key = format!("videos/{}", Uuid::new_v4());
    let upload_id = state
        .storage
        .create_multipart_upload(&object_key, content_type)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("storage: {e}")))?;
    let num_parts = num_parts_for(size);
    let ttl = std::time::Duration::from_secs(MULTIPART_PART_TTL_SECS);
    let mut parts = Vec::with_capacity(num_parts as usize);
    for n in 1..=num_parts {
        let url = state
            .storage
            .presign_put_part(&object_key, &upload_id, n as i32, ttl)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("storage: {e}")))?;
        parts.push(json!({ "part_number": n, "upload_url": url }));
    }
    Ok(Json(json!({
        "object_key": object_key,
        "upload_id": upload_id,
        "part_size": MULTIPART_PART_SIZE,
        "parts": parts,
    })))
}

/// POST /api/uploads/multipart/complete — complete a multipart upload from
/// the uploaded parts (server-side, with the app's credentials).
pub async fn multipart_complete(
    State(state): State<AppState>,
    req: axum::extract::Request,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _claims = extract_claims(&state, &req)?;
    let body = read_json_body(req).await?;
    let object_key = body
        .get("object_key")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ApiError::BadRequest("object_key required".into()))?;
    let upload_id = body
        .get("upload_id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ApiError::BadRequest("upload_id required".into()))?;
    let parts_arr = body
        .get("parts")
        .and_then(|v| v.as_array())
        .ok_or_else(|| ApiError::BadRequest("parts required".into()))?;
    let mut parts: Vec<(i32, String)> = Vec::with_capacity(parts_arr.len());
    for p in parts_arr {
        let n = p
            .get("part_number")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| ApiError::BadRequest("part_number required".into()))? as i32;
        let etag = p
            .get("etag")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| ApiError::BadRequest("etag required".into()))?;
        parts.push((n, etag.to_string()));
    }
    if parts.is_empty() {
        return Err(ApiError::BadRequest("parts must not be empty".into()));
    }
    parts.sort_by_key(|(n, _)| *n);
    state
        .storage
        .complete_multipart_upload(object_key, upload_id, &parts)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("storage: {e}")))?;
    Ok(Json(json!({ "completed": true })))
}

// ---------- Protected routes ----------

/// GET /api/me — current user, read fresh from the DB so profile edits are
/// reflected immediately (the JWT claims go stale after a name/email change).
pub async fn me(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
) -> Json<serde_json::Value> {
    let channel_row = channels::find_by_user(&state.db, claims.sub).await.ok().flatten();
    let user = users::find_by_id(&state.db, claims.sub).await.ok().flatten();
    let (email, name, channel) = match &user {
        Some(u) => (u.email.clone(), u.name.clone(), u.channel_name.clone()),
        None => (claims.email.clone(), claims.name.clone(), claims.channel.clone()),
    };
    Json(serde_json::json!({
        "id": claims.sub,
        "email": email,
        "name": name,
        "channel": channel,
        "bio": user.as_ref().and_then(|u| u.bio.clone()),
        "avatarUrl": user.as_ref().and_then(|u| u.avatar_url.clone()),
        "channelId": channel_row.map(|r| r.id.to_string()),
    }))
}

/// PATCH /api/me — update the signed-in user's name / email / bio.
pub async fn update_me(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Json(req): Json<MeUpdateBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let current = users::find_by_id(&state.db, claims.sub).await?.ok_or(ApiError::NotFound)?;

    let new_name = req
        .name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| current.name.clone());
    if new_name.is_empty() {
        return Err(ApiError::BadRequest("name cannot be empty".into()));
    }

    let new_email = req
        .email
        .as_deref()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| current.email.clone());
    if !new_email.contains('@') || !new_email.contains('.') {
        return Err(ApiError::BadRequest("a valid email is required".into()));
    }
    if new_email != current.email
        && users::find_by_email(&state.db, &new_email).await?.is_some()
    {
        return Err(ApiError::Conflict("email already in use".into()));
    }

    let new_bio = match req.bio {
        None => current.bio.clone(),
        Some(s) => {
            let t = s.trim().to_string();
            if t.is_empty() { None } else { Some(t) }
        }
    };

    let updated = users::update_profile(&state.db, claims.sub, &new_name, &new_email, new_bio)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(serde_json::json!({
        "id": updated.id,
        "email": updated.email,
        "name": updated.name,
        "channelName": updated.channel_name,
        "bio": updated.bio,
        "avatarUrl": updated.avatar_url,
    })))
}

/// POST /api/auth/change-password — verify the current password, set a new one.
pub async fn change_password(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Json(req): Json<ChangePasswordBody>,
) -> Result<StatusCode, ApiError> {
    let row = users::find_by_id(&state.db, claims.sub).await?.ok_or(ApiError::NotFound)?;
    if !auth::verify_password(&req.current_password, &row.password_hash)? {
        return Err(ApiError::Unauthorized("current password is incorrect".into()));
    }
    if req.new_password.len() < 8 {
        return Err(ApiError::BadRequest("new password must be at least 8 characters".into()));
    }
    let hash = auth::hash_password(&req.new_password)?;
    users::update_password_hash(&state.db, claims.sub, &hash)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/reports — report a video or comment for moderation.
pub async fn create_report(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Json(req): Json<ReportCreate>,
) -> Result<StatusCode, ApiError> {
    let target_type = req.target_type.trim().to_lowercase();
    if target_type != "video" && target_type != "comment" {
        return Err(ApiError::BadRequest("targetType must be 'video' or 'comment'".into()));
    }
    if req.reason.trim().is_empty() {
        return Err(ApiError::BadRequest("reason is required".into()));
    }

    // You can't report your own content.
    if target_type == "video" {
        let vrows = videos::find_by_id_all(&state.db, &[req.target_id]).await?;
        let video = match vrows.first() {
            Some(v) => v,
            None => return Err(ApiError::NotFound),
        };
        let channel = channels::find_by_id(&state.db, video.channel_id)
            .await?
            .ok_or(ApiError::NotFound)?;
        if channel.user_id == claims.sub {
            return Err(ApiError::BadRequest("you cannot report your own video".into()));
        }
    } else {
        let comment = comments::find_by_id(&state.db, req.target_id)
            .await?
            .ok_or(ApiError::NotFound)?;
        if comment.user_id == claims.sub {
            return Err(ApiError::BadRequest("you cannot report your own comment".into()));
        }
    }

    let detail = req.detail.as_deref().map(str::trim).filter(|s| !s.is_empty());
    reports::create(&state.db, claims.sub, &target_type, req.target_id, req.reason.trim(), detail)
        .await?;
    Ok(StatusCode::CREATED)
}

/// GET /api/history
pub async fn history(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Query(q): Query<HistoryQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = q.limit.unwrap_or(30).clamp(1, 100);
    let rows = history::list_for_user(&state.db, claims.sub, limit).await?;
    let video_ids: Vec<Uuid> = rows.iter().map(|h| h.video_id).collect();
    let video_rows = videos::find_by_id_all(&state.db, &video_ids).await?;
    let videos_json = videos_with_channels(&state.db, &state.storage, &video_rows).await?;
    let video_by_id: std::collections::HashMap<String, serde_json::Value> = videos_json
        .into_iter()
        .map(|v| {
            let vid = v.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string();
            (vid, v)
        })
        .collect();
    let items = rows
        .into_iter()
        .map(|h| {
            let vid = h.video_id.to_string();
            let mut item = serde_json::json!({
                "videoId": vid,
                "watchedUntilSecs": h.watched_until_secs,
                "updatedAt": h.updated_at,
            });
            if let Some(video_json) = video_by_id.get(&vid) {
                if let (Some(obj), Some(vobj)) = (item.as_object_mut(), video_json.as_object()) {
                    for (k, val) in vobj {
                        if k != "id" {
                            obj.insert(k.clone(), val.clone());
                        }
                    }
                }
            }
            item
        })
        .collect::<Vec<_>>();
    Ok(Json(serde_json::Value::Array(items)))
}

/// GET /api/home/feed — personalized feed for an authenticated user, or
/// trending (top views last 7d) for anonymous users / users with no history.
///
/// Response: `{ videos: [...], reasons: {videoId: "trending"|"subscription"|"related"} }`.
pub async fn home_feed(
    State(state): State<AppState>,
    claims: Option<Extension<JwtClaims>>,
    Query(q): Query<HomeFeedQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = q.limit.unwrap_or(24).clamp(1, 100) as usize;
    let pool = &state.db;

    let maybe_user = claims.map(|c| c.sub);
    let history_rows = match maybe_user {
        Some(uid) => history::list_for_user(pool, uid, 50).await?,
        None => Vec::new(),
    };

    let pool_rows = videos::trending_recent(pool, 100).await?;

    // If no history, just return trending rows.
    if history_rows.is_empty() {
        let selected = pool_rows.iter().take(limit).cloned().collect::<Vec<_>>();
        let videos_json = videos_with_channels(pool, &state.storage, &selected).await?;
        let mut reasons = serde_json::Map::new();
        for v in &videos_json {
            let vid = v.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string();
            reasons.insert(vid, serde_json::Value::String("trending".into()));
        }
        return Ok(Json(serde_json::json!({ "videos": videos_json, "reasons": reasons })));
    }

    // Personalized path.
    let uid = maybe_user.expect("history non-empty implies authed");
    let watched_ids: Vec<Uuid> = history_rows.iter().map(|h| h.video_id).collect();
    let watched = videos::find_by_id_all(pool, &watched_ids).await?;
    let mut tag_freq: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    for w in &watched {
        for t in &w.tags {
            *tag_freq.entry(t.clone()).or_insert(0) += 1;
        }
    }
    let sub_channels = subscriptions::list_channels_for_user(pool, uid).await?;
    let sub_set: std::collections::HashSet<Uuid> = sub_channels.into_iter().collect();

    let now = chrono::Utc::now();
    let mut scored: Vec<(i64, db::videos::VideoRow, String)> = Vec::new();
    for row in pool_rows.clone() {
        let overlap: i64 = row.tags.iter().map(|t| tag_freq.get(t).copied().unwrap_or(0)).sum();
        let sub_boost = if sub_set.contains(&row.channel_id) { 500 } else { 0 };
        let recency = row
            .published_at
            .map(|p| {
                let hours = (now.signed_duration_since(p)).num_hours();
                ((48 - hours.max(0)) * 5).max(0)
            })
            .unwrap_or(0);
        let score = overlap * 100 + sub_boost + recency;
        let reason = if sub_boost > 0 {
            "subscription".to_string()
        } else if overlap > 0 {
            "related".to_string()
        } else {
            "trending".to_string()
        };
        scored.push((score, row, reason));
    }
    scored.sort_by_key(|b| std::cmp::Reverse(b.0));

    let selected: Vec<db::videos::VideoRow> = scored.iter().take(limit).map(|(_, r, _)| r.clone()).collect();
    let videos_json = videos_with_channels(pool, &state.storage, &selected).await?;

    let mut reasons = serde_json::Map::new();
    for v in &videos_json {
        let vid = v.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string();
        let reason = scored
            .iter()
            .find(|(_, r, _)| r.id.to_string() == vid)
            .map(|(_, _, r)| r.clone())
            .unwrap_or_else(|| "trending".to_string());
        reasons.insert(vid, serde_json::Value::String(reason));
    }

    Ok(Json(serde_json::json!({ "videos": videos_json, "reasons": reasons })))
}
/// Query params for GET /api/shorts.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ShortsQuery {
    limit: Option<i64>,
}

/// GET /api/shorts — feed of short-form videos (newest first), with channel info.
pub async fn list_shorts(
    State(state): State<AppState>,
    Query(q): Query<ShortsQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = q.limit.unwrap_or(30).clamp(1, 100);
    let rows = videos::list_shorts(&state.db, limit).await?;
    Ok(Json(serde_json::json!({ "shorts": rows })))
}


/// DELETE /api/history — clear all watch history for the user.
pub async fn clear_history(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
) -> Result<StatusCode, ApiError> {
    history::clear_for_user(&state.db, claims.sub).await?;
    Ok(StatusCode::NO_CONTENT)
}
/// POST /api/history — record a watch position.
pub async fn record_history(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Json(body): Json<serde_json::Value>,
) -> Result<StatusCode, ApiError> {
    let video_id = body
        .get("videoId")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| ApiError::BadRequest("videoId required".into()))?;
    let secs = body
        .get("positionSecs")
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;
    history::upsert(&state.db, claims.sub, video_id, secs).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/subscribe
pub async fn subscribe(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Json(req): Json<SubscribeRequest>,
) -> Result<StatusCode, ApiError> {
    subscriptions::subscribe(&state.db, claims.sub, req.channel_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/subscribe
pub async fn unsubscribe(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Json(req): Json<SubscribeRequest>,
) -> Result<StatusCode, ApiError> {
    subscriptions::unsubscribe(&state.db, claims.sub, req.channel_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/videos/{id}/like
pub async fn like_video(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let row = likes::like(&state.db, claims.sub, id, None).await?;
    // Best-effort: notify the owner only when this is a new like (not a re-like).
    if row.is_some() {
        let _ = notifications::notify_new_like(&state.db, id, claims.sub).await;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/videos/{id}/like
pub async fn unlike_video(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let _ = likes::unlike(&state.db, claims.sub, id, None).await?;
    Ok(StatusCode::NO_CONTENT)
}
 /// POST /api/videos/{id}/dislike
 pub async fn dislike_video(
     State(state): State<AppState>,
     Extension(claims): Extension<JwtClaims>,
     Path(id): Path<Uuid>,
 ) -> Result<StatusCode, ApiError> {
     let _ = likes::dislike(&state.db, claims.sub, id).await?;
     Ok(StatusCode::NO_CONTENT)
 }
 
 /// DELETE /api/videos/{id}/dislike
 pub async fn undislike_video(
     State(state): State<AppState>,
     Extension(claims): Extension<JwtClaims>,
     Path(id): Path<Uuid>,
 ) -> Result<StatusCode, ApiError> {
     let _ = likes::undislike(&state.db, claims.sub, id).await?;
     Ok(StatusCode::NO_CONTENT)
 }
/// GET /api/watch-later — the user's watch-later videos (newest-saved first).
pub async fn list_watch_later(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rows = watch_later::list_for_user(&state.db, claims.sub).await?;
    let videos = videos_with_channels(&state.db, &state.storage, &rows).await?;
    Ok(Json(json!({ "videos": videos })))
}

/// POST /api/watch-later — save a video to the user's watch-later list.
pub async fn add_watch_later(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Json(body): Json<serde_json::Value>,
) -> Result<StatusCode, ApiError> {
    let video_id = body
        .get("videoId")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or(ApiError::BadRequest("videoId is required and must be a valid UUID".into()))?;
    // The video must exist and be viewable by this user (private: owner only).
    let row = videos::find_by_id(&state.db, video_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    let channel = channels::find_by_id(&state.db, row.channel_id).await?;
    let is_owner = channel.as_ref().is_some_and(|c| c.user_id == claims.sub);
    if row.visibility != "public" && !is_owner {
        return Err(ApiError::NotFound);
    }
    watch_later::add(&state.db, claims.sub, video_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/watch-later/{videoId} — remove a video from watch later.
pub async fn remove_watch_later(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Path(video_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    watch_later::remove(&state.db, claims.sub, video_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/videos/{id}/view — record a view.
pub async fn record_view(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    history::record_view(&state.db, id, Some(claims.sub)).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------- Channel routes ----------

/// paginated videos. Protected: requires a valid JWT (anonymous → 401;
/// channel page falls back gracefully).
pub async fn get_channel(
    State(state): State<AppState>,
    claims: Option<Extension<JwtClaims>>,
    Path(id): Path<Uuid>,
    Query(q): Query<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let is_subscribed = match claims {
        Some(Extension(c)) => subscriptions::is_subscribed(&state.db, c.sub, id).await?,
        None => false,
    };

    let row = channels::find_by_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound)?;
    let user_id = row.user_id;
    let channel: domain::Channel = row.into();
    let subscriber_count = subscriptions::count_for_channel(&state.db, id).await?;

    let limit = q
        .get("limit")
        .and_then(|v| v.as_i64())
        .unwrap_or(20)
        .clamp(1, 100);
    let offset = q
        .get("offset")
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
        .max(0);

    let video_rows = videos::list_for_channel(&state.db, id, limit, offset).await?;
    let videos_json = videos_with_channels(&state.db, &state.storage, &video_rows).await?;
    let (video_count, total_views) = videos::count_and_views_for_channel(&state.db, id).await?;

    let mut channel_json = serde_json::to_value(&channel).unwrap();
    channel_json["avatarUrl"] = serde_json::json!(
        users::find_by_id(&state.db, user_id)
            .await?
            .and_then(|u| u.avatar_url)
    );

    Ok(Json(serde_json::json!({
        "channel": channel_json,
        "subscriberCount": subscriber_count,
        "isSubscribed": is_subscribed,
        "videoCount": video_count,
        "totalViews": total_views,
        "videos": videos_json,
    })))
}

/// PATCH /api/channels/{id} with inline auth (merged route: GET public,
/// PATCH owner-only — Axum 0.8 requires all methods in one route).
pub async fn update_channel_with_auth(
    State(state): State<AppState>,
    claims: Option<Extension<JwtClaims>>,
    Path(id): Path<Uuid>,
    Json(body): Json<ChannelUpdateBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let claims = claims
        .map(|Extension(c)| c)
        .ok_or(ApiError::Unauthorized("missing token".into()))?;
    update_channel_inner(&state, claims, id, body).await
}

async fn update_channel_inner(
    state: &AppState,
    claims: JwtClaims,
    id: Uuid,
    body: ChannelUpdateBody,
) -> Result<Json<serde_json::Value>, ApiError> {
    let row = channels::find_by_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound)?;
    if row.user_id != claims.sub {
        return Err(ApiError::Forbidden);
    }
    let updated = channels::update(
        &state.db,
        id,
        body.name.as_deref(),
        body.description.as_deref(),
        body.banner_url.as_deref(),
    )
    .await?
    .ok_or(ApiError::NotFound)?;
    if let Some(avatar_url) = body.avatar_url.as_deref().filter(|s| !s.is_empty()) {
        users::update_avatar(&state.db, row.user_id, avatar_url).await?;
    }
    let channel: domain::Channel = updated.into();
    let mut channel_json = serde_json::to_value(&channel).unwrap();
    channel_json["avatarUrl"] = serde_json::json!(
        users::find_by_id(&state.db, row.user_id)
            .await?
            .and_then(|u| u.avatar_url)
    );
    let subscriber_count = subscriptions::count_for_channel(&state.db, id).await?;
    Ok(Json(serde_json::json!({
        "channel": channel_json,
        "subscriberCount": subscriber_count,
    })))
}

/// POST /api/channels/{id}/subscribe — subscribe to a channel.
/// Idempotent: a second subscribe leaves the count unchanged.
/// 404 when the channel does not exist; 400 when subscribing to own channel.
pub async fn subscribe_to_channel(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let row = channels::find_by_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound)?;
    if row.user_id == claims.sub {
        return Err(ApiError::BadRequest("cannot subscribe to own channel".into()));
    }
    subscriptions::subscribe(&state.db, claims.sub, id).await?;
    let subscriber_count = subscriptions::count_for_channel(&state.db, id).await?;
    Ok(Json(serde_json::json!({
        "channelId": id,
        "subscriberCount": subscriber_count,
    })))
}

/// DELETE /api/channels/{id}/subscribe — unsubscribe from a channel.
/// Idempotent: a second unsubscribe leaves the count unchanged.
/// 404 when the channel does not exist.
pub async fn unsubscribe_from_channel(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _row = channels::find_by_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound)?;
    subscriptions::unsubscribe(&state.db, claims.sub, id).await?;
    let subscriber_count = subscriptions::count_for_channel(&state.db, id).await?;
    Ok(Json(serde_json::json!({
        "channelId": id,
        "subscriberCount": subscriber_count,
    })))
}

/// GET /api/subscriptions — channels the current user is subscribed to.
pub async fn list_subscriptions(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rows = subscriptions::channels_for_user(&state.db, claims.sub).await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let channel: domain::Channel = row.into();
        let subscriber_count = subscriptions::count_for_channel(&state.db, channel.id).await?;
        out.push(serde_json::json!({
            "channel": channel,
            "subscriberCount": subscriber_count,
        }));
    }
    Ok(Json(serde_json::to_value(out).unwrap()))
}

// ---------- Notifications ----------

/// GET /api/notifications — the user's latest notifications (newest first)
/// plus the unread count.
pub async fn list_notifications(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rows = notifications::list_for_user(&state.db, claims.sub, 30).await?;
    let unread = notifications::unread_count(&state.db, claims.sub).await?;
    let items: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|n| {
            serde_json::json!({
                "id": n.id,
                "channelId": n.channel_id,
                "videoId": n.video_id,
                "type": n.r#type,
                "readAt": n.read_at,
                "createdAt": n.created_at,
                "channelName": n.channel_name,
                "channelAvatarUrl": n.channel_avatar_url,
                "videoTitle": n.video_title,
                "videoThumbnailUrl": n.video_thumbnail_url,
            })
        })
        .collect();
    Ok(Json(serde_json::json!({
        "notifications": items,
        "unreadCount": unread,
    })))
}

/// POST /api/notifications/{id}/read — mark one notification read.
pub async fn mark_notification_read(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let updated = notifications::mark_read(&state.db, id, claims.sub).await?;
    if !updated {
        return Err(ApiError::NotFound);
    }
    let unread = notifications::unread_count(&state.db, claims.sub).await?;
    Ok(Json(serde_json::json!({ "unreadCount": unread })))
}

/// POST /api/notifications/read-all — mark every notification read.
pub async fn mark_all_notifications_read(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
) -> Result<Json<serde_json::Value>, ApiError> {
    notifications::mark_all_read(&state.db, claims.sub).await?;
    Ok(Json(serde_json::json!({ "unreadCount": 0 })))
}

// ---------- Playlist routes ----------

/// POST /api/playlists — create a playlist.
pub async fn create_playlist(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Json(req): Json<PlaylistCreate>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let title = req.title.trim();
    if title.is_empty() {
        return Err(ApiError::BadRequest("title is required".into()));
    }
    let row = playlists::create(
        &state.db,
        claims.sub,
        title,
        req.description.as_deref().unwrap_or(""),
        req.is_public.unwrap_or(false),
    )
    .await?;
    let p: domain::Playlist = row.into();
    Ok(Json(serde_json::to_value(&p).unwrap()))
}

/// GET /api/playlists — the user's playlists, newest first.
pub async fn list_playlists(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rows = playlists::list_for_user(&state.db, claims.sub).await?;
    let items: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| serde_json::to_value(domain::Playlist::from(r)).unwrap())
        .collect();
    Ok(Json(serde_json::json!({ "playlists": items })))
}

/// GET /api/playlists/{id} — playlist + its videos. Public when the playlist is
/// public, otherwise owner-only. Videos carry signed stream URLs + channel.
pub async fn get_playlist(
    State(state): State<AppState>,
    claims: Option<Extension<JwtClaims>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let row = playlists::find_by_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound)?;
    let is_owner = match &claims {
        Some(Extension(c)) => c.sub == row.user_id,
        None => false,
    };
    if !row.is_public && !is_owner {
        return Err(ApiError::NotFound);
    }
    let p: domain::Playlist = row.clone().into();

    let video_ids = playlists::video_ids(&state.db, id).await?;
    let video_rows = videos::find_by_id_all(&state.db, &video_ids).await?;
    // Keep position order (find_by_id_all returns in input order).
    let videos_json = videos_with_channels(&state.db, &state.storage, &video_rows).await?;

    Ok(Json(serde_json::json!({
        "playlist": serde_json::to_value(&p).unwrap(),
        "videos": videos_json,
    })))
}

/// PATCH /api/playlists/{id} — update title/description/visibility (owner).
pub async fn update_playlist(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
    Json(req): Json<PlaylistUpdate>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let title = req.title.as_deref().map(str::trim);
    if let Some(t) = title {
        if t.is_empty() {
            return Err(ApiError::BadRequest("title cannot be empty".into()));
        }
    }
    let row = playlists::update(
        &state.db,
        id,
        claims.sub,
        title,
        req.description.as_deref(),
        req.is_public,
    )
    .await?
    .ok_or(ApiError::NotFound)?;
    // Re-fetch with the video count.
    let full = playlists::find_by_id(&state.db, id).await?;
    let p: domain::Playlist = full.unwrap_or(row).into();
    Ok(Json(serde_json::to_value(&p).unwrap()))
}

/// DELETE /api/playlists/{id} — delete a playlist (owner).
pub async fn delete_playlist(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let deleted = playlists::delete(&state.db, id, claims.sub).await?;
    if !deleted {
        return Err(ApiError::NotFound);
    }
    Ok(Json(serde_json::json!({ "deleted": true })))
}

/// POST /api/playlists/{id}/items — add a video to a playlist (owner).
pub async fn add_playlist_item(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
    Json(req): Json<PlaylistItemAdd>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // The video must exist.
    if videos::find_by_id(&state.db, req.video_id).await?.is_none() {
        return Err(ApiError::NotFound);
    }
    let added = playlists::add_item(&state.db, id, claims.sub, req.video_id).await?;
    if !added && !playlists::has_item(&state.db, id, req.video_id).await? {
        // Not added and not already present → playlist not found / not owner.
        return Err(ApiError::NotFound);
    }
    let count = playlists::find_by_id(&state.db, id).await?.map(|r| r.video_count).unwrap_or(0);
    Ok(Json(serde_json::json!({
        "added": added,
        "videoCount": count,
    })))
}

/// DELETE /api/playlists/{id}/items/{videoId} — remove a video (owner).
pub async fn remove_playlist_item(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Path((id, video_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let removed = playlists::remove_item(&state.db, id, claims.sub, video_id).await?;
    if !removed && !playlists::is_owner(&state.db, id, claims.sub).await? {
        return Err(ApiError::NotFound);
    }
    let count = playlists::find_by_id(&state.db, id).await?.map(|r| r.video_count).unwrap_or(0);
    Ok(Json(serde_json::json!({
        "removed": removed,
        "videoCount": count,
    })))
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn num_parts_rounds_up() {
        let mb = 1024 * 1024;
        // Exactly one part.
        assert_eq!(num_parts_for(50 * mb), 1);
        // One byte over -> two parts.
        assert_eq!(num_parts_for(50 * mb + 1), 2);
        // Exactly two parts.
        assert_eq!(num_parts_for(100 * mb), 2);
        // 101 MB -> three parts (the Cloudflare-boundary case).
        assert_eq!(num_parts_for(101 * mb), 3);
        // 2 GB -> 41 parts (2048 / 50 = 40.96).
        assert_eq!(num_parts_for(2 * 1024 * mb), 41);
        // Tiny file -> still one part.
        assert_eq!(num_parts_for(1), 1);
    }
}
