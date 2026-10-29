use serde_json::json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::{Extension, Json};
use uuid::Uuid;

use auth::encode_token;
use db::{channels, comments, history, likes, subscriptions, users, videos};
use domain::{
    CommentCreate, HistoryQuery, JwtClaims, LoginRequest, RegisterRequest,
    SubscribeRequest, TokenResponse, User, Video, VideoCreate, VideoStatus,
};

use crate::error::ApiError;

/// Channel PATCH body: update name / description / banner.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelUpdateBody {
    pub name: Option<String>,
    pub description: Option<String>,
    pub banner_url: Option<String>,
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

    let channel_row = channels::find_by_user(&state.db, claims.sub)
        .await?
        .ok_or(ApiError::NotFound)?;
    let vc = VideoCreate {
        channel_id: channel_row.id,
        title,
        description,
        object_key,
        tags,
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
    let items = videos_with_channels(&state.db, &rows).await?;

    Ok(Json(serde_json::Value::Array(items)))
}

/// Serialize video rows, batch-joining each video's channel and the streamUrl slot.
async fn videos_with_channels(
    db: &db::Pool,
    rows: &[db::videos::VideoRow],
) -> Result<Vec<serde_json::Value>, ApiError> {
    let channel_ids: Vec<Uuid> = rows.iter().map(|r| r.channel_id).collect();
    let channel_rows = channels::find_by_ids(db, &channel_ids).await?;
    let channel_map: std::collections::HashMap<Uuid, domain::Channel> = channel_rows
        .into_iter()
        .map(domain::Channel::from)
        .map(|c| (c.id, c))
        .collect();
    Ok(rows
        .iter()
        .cloned()
        .map(|row| {
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
            }
            v
        })
        .collect())
}
/// GET /api/videos/{id} — fetch video + channel; signed stream URL when live.
pub async fn get_video(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let row = videos::find_by_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound)?;
    let channel: Option<domain::Channel> =
        channels::find_by_id(&state.db, row.channel_id)
            .await?
            .map(Into::into);
    let video: Video = row.into();

    let stream_url = if matches!(video.status, VideoStatus::Ready) {
        Some(
            state
                .storage
                .signed_get(&video.object_key, std::time::Duration::from_secs(15 * 60))
                .await
                .map_err(|e| ApiError::Internal(anyhow::anyhow!("storage: {e}")))?,
        )
    } else {
        None
    };

    Ok(Json(serde_json::json!({
        "video": serde_json::to_value(&video).unwrap(),
        "channel": channel,
        "streamUrl": stream_url,
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
    let items = videos_with_channels(&state.db, &rows).await?;

    Ok(Json(serde_json::Value::Array(items)))
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

    let (video_rows, channel_rows) = tokio::join!(
        videos::search(&state.db, &term, limit),
        channels::search(&state.db, &term, limit),
    );
    let video_rows = video_rows?;
    let channel_rows = channel_rows?;

    let vids = videos_with_channels(&state.db, &video_rows).await?;
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
    let _claims = auth::decode_token(&state.config.jwt_secret, &token)
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
    let object_key = format!("videos/{}", Uuid::new_v4());
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

// ---------- Protected routes ----------

/// GET /api/me
pub async fn me(Extension(claims): Extension<JwtClaims>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "id": claims.sub,
        "email": claims.email,
        "name": claims.name,
        "channel": claims.channel,
    }))
}

/// GET /api/history
pub async fn history(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Query(q): Query<HistoryQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = q.limit.unwrap_or(30).clamp(1, 100);
    let rows = history::list_for_user(&state.db, claims.sub, limit).await?;
    let items: Vec<domain::WatchHistory> = rows.into_iter().map(Into::into).collect();
    Ok(Json(serde_json::to_value(items).unwrap()))
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
        let videos_json = videos_with_channels(pool, &selected).await?;
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
    let videos_json = videos_with_channels(pool, &selected).await?;

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
    let _ = likes::like(&state.db, claims.sub, id, None).await?;
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

/// GET /api/channels/{id} — channel info, live subscriber count, paginated videos.
/// GET /api/channels/{id} — channel info, live subscriber count, isSubscribed,
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
    let videos: Vec<Video> = video_rows.into_iter().map(Into::into).collect();

    Ok(Json(serde_json::json!({
        "channel": channel,
        "subscriberCount": subscriber_count,
        "isSubscribed": is_subscribed,
        "videos": videos,
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
    let channel: domain::Channel = updated.into();
    let subscriber_count = subscriptions::count_for_channel(&state.db, id).await?;
    Ok(Json(serde_json::json!({
        "channel": channel,
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


