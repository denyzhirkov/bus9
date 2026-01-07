use axum::{
    extract::{Path, Query, State, WebSocketUpgrade, ws::WebSocket},
    http::{header, StatusCode, Uri},
    response::{IntoResponse},
    routing::{get, post},
    Json, Router,
};
use clap::Parser;
use rust_embed::RustEmbed;
use std::{net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use parking_lot::RwLock;

mod engine;
use engine::{Engine, EngineConfig, ExpiredEntry, Message, MessagePriority, QueueStatus, TopicStatus};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct AppArgs {
    /// Port to listen on
    #[arg(long, env = "BUS9_PORT", default_value_t = 8080)]
    port: u16,

    /// Host to bind to
    #[arg(long, env = "BUS9_HOST", default_value = "0.0.0.0")]
    host: String,

    /// Initial capacity for broadcast channels
    #[arg(long, env = "BUS9_TOPIC_CAPACITY", default_value_t = 1024)]
    topic_capacity: usize,

    /// Maximum number of expired events to keep in history
    #[arg(long, env = "BUS9_MAX_EXPIRED", default_value_t = 50)]
    max_expired: usize,

    /// Cleanup interval in milliseconds
    #[arg(long, env = "BUS9_SWEEP_INTERVAL_MS", default_value_t = 1000)]
    sweep_interval_ms: u64,

    /// Stats update interval in milliseconds
    #[arg(long, env = "BUS9_STATS_INTERVAL_MS", default_value_t = 500)]
    stats_interval_ms: u64,

    /// Number of messages to replay to new subscribers (0 = disabled)
    #[arg(long, env = "BUS9_TOPIC_REPLAY_COUNT", default_value_t = 0)]
    topic_replay_count: usize,

    /// Inactivity timeout in seconds for auto-cleanup (0 = disabled)
    #[arg(long, env = "BUS9_INACTIVITY_TIMEOUT_SECONDS")]
    inactivity_timeout_seconds: Option<u64>,

    /// Topic exclusion patterns (comma-separated). Topics matching these patterns will be excluded from wildcard subscriptions.
    /// Example: --topic-exclude-patterns "sensor.secret,logs.admin.**"
    #[arg(long, env = "BUS9_TOPIC_EXCLUDE_PATTERNS", value_delimiter = ',')]
    topic_exclude_patterns: Vec<String>,

    /// Global authentication token. If set, all API requests (pub, sub, queue) must include this token.
    /// Token must be provided as query parameter: ?token=<token>
    #[arg(long, env = "BUS9_AUTH_TOKEN")]
    auth_token: Option<String>,
}

#[derive(RustEmbed)]
#[folder = "front/dist"]
struct Assets;

struct AppState {
    engine: Arc<Engine>,
    metrics: MetricsStore,
    config: AppConfig,
    auth_token: Option<String>,
}

struct MetricsStore {
    requests: RwLock<HashMap<String, u64>>,
}

struct AppConfig {
    stats_interval_ms: u64,
}

impl MetricsStore {
    fn new() -> Self {
        Self {
            requests: RwLock::new(HashMap::new()),
        }
    }

    fn increment(&self, key: &str) {
        let mut requests = self.requests.write();
        *requests.entry(key.to_string()).or_insert(0) += 1;
    }

    fn snapshot(&self) -> HashMap<String, u64> {
        self.requests.read().clone()
    }
}

#[derive(Serialize)]
struct MetricsResponse {
    requests: HashMap<String, u64>,
    topics: TopicMetrics,
    queues: QueueMetrics,
    expired: Vec<ExpiredEntry>,
}

#[derive(Serialize)]
struct TopicMetrics {
    active: usize,
    entries: HashMap<String, TopicStatus>,
}

#[derive(Serialize)]
struct QueueMetrics {
    active: usize,
    entries: HashMap<String, QueueStatus>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let args = AppArgs::parse();

    let engine_config = EngineConfig {
        broadcast_channel_capacity: args.topic_capacity,
        max_expired_events: args.max_expired,
        topic_replay_count: args.topic_replay_count,
        inactivity_timeout_seconds: args.inactivity_timeout_seconds,
        topic_exclude_patterns: args.topic_exclude_patterns,
    };

    let engine = Arc::new(Engine::new(engine_config));
    let sweep_engine = Arc::clone(&engine);
    let sweep_interval_ms = args.sweep_interval_ms;

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(sweep_interval_ms));
        loop {
            interval.tick().await;
            sweep_engine.sweep_expired();
        }
    });
    let metrics = MetricsStore::new();
    let state = Arc::new(AppState {
        engine,
        metrics,
        config: AppConfig {
            stats_interval_ms: args.stats_interval_ms,
        },
        auth_token: args.auth_token,
    });

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/api/pub", post(publish_handler))
        .route("/api/sub", get(subscribe_handler))
        .route("/api/queue/:name", post(push_queue_handler).get(pop_queue_handler))
        .route("/api/stats", get(stats_handler))
        .route("/api/ws/stats", get(stats_ws_handler))
        .route("/api/metrics", get(metrics_handler))
        .fallback(static_handler)
        .with_state(state);

    let addr: SocketAddr = format!("{}:{}", args.host, args.port)
        .parse()
        .expect("Invalid address format");
    println!("Listening on http://{}", addr);
    
    let listener = match TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Error: Could not bind to address {}: {}", addr, e);
            std::process::exit(1);
        }
    };

    println!("Bus9 server started successfully");

    if let Err(e) = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await 
    {
        eprintln!("Server error: {}", e);
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("Signal received, starting graceful shutdown...");
}

