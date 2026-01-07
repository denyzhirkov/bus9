use axum::{
    extract::{Path, Query, State, WebSocketUpgrade, ws::WebSocket},
    http::{header, StatusCode, Uri},
    response::{IntoResponse},
    routing::{get, post},
    Json, Router,
};
use rust_embed::RustEmbed;
use std::{net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use parking_lot::RwLock;

mod engine;
use engine::{Engine, ExpiredEntry, Message, QueueStatus, TopicStatus};

#[derive(RustEmbed)]
#[folder = "front/dist"]
struct Assets;

struct AppState {
    engine: Arc<Engine>,
    metrics: MetricsStore,
}

struct MetricsStore {
    requests: RwLock<HashMap<String, u64>>,
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

    let engine = Arc::new(Engine::new());
    let sweep_engine = Arc::clone(&engine);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        loop {
            interval.tick().await;
            sweep_engine.sweep_expired();
        }
    });
    let metrics = MetricsStore::new();
    let state = Arc::new(AppState { engine, metrics });

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

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
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

async fn health_handler(State(state): State<Arc<AppState>>) -> &'static str {
    state.metrics.increment(REQ_HEALTH);
    "OK"
}

#[derive(Deserialize)]
struct PublishParams {
    topic: String,
    ttl_seconds: Option<u64>,
}

#[derive(Deserialize)]
struct QueueParams {
    ttl_seconds: Option<u64>,
}

#[derive(Deserialize)]
struct PayloadRequest {
    payload: String,
    ttl_seconds: Option<u64>,
}

async fn publish_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PublishParams>,
    body: String,
) -> impl IntoResponse {
    state.metrics.increment(REQ_PUB);
    let (payload, body_ttl) = parse_payload(body);
    let msg = Message::new(payload);
    let count = state
        .engine
        .publish(&params.topic, msg.clone(), body_ttl.or(params.ttl_seconds));
    Json(serde_json::json!({ "count": count, "id": msg.id }))
}

async fn subscribe_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<PublishParams>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    state.metrics.increment(REQ_SUB);
    ws.on_upgrade(move |socket| handle_socket(socket, state, params.topic, params.ttl_seconds))
}

async fn handle_socket(
    mut socket: WebSocket,
    state: Arc<AppState>,
    topic: String,
    ttl_seconds: Option<u64>,
) {
    let mut rx = state.engine.subscribe(&topic, ttl_seconds);
    while let Ok(msg) = rx.recv().await {
        let text = serde_json::to_string(&msg).unwrap();
        if socket.send(axum::extract::ws::Message::Text(text.into())).await.is_err() {
            break;
        }
    }
}

async fn push_queue_handler(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Query(params): Query<QueueParams>,
    body: String,
) -> impl IntoResponse {
    state.metrics.increment(REQ_QUEUE_PUSH);
    let (payload, body_ttl) = parse_payload(body);
    let msg = Message::new(payload);
    state
        .engine
        .push_queue(&name, msg.clone(), body_ttl.or(params.ttl_seconds));
    Json(serde_json::json!({ "status": "ok", "id": msg.id }))
}

async fn pop_queue_handler(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
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
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));
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

fn parse_payload(body: String) -> (String, Option<u64>) {
    match serde_json::from_str::<PayloadRequest>(&body) {
        Ok(request) => (request.payload, request.ttl_seconds),
        Err(_) => (body, None),
    }
}
