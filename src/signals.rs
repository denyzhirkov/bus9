use tracing::error;

pub async fn shutdown_signal() {
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
