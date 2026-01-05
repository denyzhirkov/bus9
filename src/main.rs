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
use serde::Deserialize;

mod engine;
use engine::{Engine, Message};

#[derive(RustEmbed)]
#[folder = "front/dist"]
struct Assets;

struct AppState {
    engine: Arc<Engine>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let engine = Arc::new(Engine::new());
    let state = Arc::new(AppState { engine });

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/api/pub", post(publish_handler))
        .route("/api/sub", get(subscribe_handler))
        .route("/api/queue/:name", post(push_queue_handler).get(pop_queue_handler))
        .route("/api/stats", get(stats_handler))
        .fallback(static_handler)
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("Listening on http://{}", addr);
    
    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_handler() -> &'static str {
    "OK"
}

#[derive(Deserialize)]
struct PublishParams {
    topic: String,
}

async fn publish_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PublishParams>,
    body: String,
) -> impl IntoResponse {
    let msg = Message::new(body);
    let count = state.engine.publish(&params.topic, msg.clone());
    Json(serde_json::json!({ "count": count, "id": msg.id }))
}

async fn subscribe_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<PublishParams>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state, params.topic))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>, topic: String) {
    let mut rx = state.engine.subscribe(&topic);
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
    body: String,
) -> impl IntoResponse {
    let msg = Message::new(body);
    state.engine.push_queue(&name, msg.clone());
    Json(serde_json::json!({ "status": "ok", "id": msg.id }))
}

async fn pop_queue_handler(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    if let Some(msg) = state.engine.pop_queue(&name) {
        Json(msg).into_response()
    } else {
        (StatusCode::NO_CONTENT, "Queue empty").into_response()
    }
}

async fn stats_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
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
