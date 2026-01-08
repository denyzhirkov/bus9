mod auth;
mod config;
mod engine;
mod handlers;
mod routes;
mod signals;
mod state;
mod utils;

use clap::Parser;
use config::{AppArgs, AppConfig};
use engine::{Engine, EngineConfig};
use routes::create_router;
use signals::shutdown_signal;
use state::{AppState, MetricsStore};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::error;
use utils::read_version;

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

    // Spawn background task for periodic cleanup
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(sweep_interval_ms));
        loop {
            interval.tick().await;
            sweep_engine.sweep_expired();
        }
    });

    let metrics = MetricsStore::new();
    let version = read_version();
    let state = Arc::new(AppState {
        engine,
        metrics,
        config: AppConfig::new(args.stats_interval_ms),
        auth_token: args.auth_token,
        version: version.clone(),
    });

    let app = create_router(state);

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

    println!("Bus9 server v{} started successfully", version);

    if let Err(e) = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await 
    {
        eprintln!("Server error: {}", e);
    }
}
