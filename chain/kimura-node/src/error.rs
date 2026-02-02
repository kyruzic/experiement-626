use kimura_blockchain::BlockError;
use kimura_network::NetworkError;
use kimura_storage::StorageError;

/// Errors that can occur in the node
#[derive(Debug, thiserror::Error)]
pub enum NodeError {
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("network error: {0}")]
    Network(#[from] NetworkError),

    #[error("block validation error: {0}")]
    Validation(#[from] BlockError),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("node is not a leader")]
    NotLeader,

    #[error("node is not a peer")]
    NotPeer,

    #[error("no leader configured for peer mode")]
    NoLeaderConfigured,

    #[error("database initialization failed: {0}")]
    DatabaseInit(String),

    #[error("network initialization failed: {0}")]
    NetworkInit(String),

    #[error("shutdown error: {0}")]
    Shutdown(String),

    #[error("block production failed: {0}")]
    BlockProduction(String),

    #[error("block processing failed: {0}")]
    BlockProcessing(String),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl NodeError {
    /// Create a configuration error
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    /// Create a database initialization error
    pub fn db_init(msg: impl Into<String>) -> Self {
        Self::DatabaseInit(msg.into())
    }

    /// Create a network initialization error
    pub fn network_init(msg: impl Into<String>) -> Self {
        Self::NetworkInit(msg.into())
    }

    /// Create a block production error
    pub fn block_production(msg: impl Into<String>) -> Self {
        Self::BlockProduction(msg.into())
    }

    /// Create a block processing error
    pub fn block_processing(msg: impl Into<String>) -> Self {
        Self::BlockProcessing(msg.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_error() {
        let err = NodeError::config("missing field");
        assert!(matches!(err, NodeError::Config(_)));
        assert!(err.to_string().contains("missing field"));
    }

    #[test]
    fn test_db_init_error() {
        let err = NodeError::db_init("connection failed");
        assert!(matches!(err, NodeError::DatabaseInit(_)));
        assert!(err.to_string().contains("connection failed"));
    }

    #[test]
    fn test_network_init_error() {
        let err = NodeError::network_init("bind failed");
        assert!(matches!(err, NodeError::NetworkInit(_)));
    }

    #[test]
    fn test_block_production_error() {
        let err = NodeError::block_production("timeout");
        assert!(matches!(err, NodeError::BlockProduction(_)));
    }

    #[test]
    fn test_block_processing_error() {
        let err = NodeError::block_processing("invalid hash");
        assert!(matches!(err, NodeError::BlockProcessing(_)));
    }
}
