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
        .route(
            "/api/uploads/presign",
            axum::routing::post(routes::presign_upload),
        )
        .route("/api/videos/{id}", axum::routing::get(routes::get_video))
        .route(
            "/api/videos/{id}/comments",
            axum::routing::get(routes::list_comments),
        )
        .route(
            "/api/videos/{id}/comments",
            axum::routing::post(routes::create_comment),
        )
        .route("/api/search", axum::routing::get(routes::search))
        .layer(axum::middleware::from_fn(middleware::rate_limit))
        .with_state(state.clone());

    let protected_routes = Router::new()
        .route("/api/me", axum::routing::get(routes::me))
        .route("/api/history", axum::routing::get(routes::history))
        .route("/api/history", axum::routing::post(routes::record_history))
        .route("/api/subscribe", axum::routing::post(routes::subscribe))
        .route(
            "/api/subscribe",
            axum::routing::delete(routes::unsubscribe),
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
            "/api/videos/{id}/view",
            axum::routing::post(routes::record_view),
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

    // /api/channels/{id}: GET public (optional auth via lenient middleware),
    // PATCH owner-only (inline auth check in the handler).
    let channel_routes = Router::new()
        .route(
            "/api/channels/{id}",
            axum::routing::get(routes::get_channel)
                .patch(routes::update_channel_with_auth),
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
        .merge(channel_routes)
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

