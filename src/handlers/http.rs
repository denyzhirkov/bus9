use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::warn;

use crate::auth::check_auth;
use crate::engine::{ExpiredEntry, Message, QueueStatus, TopicStatus};
use crate::state::AppState;
use crate::utils::{parse_payload, parse_priority};

// Request type constants
pub const REQ_HEALTH: &str = "GET /health";
pub const REQ_PUB: &str = "POST /api/pub";
pub const REQ_QUEUE_PUSH: &str = "POST /api/queue";
pub const REQ_QUEUE_POP: &str = "GET /api/queue";
pub const REQ_STATS: &str = "GET /api/stats";
pub const REQ_METRICS: &str = "GET /api/metrics";
pub const REQ_QUEUE_ACK: &str = "POST /api/queue/:name/ack";
pub const REQ_QUEUE_NACK: &str = "POST /api/queue/:name/nack";

#[derive(Deserialize)]
pub struct PublishParams {
    pub topic: String,
    pub ttl_seconds: Option<u64>,
    pub token: Option<String>,
}

#[derive(Deserialize)]
pub struct QueueParams {
    pub ttl_seconds: Option<u64>,
    pub priority: Option<String>,
    pub token: Option<String>,
}

#[derive(Serialize)]
pub struct MetricsResponse {
    pub requests: HashMap<String, u64>,
    pub topics: TopicMetrics,
    pub queues: QueueMetrics,
    pub expired: Vec<ExpiredEntry>,
}

#[derive(Serialize)]
pub struct TopicMetrics {
    pub active: usize,
    pub entries: HashMap<String, TopicStatus>,
}

#[derive(Serialize)]
pub struct QueueMetrics {
    pub active: usize,
    pub entries: HashMap<String, QueueStatus>,
}

pub async fn health_handler(State(state): State<Arc<AppState>>) -> &'static str {
    state.metrics.increment(REQ_HEALTH);
    "OK"
}

#[derive(Serialize)]
pub struct VersionResponse {
    pub version: String,
}

pub async fn version_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(VersionResponse {
        version: state.version.clone(),
    })
}

pub async fn publish_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PublishParams>,
    body: String,
) -> impl IntoResponse {
    // Check authentication
    if let Err(status) = check_auth(&state, params.token.as_ref()) {
        return (status, "Unauthorized").into_response();
    }
    
    state.metrics.increment(REQ_PUB);
    let (payload, body_ttl, _) = parse_payload(body);
    let msg = Message::new(payload);
    
    match state.engine.publish(&params.topic, msg.clone(), body_ttl.or(params.ttl_seconds)) {
        Ok(count) => Json(serde_json::json!({ "count": count, "id": msg.id })).into_response(),
        Err(e) => {
            warn!("Publish failed for topic '{}': {}", params.topic, e);
            (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))).into_response()
        }
    }
}

pub async fn push_queue_handler(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Query(params): Query<QueueParams>,
    body: String,
) -> impl IntoResponse {
    // Check authentication
    if let Err(status) = check_auth(&state, params.token.as_ref()) {
        return (status, "Unauthorized").into_response();
    }
    
    state.metrics.increment(REQ_QUEUE_PUSH);
    let (payload, body_ttl, body_priority) = parse_payload(body);
    
    // Parse priority from query param (takes precedence) or body
    let priority = parse_priority(params.priority.as_ref())
        .or_else(|| parse_priority(body_priority.as_ref()));
    
    let msg = if let Some(prio) = priority {
        Message::with_priority(payload, Some(prio))
    } else {
        Message::new(payload)
    };
    
    match state.engine.push_queue(&name, msg.clone(), body_ttl.or(params.ttl_seconds)) {
        Ok(()) => Json(serde_json::json!({ "status": "ok", "id": msg.id })).into_response(),
        Err(e) => {
            warn!("Queue push failed for queue '{}': {}", name, e);
            (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))).into_response()
        }
    }
}

pub async fn pop_queue_handler(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Query(params): Query<QueueParams>,
) -> impl IntoResponse {
    // Check authentication
    if let Err(status) = check_auth(&state, params.token.as_ref()) {
        return (status, "Unauthorized").into_response();
    }
    
    state.metrics.increment(REQ_QUEUE_POP);
    if let Some(msg) = state.engine.pop_queue(&name) {
        Json(msg).into_response()
    } else {
        (StatusCode::NO_CONTENT, "Queue empty").into_response()
    }
}

#[derive(Deserialize)]
pub struct AckParams {
    pub id: String,
    pub token: Option<String>,
}

pub async fn ack_queue_handler(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Query(params): Query<AckParams>,
) -> impl IntoResponse {
    if let Err(status) = check_auth(&state, params.token.as_ref()) {
        return (status, "Unauthorized").into_response();
    }

    state.metrics.increment(REQ_QUEUE_ACK);
    match state.engine.ack_queue(&name, &params.id) {
        Ok(()) => Json(serde_json::json!({ "status": "acked", "id": params.id })).into_response(),
        Err(e) => {
            warn!("ACK failed for queue '{}', message '{}': {}", name, params.id, e);
            (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))).into_response()
        }
    }
}

pub async fn nack_queue_handler(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Query(params): Query<AckParams>,
) -> impl IntoResponse {
    if let Err(status) = check_auth(&state, params.token.as_ref()) {
        return (status, "Unauthorized").into_response();
    }

    state.metrics.increment(REQ_QUEUE_NACK);
    match state.engine.nack_queue(&name, &params.id) {
        Ok(()) => Json(serde_json::json!({ "status": "nacked", "id": params.id })).into_response(),
        Err(e) => {
            warn!("NACK failed for queue '{}', message '{}': {}", name, params.id, e);
            (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))).into_response()
        }
    }
}

pub async fn stats_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    state.metrics.increment(REQ_STATS);
    Json(state.engine.get_stats())
}

pub async fn metrics_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    state.metrics.increment(REQ_METRICS);
    let broker_metrics = state.engine.get_metrics();
    let response = MetricsResponse {
        requests: state.metrics.snapshot(),
        topics: TopicMetrics {
            active: broker_metrics.topics.len(),
            entries: broker_metrics.topics,
        },
        queues: QueueMetrics {
            active: broker_metrics.queues.len(),
            entries: broker_metrics.queues,
        },
        expired: broker_metrics.expired,
    };

    Json(response)
}