const REQ_HEALTH: &str = "GET /health";
const REQ_PUB: &str = "POST /api/pub";
const REQ_SUB: &str = "GET /api/sub";
const REQ_QUEUE_PUSH: &str = "POST /api/queue";
const REQ_QUEUE_POP: &str = "GET /api/queue";
const REQ_STATS: &str = "GET /api/stats";
const REQ_STATS_WS: &str = "GET /api/ws/stats";
const REQ_METRICS: &str = "GET /api/metrics";

/// Check if request is authorized
fn check_auth(state: &AppState, query_token: Option<&String>) -> Result<(), StatusCode> {
    // If no auth token is configured, allow all requests
    let required_token = match &state.auth_token {
        Some(token) => token,
        None => return Ok(()),
    };
    
    match query_token {
        Some(token) if token == required_token => Ok(()),
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}

async fn health_handler(State(state): State<Arc<AppState>>) -> &'static str {
    state.metrics.increment(REQ_HEALTH);
    "OK"
}

#[derive(Deserialize)]
struct PublishParams {
    topic: String,
    ttl_seconds: Option<u64>,
    token: Option<String>,
}

#[derive(Deserialize)]
struct QueueParams {
    ttl_seconds: Option<u64>,
    priority: Option<String>,
    token: Option<String>,
}

#[derive(Deserialize)]
struct PayloadRequest {
    payload: String,
    ttl_seconds: Option<u64>,
    priority: Option<String>,
}

async fn publish_handler(
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
    let count = state
        .engine
        .publish(&params.topic, msg.clone(), body_ttl.or(params.ttl_seconds));
    Json(serde_json::json!({ "count": count, "id": msg.id })).into_response()
}

async fn subscribe_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<PublishParams>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // Check authentication
    if let Err(status) = check_auth(&state, params.token.as_ref()) {
        return (status, "Unauthorized").into_response();
    }
    
    state.metrics.increment(REQ_SUB);
    ws.on_upgrade(move |socket| handle_socket(socket, state, params.topic, params.ttl_seconds))
        .into_response()
}

