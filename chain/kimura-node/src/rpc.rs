//! HTTP RPC server for querying node state
//!
//! Provides REST API for integration testing:
//! - GET /health           -> Node status
//! - GET /height           -> Current chain height  
//! - GET /block/:height    -> Get specific block
//! - GET /latest           -> Get latest block
//! - POST /message         -> Submit message

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info};

use crate::error::NodeError;
use kimura_storage::RocksDB;

/// RPC server handle
pub struct RpcServer {
    addr: SocketAddr,
    shutdown_tx: tokio::sync::oneshot::Sender<()>,
}

impl RpcServer {
    /// Start RPC server with auto-selected port
    /// Returns server handle and the actual port bound
    pub async fn start(db: Arc<RocksDB>) -> Result<(Self, u16), NodeError> {
        // Bind to port 0 to auto-select
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| NodeError::network_init(format!("Failed to bind RPC: {}", e)))?;

        let addr = listener
            .local_addr()
            .map_err(|e| NodeError::network_init(format!("Failed to get addr: {}", e)))?;

        info!("RPC server starting on {}", addr);

        // Create stores from database (wrapped in Arc for Clone impl)
        let block_store = std::sync::Arc::new(kimura_storage::BlockStore::new(db.clone()));
        let message_store = std::sync::Arc::new(kimura_storage::MessageStore::new(db.clone()));
        let metadata_store = std::sync::Arc::new(kimura_storage::MetadataStore::new(db));

        let app = Router::new()
            .route("/health", get(health_check))
            .route("/height", get(get_height))
            .route("/block/:height", get(get_block))
            .route("/latest", get(get_latest))
            .route("/message", post(submit_message))
            .with_state(RpcState {
                block_store,
                message_store,
                metadata_store,
            });

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

        let server = RpcServer { addr, shutdown_tx };

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

/// RPC state - only contains storage (Send + Sync), not network
/// Stores are wrapped in Arc since they don't implement Clone directly
struct RpcState {
    block_store: std::sync::Arc<kimura_storage::BlockStore>,
    message_store: std::sync::Arc<kimura_storage::MessageStore>,
    metadata_store: std::sync::Arc<kimura_storage::MetadataStore>,
}

impl Clone for RpcState {
    fn clone(&self) -> Self {
        Self {
            block_store: std::sync::Arc::clone(&self.block_store),
            message_store: std::sync::Arc::clone(&self.message_store),
            metadata_store: std::sync::Arc::clone(&self.metadata_store),
        }
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
async fn health_check(State(state): State<RpcState>) -> Result<Json<HealthResponse>, StatusCode> {
    let height = state
        .metadata_store
        .get_last_height()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .unwrap_or(0);

    Ok(Json(HealthResponse {
        status: "ok".to_string(),
        height,
    }))
}

async fn get_height(State(state): State<RpcState>) -> Result<Json<HeightResponse>, StatusCode> {
    let height = state
        .metadata_store
        .get_last_height()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .unwrap_or(0);

    Ok(Json(HeightResponse { height }))
}

async fn get_block(
    Path(height): Path<u64>,
    State(state): State<RpcState>,
) -> Result<Json<BlockResponse>, StatusCode> {
    let block: kimura_blockchain::Block = state
        .block_store
        .get_block(height)
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

async fn get_latest(State(state): State<RpcState>) -> Result<Json<BlockResponse>, StatusCode> {
    let height = state
        .metadata_store
        .get_last_height()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .unwrap_or(0);

    let block: kimura_blockchain::Block = state
        .block_store
        .get_block(height)
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
    State(state): State<RpcState>,
    Json(req): Json<SubmitMessageRequest>,
) -> Result<Json<SubmitMessageResponse>, StatusCode> {
    let timestamp = current_unix_time();
    let nonce = generate_nonce();

    let message = kimura_blockchain::Message::new(req.sender, req.content, timestamp, nonce);
    let message_id = message.id;

    state
        .message_store
        .put_message(&message_id, &message)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(SubmitMessageResponse {
        message_id: hex::encode(message_id),
    }))
}

/// Get current Unix timestamp
fn current_unix_time() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Generate a random nonce for message uniqueness
fn generate_nonce() -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::Instant;

    let mut hasher = DefaultHasher::new();
    Instant::now().hash(&mut hasher);
    hasher.finish()
}
