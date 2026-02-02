use crate::{config::NodeConfig, error::NodeError};
use kimura_network::{NetworkConfig as P2PNetworkConfig, P2PNetwork};
use kimura_storage::{BlockStore, MessageStore, MetadataStore, RocksDB};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info};

/// Services container for all node components
pub struct NodeServices {
    /// Database connection (shared across stores)
    pub db: Arc<RocksDB>,
    /// Block storage
    pub block_store: BlockStore,
    /// Message storage
    pub message_store: MessageStore,
    /// Metadata storage (chain state)
    pub metadata_store: MetadataStore,
    /// P2P network
    pub network: P2PNetwork,
    /// Network configuration (kept for reference)
    pub network_config: P2PNetworkConfig,
}

impl NodeServices {
    /// Initialize all services from configuration
    pub fn new(config: &NodeConfig) -> Result<Self, NodeError> {
        info!("Initializing node services...");

        // Initialize database
        let db = Self::init_database(&config.db_path)?;
        let db_arc = Arc::new(db);

        // Create stores
        let block_store = BlockStore::new(db_arc.clone());
        let message_store = MessageStore::new(db_arc.clone());
        let metadata_store = MetadataStore::new(db_arc.clone());

        debug!("Storage services initialized");

        // Initialize network
        let (network, network_config) = Self::init_network(config)?;

        info!("All services initialized successfully");

        Ok(Self {
            db: db_arc,
            block_store,
            message_store,
            metadata_store,
            network,
            network_config,
        })
    }

    /// Initialize the RocksDB database
    fn init_database(db_path: &Path) -> Result<RocksDB, NodeError> {
        info!("Initializing database at {:?}", db_path);

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                NodeError::db_init(format!("Failed to create database directory: {}", e))
            })?;
        }

        let db = RocksDB::new(db_path)
            .map_err(|e| NodeError::db_init(format!("Failed to open database: {}", e)))?;

        info!("Database initialized successfully");
        Ok(db)
    }

    /// Initialize the P2P network
    fn init_network(config: &NodeConfig) -> Result<(P2PNetwork, P2PNetworkConfig), NodeError> {
        info!("Initializing P2P network...");

        let network_config = P2PNetworkConfig::new(config.listen_addr.clone())
            .with_leader(config.leader_addr.clone().unwrap_or_default());

        let network = P2PNetwork::new(network_config.clone())
            .map_err(|e| NodeError::network_init(format!("Failed to create network: {}", e)))?;

        info!("P2P network initialized");
        Ok((network, network_config))
    }

    /// Get the local peer ID
    pub fn local_peer_id(&self) -> &kimura_network::PeerId {
        self.network.local_peer_id()
    }

    /// Start listening on the configured address
    pub fn start_listening(&mut self, listen_addr: &str) -> Result<(), NodeError> {
        info!("Starting network listener on {}", listen_addr);

        self.network
            .start(listen_addr)
            .map_err(|e| NodeError::network_init(format!("Failed to start listening: {}", e)))?;

        info!("Network listener started");
        Ok(())
    }

    /// Get the current chain height from metadata
    pub fn get_current_height(&self) -> Result<u64, NodeError> {
        match self.metadata_store.get_last_height()? {
            Some(height) => Ok(height),
            None => Ok(0), // Genesis block is height 0
        }
    }

    /// Get the current chain hash from metadata
    pub fn get_current_hash(&self) -> Result<Option<[u8; 32]>, NodeError> {
        self.metadata_store.get_last_hash().map_err(|e| e.into())
    }

    /// Save chain metadata (height and hash)
    pub fn save_metadata(&self, height: u64, hash: [u8; 32]) -> Result<(), NodeError> {
        self.metadata_store.set_last_height(height)?;
        self.metadata_store.set_last_hash(&hash)?;
        Ok(())
    }

    /// Check if genesis block exists, create if not
    pub fn ensure_genesis(&self) -> Result<(), NodeError> {
        let genesis_height = 0;

        // Check if genesis already exists
        if self
            .block_store
            .get_block::<kimura_blockchain::Block>(genesis_height)?
            .is_some()
        {
            debug!("Genesis block already exists");
            return Ok(());
        }

        info!("Creating genesis block...");

        // Create genesis block
        let genesis = kimura_blockchain::Block::genesis();
        let genesis_hash = genesis.hash();

        // Save genesis block
        self.block_store.put_block(genesis_height, &genesis)?;

        // Save genesis metadata
        self.metadata_store.set_last_height(genesis_height)?;
        self.metadata_store.set_last_hash(genesis_hash.as_bytes())?;
        self.metadata_store
            .set_genesis_hash(genesis_hash.as_bytes())?;

        info!("Genesis block created and saved");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_config() -> NodeConfig {
        NodeConfig {
            is_leader: true,
            db_path: TempDir::new().unwrap().path().join("test_db"),
            listen_addr: "/ip4/127.0.0.1/tcp/0".to_string(),
            leader_addr: None,
            block_interval_secs: 5,
            log_level: "info".to_string(),
        }
    }

    #[test]
    #[ignore = "requires full network stack, run as integration test"]
    fn test_services_creation() {
        let config = create_test_config();
        let services = NodeServices::new(&config);
        assert!(services.is_ok());
    }

    #[test]
    #[ignore = "requires full network stack, run as integration test"]
    fn test_ensure_genesis() {
        let config = create_test_config();
        let services = NodeServices::new(&config).unwrap();

        // First call creates genesis
        assert!(services.ensure_genesis().is_ok());

        // Second call should succeed (idempotent)
        assert!(services.ensure_genesis().is_ok());

        // Verify genesis exists
        let genesis = services
            .block_store
            .get_block::<kimura_blockchain::Block>(0)
            .unwrap();
        assert!(genesis.is_some());
    }

    #[test]
    #[ignore = "requires full network stack, run as integration test"]
    fn test_metadata_operations() {
        let config = create_test_config();
        let services = NodeServices::new(&config).unwrap();

        // Initially no metadata
        let height = services.get_current_height().unwrap();
        assert_eq!(height, 0);

        // Save metadata
        let test_hash = [0xAB; 32];
        services.save_metadata(42, test_hash).unwrap();

        // Verify saved
        let height = services.get_current_height().unwrap();
        assert_eq!(height, 42);

        let hash = services.get_current_hash().unwrap();
        assert_eq!(hash, Some(test_hash));
    }
}
