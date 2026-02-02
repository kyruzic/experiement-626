use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Kimura blockchain node configuration
#[derive(Debug, Clone, Parser, Serialize, Deserialize)]
#[command(name = "kimura-node")]
#[command(about = "Kimura blockchain node")]
#[command(version)]
pub struct NodeConfig {
    /// Run as leader node
    #[arg(long, default_value = "false")]
    pub is_leader: bool,

    /// Database path
    #[arg(long, default_value = "./data")]
    pub db_path: PathBuf,

    /// Network listen address
    #[arg(long, default_value = "/ip4/0.0.0.0/tcp/0")]
    pub listen_addr: String,

    /// Leader address (required for peer mode)
    #[arg(long)]
    pub leader_addr: Option<String>,

    /// Block production interval in seconds (leader only)
    #[arg(long, default_value = "5")]
    pub block_interval_secs: u64,

    /// Log level
    #[arg(long, default_value = "info")]
    pub log_level: String,

    /// RPC server port (0 = auto-assign)
    #[arg(long, default_value = "0")]
    pub rpc_port: u16,
}

impl NodeConfig {
    /// Create a new configuration for a leader node
    pub fn leader(db_path: impl Into<PathBuf>, listen_addr: impl Into<String>) -> Self {
        Self {
            is_leader: true,
            db_path: db_path.into(),
            listen_addr: listen_addr.into(),
            leader_addr: None,
            block_interval_secs: 5,
            log_level: "info".to_string(),
            rpc_port: 0,
        }
    }

    /// Create a new configuration for a peer node
    pub fn peer(
        db_path: impl Into<PathBuf>,
        listen_addr: impl Into<String>,
        leader_addr: impl Into<String>,
    ) -> Self {
        Self {
            is_leader: false,
            db_path: db_path.into(),
            listen_addr: listen_addr.into(),
            leader_addr: Some(leader_addr.into()),
            block_interval_secs: 5,
            log_level: "info".to_string(),
            rpc_port: 0,
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Peer mode requires leader address
        if !self.is_leader && self.leader_addr.is_none() {
            return Err(ConfigError::MissingLeaderAddr);
        }

        // Validate block interval
        if self.block_interval_secs == 0 {
            return Err(ConfigError::InvalidBlockInterval);
        }

        // Validate leader doesn't have leader_addr set (optional warning)
        if self.is_leader && self.leader_addr.is_some() {
            eprintln!("Warning: Leader node has leader_addr set, this will be ignored");
        }

        Ok(())
    }

    /// Get the block interval as a Duration
    pub fn block_interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.block_interval_secs)
    }
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            is_leader: false,
            db_path: PathBuf::from("./data"),
            listen_addr: "/ip4/0.0.0.0/tcp/0".to_string(),
            leader_addr: None,
            block_interval_secs: 5,
            log_level: "info".to_string(),
            rpc_port: 0,
        }
    }
}

/// Configuration errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("peer mode requires --leader-addr")]
    MissingLeaderAddr,

    #[error("block interval must be greater than 0")]
    InvalidBlockInterval,

    #[error("failed to load config file: {0}")]
    FileLoadError(String),

    #[error("invalid config: {0}")]
    InvalidConfig(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_leader() {
        let config = NodeConfig::leader("/tmp/leader", "/ip4/0.0.0.0/tcp/5001");
        assert!(config.is_leader);
        assert_eq!(config.db_path, PathBuf::from("/tmp/leader"));
        assert_eq!(config.listen_addr, "/ip4/0.0.0.0/tcp/5001");
        assert!(config.leader_addr.is_none());
        assert_eq!(config.block_interval_secs, 5);
    }

    #[test]
    fn test_config_peer() {
        let config = NodeConfig::peer("/tmp/peer", "/ip4/0.0.0.0/tcp/0", "/ip4/127.0.0.1/tcp/5001");
        assert!(!config.is_leader);
        assert_eq!(config.db_path, PathBuf::from("/tmp/peer"));
        assert_eq!(config.listen_addr, "/ip4/0.0.0.0/tcp/0");
        assert_eq!(
            config.leader_addr,
            Some("/ip4/127.0.0.1/tcp/5001".to_string())
        );
    }

    #[test]
    fn test_validate_leader_ok() {
        let config = NodeConfig::leader("/tmp/leader", "/ip4/0.0.0.0/tcp/5001");
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_peer_ok() {
        let config = NodeConfig::peer("/tmp/peer", "/ip4/0.0.0.0/tcp/0", "/ip4/127.0.0.1/tcp/5001");
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_peer_missing_leader() {
        let mut config = NodeConfig::default();
        config.is_leader = false;
        config.leader_addr = None;
        assert!(matches!(
            config.validate(),
            Err(ConfigError::MissingLeaderAddr)
        ));
    }

    #[test]
    fn test_validate_invalid_interval() {
        let mut config = NodeConfig::leader("/tmp/leader", "/ip4/0.0.0.0/tcp/5001");
        config.block_interval_secs = 0;
        assert!(matches!(
            config.validate(),
            Err(ConfigError::InvalidBlockInterval)
        ));
    }

    #[test]
    fn test_default_config() {
        let config = NodeConfig::default();
        assert!(!config.is_leader);
        assert_eq!(config.block_interval_secs, 5);
    }

    #[test]
    fn test_block_interval() {
        let config = NodeConfig::leader("/tmp/leader", "/ip4/0.0.0.0/tcp/5001");
        let duration = config.block_interval();
        assert_eq!(duration, std::time::Duration::from_secs(5));
    }
}