async fn handle_socket(
    mut socket: WebSocket,
    state: Arc<AppState>,
    topic: String,
    ttl_seconds: Option<u64>,
) {
    // Check if this is a pattern subscription
    if topic.contains('*') {
        // Pattern-based subscription
        let subscriptions = state.engine.subscribe_pattern(&topic, ttl_seconds);
        
        // Send replay history for all matching topics
        for (_topic_name, _, history) in &subscriptions {
            for msg in history {
                let text = serde_json::to_string(&msg).unwrap();
                if socket.send(axum::extract::ws::Message::Text(text.into())).await.is_err() {
                    return;
                }
            }
        }
        
        // Merge messages from all matching topics
        if subscriptions.is_empty() {
            // No matching topics yet, wait for new ones
            // We'll need to periodically check for new topics
            // For now, just close if no matches
            return;
        }
        
        // Create a merged stream from all subscriptions
        use tokio::sync::mpsc;
        let (tx, mut rx) = mpsc::unbounded_channel();
        
        // Spawn tasks to forward messages from each subscription
        for (topic_name, mut receiver, _) in subscriptions {
            let tx_clone = tx.clone();
            tokio::spawn(async move {
                while let Ok(msg) = receiver.recv().await {
                    let _ = tx_clone.send((topic_name.clone(), msg));
                }
            });
        }
        drop(tx);
        
        // Forward merged messages to WebSocket
        while let Some((_topic_name, msg)) = rx.recv().await {
            // Optionally include topic name in message
            let text = serde_json::to_string(&msg).unwrap();
            if socket.send(axum::extract::ws::Message::Text(text.into())).await.is_err() {
                break;
            }
        }
    } else {
        // Regular single topic subscription
        let (mut rx, history) = state.engine.subscribe(&topic, ttl_seconds);
        
        // Send replay history first
        for msg in history {
            let text = serde_json::to_string(&msg).unwrap();
            if socket.send(axum::extract::ws::Message::Text(text.into())).await.is_err() {
                return;
            }
        }
        
        // Then send live messages
        while let Ok(msg) = rx.recv().await {
            let text = serde_json::to_string(&msg).unwrap();
            if socket.send(axum::extract::ws::Message::Text(text.into())).await.is_err() {
                break;
            }
        }
    }
}

fn parse_priority(priority_str: Option<&String>) -> Option<MessagePriority> {
    priority_str.as_ref().and_then(|p| match p.to_lowercase().as_str() {
        "high" => Some(MessagePriority::High),
        "normal" => Some(MessagePriority::Normal),
        "low" => Some(MessagePriority::Low),
        _ => None,
    })
}

async fn push_queue_handler(
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
    
    state
        .engine
        .push_queue(&name, msg.clone(), body_ttl.or(params.ttl_seconds));
    Json(serde_json::json!({ "status": "ok", "id": msg.id })).into_response()
}

async fn pop_queue_handler(
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

async fn stats_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    state.metrics.increment(REQ_STATS);
    Json(state.engine.get_stats())
}

async fn static_handler(uri: Uri) -> impl IntoResponse {
    let mut path = uri.path().trim_start_matches('/').to_string();

    if path.is_empty() {
        path = "index.html".to_string();
    }

    match Assets::get(&path) {
        Some(content) => {
            let mime = mime_guess::from_path(&path).first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
        }
        None => {
            if path.contains('.') {
                return StatusCode::NOT_FOUND.into_response();
            }
            index_handler().await.into_response()
        }
    }
}

async fn index_handler() -> impl IntoResponse {
    match Assets::get("index.html") {
        Some(content) => {
            let mime = mime_guess::from_path("index.html").first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
        }
        None => (StatusCode::NOT_FOUND, "index.html not found").into_response(),
    }
}

async fn stats_ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    state.metrics.increment(REQ_STATS_WS);
    ws.on_upgrade(|socket| handle_stats_socket(socket, state))
}

async fn handle_stats_socket(mut socket: WebSocket, state: Arc<AppState>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(state.config.stats_interval_ms));
    loop {
        interval.tick().await;
        let stats = state.engine.get_stats();
        let text = serde_json::to_string(&stats).unwrap();
        
        if socket.send(axum::extract::ws::Message::Text(text.into())).await.is_err() {
            break;
        }
    }
}

async fn metrics_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
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

fn parse_payload(body: String) -> (String, Option<u64>, Option<String>) {
    match serde_json::from_str::<PayloadRequest>(&body) {
        Ok(request) => (request.payload, request.ttl_seconds, request.priority),
        Err(_) => (body, None, None),
    }
}
