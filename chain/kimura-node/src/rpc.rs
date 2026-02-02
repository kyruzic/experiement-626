//! HTTP RPC server for querying node state
//! 
//! Provides REST API for integration testing:
//! - GET /health           -> Node status
//! - GET /height           -> Current chain height  
//! - GET /block/:height    -> Get specific block
//! - GET /latest           -> Get latest block
//! - POST /message         -> Submit message

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{info, error};

use crate::{NodeError, NodeServices};

/// RPC server handle
pub struct RpcServer {
    addr: SocketAddr,
    shutdown_tx: tokio::sync::oneshot::Sender<()>,
}

impl RpcServer {
    /// Start RPC server with auto-selected port
    /// Returns server handle and the actual port bound
    pub async fn start(
        services: Arc<NodeServices>,
    ) -> Result<(Self, u16), NodeError> {
        // Bind to port 0 to auto-select
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| NodeError::network_init(format!("Failed to bind RPC: {}", e)))?;
        
        let addr = listener.local_addr()
            .map_err(|e| NodeError::network_init(format!("Failed to get addr: {}", e)))?;
        
        info!("RPC server starting on {}", addr);

        let app = Router::new()
            .route("/health", get(health_check))
            .route("/height", get(get_height))
            .route("/block/:height", get(get_block))
            .route("/latest", get(get_latest))
            .route("/message", post(submit_message))
            .with_state(services);

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        // Spawn server in background
        tokio::spawn(async move {
            let server = axum::serve(listener, app);
            
            // Handle graceful shutdown
            let server_with_shutdown = server.with_graceful_shutdown(async {
                shutdown_rx.await.ok();
            });
            
            if let Err(e) = server_with_shutdown.await {
                error!("RPC server error: {}", e);
            }
        });

        let server = RpcServer {
            addr,
            shutdown_tx,
        };

        Ok((server, addr.port()))
    }

    /// Get the bound port
    pub fn port(&self) -> u16 {
        self.addr.port()
    }

    /// Shutdown the RPC server
    pub async fn shutdown(self) {
        let _ = self.shutdown_tx.send(());
        info!("RPC server shutting down");
    }
}

/// Request/Response types
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub height: u64,
}

#[derive(Serialize)]
pub struct HeightResponse {
    pub height: u64,
}

#[derive(Serialize, Debug, Clone)]
pub struct BlockResponse {
    pub height: u64,
    pub timestamp: u64,
    pub prev_hash: String,
    pub message_count: usize,
    pub hash: String,
}

#[derive(Deserialize)]
pub struct SubmitMessageRequest {
    pub sender: String,
    pub content: String,
}

#[derive(Serialize)]
pub struct SubmitMessageResponse {
    pub message_id: String,
}

/// Handlers
async fn health_check(
    State(services): State<Arc<NodeServices>>,
) -> Result<Json<HealthResponse>, StatusCode> {
    let height = services.get_current_height()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(HealthResponse {
        status: "ok".to_string(),
        height,
    }))
}

async fn get_height(
    State(services): State<Arc<NodeServices>>,
) -> Result<Json<HeightResponse>, StatusCode> {
    let height = services.get_current_height()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(HeightResponse { height }))
}

async fn get_block(
    Path(height): Path<u64>,
    State(services): State<Arc<NodeServices>>,
) -> Result<Json<BlockResponse>, StatusCode> {
    let block = services.get_block(height)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    
    Ok(Json(BlockResponse {
        height: block.header.height,
        timestamp: block.header.timestamp,
        prev_hash: hex::encode(&block.header.prev_hash[..8]),
        message_count: block.message_ids.len(),
        hash: hex::encode(block.hash().as_bytes()),
    }))
}

async fn get_latest(
    State(services): State<Arc<NodeServices>>,
) -> Result<Json<BlockResponse>, StatusCode> {
    let block = services.get_latest_block()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    
    Ok(Json(BlockResponse {
        height: block.header.height,
        timestamp: block.header.timestamp,
        prev_hash: hex::encode(&block.header.prev_hash[..8]),
        message_count: block.message_ids.len(),
        hash: hex::encode(block.hash().as_bytes()),
    }))
}

async fn submit_message(
    State(services): State<Arc<NodeServices>>,
    Json(req): Json<SubmitMessageRequest>,
) -> Result<Json<SubmitMessageResponse>, StatusCode> {
    let message = services.submit_message(req.sender, req.content)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(SubmitMessageResponse {
        message_id: hex::encode(message.id),
    }))
}
