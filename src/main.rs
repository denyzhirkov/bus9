use axum::{
    extract::{Path, Query, State, WebSocketUpgrade, ws::WebSocket},
    http::{header, StatusCode, Uri},
    response::{IntoResponse, sse::{Event, KeepAlive, Sse}},
    routing::{get, post},
    Json, Router,
};
use futures_util::stream::{self, Stream, StreamExt};
use std::convert::Infallible;
use std::pin::Pin;
use std::task::{Context, Poll};
use clap::Parser;
use rust_embed::RustEmbed;
use std::{net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use parking_lot::RwLock;
use tracing::{error, warn};

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

    /// Maximum payload size in bytes (0 = unlimited)
    #[arg(long, env = "BUS9_MAX_PAYLOAD_SIZE", default_value_t = 10 * 1024 * 1024)]
    max_payload_size: usize,

    /// Maximum topic/queue name length in characters (0 = unlimited)
    #[arg(long, env = "BUS9_MAX_NAME_LENGTH", default_value_t = 256)]
    max_name_length: usize,

    /// Maximum number of topics (0 = unlimited)
    #[arg(long, env = "BUS9_MAX_TOPICS", default_value_t = 0)]
    max_topics: usize,

    /// Maximum number of queues (0 = unlimited)
    #[arg(long, env = "BUS9_MAX_QUEUES", default_value_t = 0)]
    max_queues: usize,

    /// Maximum queue depth (0 = unlimited)
    #[arg(long, env = "BUS9_MAX_QUEUE_DEPTH", default_value_t = 0)]
    max_queue_depth: usize,
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
        max_payload_size: args.max_payload_size,
        max_name_length: args.max_name_length,
        max_topics: args.max_topics,
        max_queues: args.max_queues,
        max_queue_depth: args.max_queue_depth,
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
        .route("/api/stream", get(stream_handler))
        .route("/api/queue/:name", post(push_queue_handler).get(pop_queue_handler))
        .route("/api/stats", get(stats_handler))
        .route("/api/ws/stats", get(stats_ws_handler))
        .route("/api/metrics", get(metrics_handler))
        .fallback(static_handler)
        .with_state(state);

    let addr: SocketAddr = match format!("{}:{}", args.host, args.port).parse() {
        Ok(addr) => addr,
        Err(e) => {
            error!("Invalid address format '{}:{}': {}", args.host, args.port, e);
            eprintln!("Error: Invalid address format '{}:{}': {}", args.host, args.port, e);
            std::process::exit(1);
        }
    };
    println!("Listening on http://{}", addr);
    
    let listener = match TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            error!("Could not bind to address {}: {}", addr, e);
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
        match tokio::signal::ctrl_c().await {
            Ok(_) => {}
            Err(e) => {
                error!("Failed to install Ctrl+C handler: {}", e);
                // Continue anyway, server will run without graceful shutdown
            }
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(e) => {
                error!("Failed to install terminate signal handler: {}", e);
                // Wait forever to prevent shutdown
                std::future::pending::<()>().await;
            }
        }
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
const REQ_STREAM: &str = "GET /api/stream";
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
    
    match state.engine.publish(&params.topic, msg.clone(), body_ttl.or(params.ttl_seconds)) {
        Ok(count) => Json(serde_json::json!({ "count": count, "id": msg.id })).into_response(),
        Err(e) => {
            warn!("Publish failed for topic '{}': {}", params.topic, e);
            (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))).into_response()
        }
    }
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

/// Server-Sent Events (SSE) streaming subscription endpoint
/// Alternative to WebSocket for one-way streaming (server -> client)
/// Usage: GET /api/stream?topic=sensor.temp&token=xxx
async fn stream_handler(
    Query(params): Query<PublishParams>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // Check authentication
    if let Err(status) = check_auth(&state, params.token.as_ref()) {
        return (status, "Unauthorized").into_response();
    }
    
    state.metrics.increment(REQ_STREAM);
    
    // Create SSE stream
    let stream = create_sse_stream(state, params.topic, params.ttl_seconds);
    
    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("keep-alive-text"),
    )
}

