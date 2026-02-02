use clap::Parser;
use kimura_node::{Node, NodeConfig};
use std::sync::Arc;
use tokio::sync::Notify;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let config = NodeConfig::parse();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(match config.log_level.as_str() {
            "trace" => tracing::Level::TRACE,
            "debug" => tracing::Level::DEBUG,
            "info" => tracing::Level::INFO,
            "warn" => tracing::Level::WARN,
            "error" => tracing::Level::ERROR,
            _ => tracing::Level::INFO,
        })
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .compact()
        .init();

    info!("Starting Kimura blockchain node...");
    info!("Mode: {}", if config.is_leader { "LEADER" } else { "PEER" });
    info!("Database path: {:?}", config.db_path);
    info!("Listen address: {}", config.listen_addr);

    if let Some(ref leader) = config.leader_addr {
        info!("Leader address: {}", leader);
    }

    // Validate configuration
    if let Err(e) = config.validate() {
        error!("Configuration error: {}", e);
        std::process::exit(1);
    }

    // Create shutdown notifier
    let shutdown = Arc::new(Notify::new());
    let shutdown_clone = shutdown.clone();

    // Setup Ctrl+C handler
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for Ctrl+C");
        info!("Received shutdown signal");
        shutdown_clone.notify_one();
    });

    // Create and run node
    let node = match Node::new(config) {
        Ok(node) => node,
        Err(e) => {
            error!("Failed to create node: {}", e);
            std::process::exit(1);
        }
    };

    info!("Node initialized, starting main loop...");

    // Run node in a separate task so we can handle shutdown
    let node_handle = tokio::spawn(async move {
        if let Err(e) = node.run().await {
            error!("Node error: {}", e);
            return Err(e);
        }
        Ok(())
    });

    // Wait for either shutdown signal or node completion
    tokio::select! {
        _ = shutdown.notified() => {
            info!("Shutting down gracefully...");
            // The node will finish its current iteration and return
            // In a more complex implementation, we might signal the node to stop
        }
        result = node_handle => {
            match result {
                Ok(Ok(())) => info!("Node completed successfully"),
                Ok(Err(e)) => {
                    error!("Node failed: {}", e);
                    std::process::exit(1);
                }
                Err(e) => {
                    error!("Node task panicked: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }

    info!("Kimura node stopped");
    Ok(())
}
