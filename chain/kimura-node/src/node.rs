use crate::{config::NodeConfig, error::NodeError, services::NodeServices};
use futures::stream::StreamExt;
use kimura_blockchain::{Block, BlockHeader};
use kimura_network::NetworkEvent;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::interval;
use tracing::{debug, error, info, warn};

/// Kimura blockchain node
pub struct Node {
    config: NodeConfig,
    services: NodeServices,
    mode: NodeMode,
}

/// Node operating mode
pub enum NodeMode {
    Leader(LeaderState),
    Peer(PeerState),
}

/// State for leader mode
pub struct LeaderState {
    last_height: u64,
    last_hash: [u8; 32],
}

/// State for peer mode
pub struct PeerState;

impl Node {
    /// Create a new node
    pub fn new(config: NodeConfig) -> Result<Self, NodeError> {
        info!("Creating node (leader: {})...", config.is_leader);

        // Validate config
        config.validate().map_err(|e| NodeError::Config(e.to_string()))?;

        // Initialize services
        let mut services = NodeServices::new(&config)?;

        // Start network listener
        services.start_listening(&config.listen_addr)?;

        // Ensure genesis block exists
        services.ensure_genesis()?;

        // Initialize mode-specific state
        let mode = if config.is_leader {
            let last_height = services.get_current_height()?;
            let last_hash = services
                .get_current_hash()?
                .unwrap_or([0u8; 32]);

            info!("Leader initialized at height {} with hash {:?}", last_height, &last_hash[..8]);

            NodeMode::Leader(LeaderState {
                last_height,
                last_hash,
            })
        } else {
            info!("Peer initialized, will connect to leader");
            NodeMode::Peer(PeerState)
        };

        info!("Node created successfully with peer ID: {}", services.local_peer_id());

        Ok(Self {
            config,
            services,
            mode,
        })
    }

    /// Run the node (main event loop)
    pub async fn run(self) -> Result<(), NodeError> {
        info!("Starting node main loop...");

        let Node { config, services, mode } = self;

        match mode {
            NodeMode::Leader(state) => run_leader(config, services, state).await,
            NodeMode::Peer(state) => run_peer(config, services, state).await,
        }
    }

    /// Graceful shutdown
    pub fn shutdown(&self) -> Result<(), NodeError> {
        info!("Shutting down node...");

        // Save final metadata
        // Note: In a real implementation, we might want to flush RocksDB

        info!("Node shutdown complete");
        Ok(())
    }

    /// Get current chain height
    pub fn get_height(&self) -> Result<u64, NodeError> {
        self.services.get_current_height()
    }

    /// Check if node is a leader
    pub fn is_leader(&self) -> bool {
        self.config.is_leader
    }
}

/// Run leader mode
async fn run_leader(
    config: NodeConfig,
    mut services: NodeServices,
    mut state: LeaderState,
) -> Result<(), NodeError> {
    info!("Running in LEADER mode");
    info!("Block production interval: {} seconds", config.block_interval_secs);

    let mut block_timer = interval(config.block_interval());

    loop {
        tokio::select! {
            _ = block_timer.tick() => {
                if let Err(e) = produce_block(&mut services, &mut state).await {
                    error!("Block production failed: {}", e);
                    // Continue running even if block production fails
                }
            }
            event = services.network.next() => {
                match event {
                    Some(NetworkEvent::PeerConnected(peer_id)) => {
                        info!("Peer connected: {}", peer_id);
                    }
                    Some(NetworkEvent::PeerDisconnected(peer_id)) => {
                        warn!("Peer disconnected: {}", peer_id);
                    }
                    Some(NetworkEvent::BlockReceived { data, source }) => {
                        warn!("Leader received block from {}, ignoring", source);
                        // Leaders don't process incoming blocks
                    }
                    None => {
                        info!("Network stream closed, shutting down");
                        break;
                    }
                }
            }
        }
    }

    info!("Leader node shutting down");
    Ok(())
}

