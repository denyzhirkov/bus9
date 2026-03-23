use axum::{
    extract::{Query, State, WebSocketUpgrade, ws::WebSocket},
    response::IntoResponse,
};
use std::sync::Arc;
use tokio::sync::broadcast::error::RecvError;
use tracing::warn;

use crate::auth::check_auth;
use crate::engine::Message;
use crate::handlers::http::PublishParams;
use crate::state::AppState;

pub const REQ_SUB: &str = "GET /api/sub";
pub const REQ_STATS_WS: &str = "GET /api/ws/stats";

pub async fn subscribe_handler(
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
        let disconnect_on_lag = state.config.backpressure == "disconnect";
        let mut task_handles = Vec::new();
        for (topic_name, mut receiver, _) in subscriptions {
            let tx_clone = tx.clone();
            let handle = tokio::spawn(async move {
                loop {
                    match receiver.recv().await {
                        Ok(msg) => {
                            if tx_clone.send((topic_name.clone(), msg)).is_err() {
                                break;
                            }
                        }
                        Err(RecvError::Lagged(n)) => {
                            warn!("Pattern subscriber lagged on '{}', skipped {} messages", topic_name, n);
                            if disconnect_on_lag {
                                break;
                            }
                            // "drop" strategy: continue receiving
                        }
                        Err(RecvError::Closed) => break,
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
        let disconnect_on_lag = state.config.backpressure == "disconnect";
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    if send_message(&mut socket, &msg).await.is_err() {
                        break;
                    }
                }
                Err(RecvError::Lagged(n)) => {
                    warn!("WebSocket subscriber lagged on topic '{}', skipped {} messages", topic, n);
                    if disconnect_on_lag {
                        let _ = socket.send(axum::extract::ws::Message::Text(
                            format!(r#"{{"warning":"lagged","skipped":{}}}"#, n).into()
                        )).await;
                        break;
                    }
                    // "drop" strategy: notify client and continue
                    if socket.send(axum::extract::ws::Message::Text(
                        format!(r#"{{"warning":"lagged","skipped":{}}}"#, n).into()
                    )).await.is_err() {
                        break;
                    }
                }
                Err(RecvError::Closed) => break,
            }
        }
    }
}

pub async fn stats_ws_handler(
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