fn create_sse_stream(
    state: Arc<AppState>,
    topic: String,
    ttl_seconds: Option<u64>,
) -> impl Stream<Item = Result<Event, Infallible>> + Send + 'static {
    // Check if this is a pattern subscription
    if topic.contains('*') {
        // Pattern-based subscription
        let subscriptions = match state.engine.subscribe_pattern(&topic, ttl_seconds) {
            Ok(subs) => subs,
            Err(_) => {
                // Return error event stream
                return Box::pin(stream::once(async move {
                    Ok(Event::default().data(format!(
                        r#"{{"error": "Pattern subscription failed for '{}'"}}"#,
                        topic
                    )))
                })) as Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>>;
            }
        };
        
        // Send replay history first
        let history_events: Vec<_> = subscriptions
            .iter()
            .flat_map(|(_topic_name, _, history)| {
                history.iter().map(|msg| {
                    serde_json::to_string(msg)
                        .map(|data| Event::default().data(data))
                        .unwrap_or_else(|_| Event::default().data(r#"{"error": "serialization failed"}"#))
                })
            })
            .collect();
        
        // Create merged stream from all subscriptions
        use tokio::sync::mpsc;
        let (tx, mut rx) = mpsc::unbounded_channel();
        
        // Spawn tasks to forward messages
        let mut task_handles = Vec::new();
        for (topic_name, mut receiver, _) in subscriptions {
            let tx_clone = tx.clone();
            let handle = tokio::spawn(async move {
                while let Ok(msg) = receiver.recv().await {
                    if tx_clone.send((topic_name.clone(), msg)).is_err() {
                        break;
                    }
                }
            });
            task_handles.push(handle);
        }
        drop(tx);
        
        // Combine history and live stream
        let history_stream = stream::iter(history_events.into_iter().map(Ok));
        
        // Create live stream from channel
        struct ChannelStream {
            rx: tokio::sync::mpsc::UnboundedReceiver<(String, Message)>,
            task_handles: Vec<tokio::task::JoinHandle<()>>,
        }
        
        impl Stream for ChannelStream {
            type Item = Result<Event, Infallible>;
            
            fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
                match self.rx.poll_recv(cx) {
                    Poll::Ready(Some((_topic_name, msg))) => {
                        let event = match serde_json::to_string(&msg) {
                            Ok(data) => Ok(Event::default().data(data)),
                            Err(e) => {
                                warn!("Failed to serialize message in SSE stream: {}", e);
                                Ok(Event::default().data(r#"{"error": "serialization failed"}"#))
                            }
                        };
                        Poll::Ready(Some(event))
                    }
                    Poll::Ready(None) => {
                        // Cleanup tasks when stream ends
                        for handle in &self.task_handles {
                            handle.abort();
                        }
                        Poll::Ready(None)
                    }
                    Poll::Pending => Poll::Pending,
                }
            }
        }
        
        let live_stream = ChannelStream { rx, task_handles };
        Box::pin(stream::select(history_stream, live_stream)) as Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>>
    } else {
        // Regular single topic subscription
        let (mut rx, history) = match state.engine.subscribe(&topic, ttl_seconds) {
            Ok(result) => result,
            Err(_) => {
                return Box::pin(stream::once(async move {
                    Ok(Event::default().data(format!(
                        r#"{{"error": "Subscription failed for topic '{}'"}}"#,
                        topic
                    )))
                })) as Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>>;
            }
        };
        
        // Send replay history first
        let history_events: Vec<_> = history
            .into_iter()
            .map(|msg| {
                serde_json::to_string(&msg)
                    .map(|data| Event::default().data(data))
                    .unwrap_or_else(|_| Event::default().data(r#"{"error": "serialization failed"}"#))
            })
            .collect();
        
        let history_stream = stream::iter(history_events.into_iter().map(Ok));
        
        // Then live messages - convert broadcast receiver to stream via channel
        let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(async move {
            while let Ok(msg) = rx.recv().await {
                let event = match serde_json::to_string(&msg) {
                    Ok(data) => Ok(Event::default().data(data)),
                    Err(e) => {
                        warn!("Failed to serialize message in SSE stream: {}", e);
                        Ok(Event::default().data(r#"{"error": "serialization failed"}"#))
                    }
                };
                if event_tx.send(event).is_err() {
                    break;
                }
            }
        });
        
        struct ChannelStream {
            rx: tokio::sync::mpsc::UnboundedReceiver<Result<Event, Infallible>>,
        }
        
        impl Stream for ChannelStream {
            type Item = Result<Event, Infallible>;
            
            fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
                self.rx.poll_recv(cx)
            }
        }
        
        let live_stream = ChannelStream { rx: event_rx };
        Box::pin(stream::select(history_stream, live_stream)) as Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>>
    }
}

async fn handle_socket(
    mut socket: WebSocket,
    state: Arc<AppState>,
    topic: String,
    ttl_seconds: Option<u64>,
) {
    // Helper function to safely serialize and send message
    async fn send_message(socket: &mut WebSocket, msg: &Message) -> Result<(), ()> {
        match serde_json::to_string(msg) {
            Ok(text) => {
                socket.send(axum::extract::ws::Message::Text(text.into())).await.map_err(|_| ())
            }
            Err(e) => {
                warn!("Failed to serialize message: {}", e);
                Err(())
            }
        }
    }

    // Check if this is a pattern subscription
    if topic.contains('*') {
        // Pattern-based subscription
        let subscriptions = match state.engine.subscribe_pattern(&topic, ttl_seconds) {
            Ok(subs) => subs,
            Err(e) => {
                warn!("Pattern subscription failed for '{}': {}", topic, e);
                // Try to send error message to client
                let _ = socket.send(axum::extract::ws::Message::Text(
                    format!(r#"{{"error": "Subscription failed: {}"}}"#, e).into()
                )).await;
                return;
            }
        };
        
        // Send replay history for all matching topics
        for (_topic_name, _, history) in &subscriptions {
            for msg in history {
                if send_message(&mut socket, msg).await.is_err() {
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
        // Use a bounded channel or limit the number of tasks to prevent resource exhaustion
        let mut task_handles = Vec::new();
        for (topic_name, mut receiver, _) in subscriptions {
            let tx_clone = tx.clone();
            let handle = tokio::spawn(async move {
                while let Ok(msg) = receiver.recv().await {
                    if tx_clone.send((topic_name.clone(), msg)).is_err() {
                        // Receiver dropped, exit
                        break;
                    }
                }
            });
            task_handles.push(handle);
        }
        drop(tx);
        
        // Forward merged messages to WebSocket
        while let Some((_topic_name, msg)) = rx.recv().await {
            if send_message(&mut socket, &msg).await.is_err() {
                break;
            }
        }
        
        // Cancel all tasks when connection closes
        for handle in task_handles {
            handle.abort();
        }
    } else {
        // Regular single topic subscription
        let (mut rx, history) = match state.engine.subscribe(&topic, ttl_seconds) {
            Ok(result) => result,
            Err(e) => {
                warn!("Subscription failed for topic '{}': {}", topic, e);
                // Try to send error message to client
                let _ = socket.send(axum::extract::ws::Message::Text(
                    format!(r#"{{"error": "Subscription failed: {}"}}"#, e).into()
                )).await;
                return;
            }
        };
        
        // Send replay history first
        for msg in history {
            if send_message(&mut socket, &msg).await.is_err() {
                return;
            }
        }
        
        // Then send live messages
        while let Ok(msg) = rx.recv().await {
            if send_message(&mut socket, &msg).await.is_err() {
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
    
    match state.engine.push_queue(&name, msg.clone(), body_ttl.or(params.ttl_seconds)) {
        Ok(()) => Json(serde_json::json!({ "status": "ok", "id": msg.id })).into_response(),
        Err(e) => {
            warn!("Queue push failed for queue '{}': {}", name, e);
            (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))).into_response()
        }
    }
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
        
        match serde_json::to_string(&stats) {
            Ok(text) => {
                if socket.send(axum::extract::ws::Message::Text(text.into())).await.is_err() {
                    break;
                }
            }
            Err(e) => {
                warn!("Failed to serialize stats: {}", e);
                // Continue trying, don't break the connection
            }
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
