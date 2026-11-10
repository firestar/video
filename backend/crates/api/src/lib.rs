//! API crate: Axum server with middleware, routes, health check.

mod config;
mod error;
mod middleware;
mod routes;
mod state;

pub use config::Config;
pub use error::ApiError;
pub use state::AppState;

use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

/// Build the Axum router with all middleware and routes.
pub fn build_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
        .max_age(std::time::Duration::from_secs(60));

    let public_routes = Router::new()
        .route("/health", axum::routing::get(routes::health))
        .route("/api/health", axum::routing::get(routes::api_health))
        .route(
            "/api/auth/register",
            axum::routing::post(routes::register),
        )
        .route("/api/auth/login", axum::routing::post(routes::login))
        .route("/api/search", axum::routing::get(routes::search))
        .route(
            "/api/uploads/presign",
            axum::routing::post(routes::presign_upload),
        )
        .route(
            "/api/uploads/multipart/init",
            axum::routing::post(routes::multipart_init),
        )
        .route(
            "/api/uploads/multipart/complete",
            axum::routing::post(routes::multipart_complete),
        )
        .route(
            "/api/videos/{id}/related",
            axum::routing::get(routes::get_related_videos),
        )
        // `{id}` = video id here; must match the param name used by
        // DELETE /api/comments/{id} (matchit rejects differing names).
        .route(
            "/api/comments/{id}",
            axum::routing::get(routes::list_comments_threaded),
        )
        .layer(axum::middleware::from_fn(middleware::rate_limit))
        .with_state(state.clone());

    // /api/home/feed: public, but personalized when a valid token is present.
    let home_feed_routes = Router::new()
        .route("/api/home/feed", axum::routing::get(routes::home_feed))
        .layer(axum::middleware::from_fn(middleware::rate_limit))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::optional_auth_middleware,
        ))
        .with_state(state.clone());

    let protected_routes = Router::new()
        .route(
            "/api/videos/{id}/comments",
            axum::routing::post(routes::create_comment),
        )
        .route(
            "/api/me",
            axum::routing::get(routes::me).patch(routes::update_me),
        )
        .route(
            "/api/auth/change-password",
            axum::routing::post(routes::change_password),
        )
        .route(
            "/api/reports",
            axum::routing::post(routes::create_report),
        )
        .route(
            "/api/history",
            axum::routing::get(routes::history)
                .post(routes::record_history)
                .delete(routes::clear_history),
        )
        .route(
            "/api/subscribe",
            axum::routing::post(routes::subscribe)
                .delete(routes::unsubscribe),
        )
        .route(
            "/api/videos/{id}/like",
            axum::routing::post(routes::like_video),
        )
        .route(
            "/api/videos/{id}/like",
            axum::routing::delete(routes::unlike_video),
        )
        .route(
            "/api/videos/{id}/dislike",
            axum::routing::post(routes::dislike_video),
        )
        .route(
            "/api/videos/{id}/dislike",
            axum::routing::delete(routes::undislike_video),
        )
        .route(
            "/api/videos/{id}/view",
            axum::routing::post(routes::record_view),
        )
        .route(
            "/api/watch-later",
            axum::routing::get(routes::list_watch_later).post(routes::add_watch_later),
        )
        .route(
            "/api/watch-later/{videoId}",
            axum::routing::delete(routes::remove_watch_later),
        )
        .route(
            "/api/channels/{id}/subscribe",
            axum::routing::post(routes::subscribe_to_channel),
        )
        .route(
            "/api/channels/{id}/subscribe",
            axum::routing::delete(routes::unsubscribe_from_channel),
        )
        .route(
            "/api/subscriptions",
            axum::routing::get(routes::list_subscriptions),
        )
        .route(
            "/api/comments/{id}",
            axum::routing::delete(routes::delete_comment)
                .patch(routes::edit_comment),
        )
        .route(
            "/api/comments/{id}/like",
            axum::routing::post(routes::toggle_comment_like),
        )
        .route(
            "/api/comments/{id}/pin",
            axum::routing::post(routes::pin_comment)
                .delete(routes::unpin_comment),
        )
        .route(
            "/api/videos/{id}",
            axum::routing::patch(routes::update_video)
                .delete(routes::delete_video),
        )
        .route(
            "/api/notifications",
            axum::routing::get(routes::list_notifications),
        )
        .route(
            "/api/notifications/read-all",
            axum::routing::post(routes::mark_all_notifications_read),
        )
        .route("/api/videos/{id}/subtitles", axum::routing::post(routes::register_subtitle))
        .route("/api/videos/{id}/subtitles/{language}", axum::routing::delete(routes::delete_subtitle))
         .route(
             "/api/notifications/{id}/read",
             axum::routing::post(routes::mark_notification_read),
         )
         .route(
             "/api/playlists",
             axum::routing::get(routes::list_playlists)
                 .post(routes::create_playlist),
         )
         .route(
             "/api/playlists/{id}",
             axum::routing::patch(routes::update_playlist)
                 .delete(routes::delete_playlist),
         )
         .route(
             "/api/playlists/{id}/items",
             axum::routing::post(routes::add_playlist_item),
         )
         .route(
             "/api/playlists/{id}/items/{videoId}",
             axum::routing::delete(routes::remove_playlist_item),
         )
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::auth_middleware,
        ))
        .with_state(state.clone());

    // /api/videos: GET public, POST protected.
    // Axum 0.8 requires all methods on a path in one route, so POST
    // uses a wrapper that checks auth inline.
    let videos_routes = Router::new()
        .route(
            "/api/videos",
            axum::routing::get(routes::list_videos)
                .post(routes::create_video_with_auth),
        )
        .with_state(state.clone());
    // /api/shorts: public Shorts feed.
    let shorts_routes = Router::new()
        .route(
            "/api/shorts",
            axum::routing::get(routes::list_shorts),
        )
        .with_state(state.clone());
    // /api/videos/{id}: GET is public but visibility-gated (private videos
    // are owner-only), so it runs under optional auth; rate-limited like the
    // other public reads.
    let video_detail_routes = Router::new()
        .route(
            "/api/videos/{id}",
            axum::routing::get(routes::get_video),
        )
        .route(
            "/api/videos/{id}/subtitles",
            axum::routing::get(routes::list_subtitles),
        )
        .layer(axum::middleware::from_fn(middleware::rate_limit))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::optional_auth_middleware,
        ))
        .with_state(state.clone());

    // /api/channels/{id}: GET public (optional auth via lenient middleware),
    // PATCH owner-only (inline auth check in the handler).
    let channel_routes = Router::new()
        .route(
            "/api/channels/{id}",
            axum::routing::get(routes::get_channel)
                .patch(routes::update_channel_with_auth),
        )
        .route(
            "/api/channels/{id}/videos",
            axum::routing::get(routes::list_channel_videos),
        )
        .route(
            "/api/channels/{id}/analytics",
            axum::routing::get(routes::channel_analytics),
        )
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::optional_auth_middleware,
        ))
        .with_state(state.clone());

    // /api/playlists/{id}: GET public when the playlist is public, or for the
    // owner. Uses optional auth so anonymous users can open public playlists.
    let playlist_detail_routes = Router::new()
        .route(
            "/api/playlists/{id}",
            axum::routing::get(routes::get_playlist),
        )
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::optional_auth_middleware,
        ))
        .with_state(state.clone());

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .merge(videos_routes)
        .merge(shorts_routes)
        .merge(video_detail_routes)
        .merge(channel_routes)
        .merge(home_feed_routes)
        .merge(playlist_detail_routes)
        .layer(cors)
        .layer(TraceLayer::new_for_http())

}

/// Run the server. Blocks until shutdown.
pub async fn run(config: Config) -> anyhow::Result<()> {
    let state = AppState::new(config).await?;
    let router = build_router(state.clone());
    let listener = tokio::net::TcpListener::bind(state.config.listen.clone()).await?;
    tracing::info!(listen = %state.config.listen, "API server starting");
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutdown signal received");
}