/// Produce a new block (leader only)
async fn produce_block(
    services: &mut NodeServices,
    state: &mut LeaderState,
) -> Result<(), NodeError> {
    let new_height = state.last_height + 1;
    let timestamp = current_unix_time();

    info!("Producing block at height {}...", new_height);

    // Collect pending messages
    let pending_messages = services.collect_pending_messages()?;
    let message_count = pending_messages.len();
    let message_ids: Vec<[u8; 32]> = pending_messages.iter().map(|m| m.id).collect();

    // Create block header
    let header = BlockHeader {
        height: new_height,
        timestamp,
        prev_hash: state.last_hash,
        message_root: [0u8; 32], // Placeholder for M3
    };

    // Create block with messages
    let block = Block {
        header,
        message_ids,
    };

    let block_hash = block.hash();

    // Save block to database
    services
        .block_store
        .put_block(new_height, &block)
        .map_err(|e| NodeError::block_production(format!("Failed to save block: {}", e)))?;

    // Update metadata
    services
        .save_metadata(new_height, *block_hash.as_bytes())
        .map_err(|e| NodeError::block_production(format!("Failed to save metadata: {}", e)))?;

    // Clear pending messages
    services.clear_pending_messages()?;

    // Publish to network
    services
        .network
        .publish_block(&block)
        .map_err(|e| NodeError::block_production(format!("Failed to publish block: {}", e)))?;

    // Update leader state
    state.last_height = new_height;
    state.last_hash = *block_hash.as_bytes();

    info!(
        "Block {} produced and published with {} messages",
        new_height, message_count
    );

    Ok(())
}

/// Run peer mode
async fn run_peer(
    config: NodeConfig,
    mut services: NodeServices,
    mut state: PeerState,
) -> Result<(), NodeError> {
    info!("Running in PEER mode");

    // Dial leader if configured
    if let Some(ref leader_addr) = config.leader_addr {
        info!("Connecting to leader at {}...", leader_addr);
        if let Err(e) = services.network.dial(leader_addr.clone()) {
            warn!("Failed to dial leader: {}. Will retry via network events.", e);
        }
    }

    loop {
        match services.network.next().await {
            Some(NetworkEvent::BlockReceived { data, source }) => {
                debug!("Received block data from {}", source);
                if let Err(e) = process_received_block(&mut services, &data).await {
                    error!("Failed to process block from {}: {}", source, e);
                }
            }
            Some(NetworkEvent::PeerConnected(peer_id)) => {
                info!("Connected to peer: {}", peer_id);
                // Could track if this is the leader
            }
            Some(NetworkEvent::PeerDisconnected(peer_id)) => {
                warn!("Peer disconnected: {}", peer_id);
            }
            None => {
                info!("Network stream closed, shutting down");
                break;
            }
        }
    }

    info!("Peer node shutting down");
    Ok(())
}

/// Process a received block
async fn process_received_block(
    services: &mut NodeServices,
    data: &[u8],
) -> Result<(), NodeError> {
    // Deserialize block
    let block: Block = serde_json::from_slice(data)
        .map_err(|e| NodeError::block_processing(format!("Deserialization failed: {}", e)))?;

    let block_height = block.header.height;
    let block_hash = block.hash();

    debug!("Processing block {}...", block_height);

    // Validate block
    let current_height = services.get_current_height()?;
    let current_hash = services.get_current_hash()?.unwrap_or([0u8; 32]);

    // Check height continuity
    if block_height != current_height + 1 {
        return Err(NodeError::block_processing(format!(
            "Height mismatch: expected {}, got {}",
            current_height + 1,
            block_height
        )));
    }

    // Check previous hash
    if block.header.prev_hash != current_hash {
        return Err(NodeError::block_processing(format!(
            "Previous hash mismatch at height {}",
            block_height
        )));
    }

    // Block is valid, save it
    services
        .block_store
        .put_block(block_height, &block)
        .map_err(|e| NodeError::block_processing(format!("Failed to save block: {}", e)))?;

    // Update metadata
    services
        .save_metadata(block_height, *block_hash.as_bytes())
        .map_err(|e| NodeError::block_processing(format!("Failed to save metadata: {}", e)))?;

    info!(
        "Block {} validated and saved",
        block_height
    );

    Ok(())
}

/// Get current Unix timestamp
fn current_unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unix_time() {
        let t1 = current_unix_time();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let t2 = current_unix_time();
        assert!(t2 >= t1);
    }

    // Note: Node creation tests require full database and network initialization
    // These are integration tests and should be run separately with:
    // cargo test --test integration -- --ignored
}
