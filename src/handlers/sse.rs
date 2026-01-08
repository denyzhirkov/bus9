use axum::{
    extract::{Query, State},
    response::{IntoResponse, sse::{Event, KeepAlive, Sse}},
};
use futures_util::stream::{self, Stream};
use std::convert::Infallible;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tracing::warn;

use crate::auth::check_auth;
use crate::engine::Message;
use crate::handlers::http::PublishParams;
use crate::state::AppState;

pub const REQ_STREAM: &str = "GET /api/stream";

/// Server-Sent Events (SSE) streaming subscription endpoint
/// Alternative to WebSocket for one-way streaming (server -> client)
/// Usage: GET /api/stream?topic=sensor.temp&token=xxx
pub async fn stream_handler(
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
    
    Sse::new(stream)
        .keep_alive(
            KeepAlive::new()
                .interval(std::time::Duration::from_secs(15))
                .text("keep-alive-text"),
        )
        .into_response()
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
        let (tx, rx) = mpsc::unbounded_channel();
        
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
