use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

use crate::handlers::{http, sse, static_files, websocket};
use crate::state::AppState;

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(http::health_handler))
        .route("/api/version", get(http::version_handler))
        .route("/api/pub", post(http::publish_handler))
        .route("/api/sub", get(websocket::subscribe_handler))
        .route("/api/stream", get(sse::stream_handler))
        .route("/api/queue/:name", post(http::push_queue_handler).get(http::pop_queue_handler))
        .route("/api/stats", get(http::stats_handler))
        .route("/api/ws/stats", get(websocket::stats_ws_handler))
        .route("/api/metrics", get(http::metrics_handler))
        .fallback(static_files::static_handler)
        .with_state(state)
}
