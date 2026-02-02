//! Integration tests for the Kimura blockchain using RPC
//!
//! These tests verify end-to-end functionality by running actual kimura-node binaries
//! and testing via HTTP RPC calls.

use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

mod rpc_client;
use rpc_client::{RpcClient, BlockResponse};

use tempfile::TempDir;
use tokio::time::{sleep, timeout};
use tracing::info;

/// Short block interval for faster tests (1 second)
const TEST_BLOCK_INTERVAL: u64 = 1;
/// Test network base port
const TEST_BASE_PORT: u16 = 15000;
/// Path to kimura-node binary
const KIMURA_NODE_BIN: &str = "target/release/kimura-node";

/// Test wrapper around a running kimura-node process
pub struct TestNode {
    /// Node configuration
    pub config: NodeConfig,
    /// Temporary directory for database (auto-cleaned)
    pub temp_dir: TempDir,
    /// Network port for this node
    pub port: u16,
    /// RPC port (read from process output)
    pub rpc_port: u16,
    /// RPC client for making HTTP calls
    pub rpc_client: RpcClient,
    /// Child process handle
    child: Option<Child>,
    /// Shutdown signal
    shutdown_signal: Arc<AtomicBool>,
}

/// Simplified config for testing
#[derive(Clone)]
pub struct NodeConfig {
    pub is_leader: bool,
    pub db_path: std::path::PathBuf,
    pub listen_addr: String,
    pub leader_addr: Option<String>,
    pub block_interval_secs: u64,
}

impl TestNode {
    /// Create and start a new test node as leader with RPC
    pub async fn new_leader(port: u16) -> Self {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("db");
        let listen_addr = format!("/ip4/127.0.0.1/tcp/{}", port);
        
        // Build command to start leader node
        let mut cmd = Command::new(KIMURA_NODE_BIN);
        cmd.arg("--leader")
            .arg("--db-path")
            .arg(&db_path)
            .arg("--listen-addr")
            .arg(&listen_addr)
            .arg("--block-interval")
            .arg(TEST_BLOCK_INTERVAL.to_string())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        
        info!("Starting leader node: {:?}", cmd);
        
        let mut child = cmd.spawn().expect("Failed to start kimura-node leader");
        
        // Wait a bit for the node to start and print RPC port
        sleep(Duration::from_millis(500)).await;
        
        // Try to find RPC port from the process
        // The node should print something like "RPC server started on port XXXX"
        let rpc_port = wait_for_rpc_port(&mut child, Duration::from_secs(5)).await
            .expect("Failed to get RPC port from leader");
        
        info!("Leader node started with RPC on port {}", rpc_port);
        
        let rpc_client = RpcClient::new(rpc_port);
        let shutdown_signal = Arc::new(AtomicBool::new(false));
        
        let test_node = Self {
            config: NodeConfig {
                is_leader: true,
                db_path,
                listen_addr,
                leader_addr: None,
                block_interval_secs: TEST_BLOCK_INTERVAL,
            },
            temp_dir,
            port,
            rpc_port,
            rpc_client,
            child: Some(child),
            shutdown_signal,
        };
        
        // Wait for RPC to be ready
        test_node.wait_for_rpc().await;
        
        test_node
    }
    
    /// Create and start a new test node as peer with RPC
    pub async fn new_peer(port: u16, leader_port: u16) -> Self {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("db");
        let listen_addr = format!("/ip4/127.0.0.1/tcp/{}", port);
        let leader_addr = format!("/ip4/127.0.0.1/tcp/{}", leader_port);
        
        // Build command to start peer node
        let mut cmd = Command::new(KIMURA_NODE_BIN);
        cmd.arg("--db-path")
            .arg(&db_path)
            .arg("--listen-addr")
            .arg(&listen_addr)
            .arg("--leader-addr")
            .arg(&leader_addr)
            .arg("--block-interval")
            .arg(TEST_BLOCK_INTERVAL.to_string())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        
        info!("Starting peer node: {:?}", cmd);
        
        let mut child = cmd.spawn().expect("Failed to start kimura-node peer");
        
        // Wait a bit for the node to start
        sleep(Duration::from_millis(500)).await;
        
        // Try to find RPC port from the process
        let rpc_port = wait_for_rpc_port(&mut child, Duration::from_secs(5)).await
            .expect("Failed to get RPC port from peer");
        
        info!("Peer node started with RPC on port {}", rpc_port);
        
        let rpc_client = RpcClient::new(rpc_port);
        let shutdown_signal = Arc::new(AtomicBool::new(false));
        
        let test_node = Self {
            config: NodeConfig {
                is_leader: false,
                db_path,
                listen_addr,
                leader_addr: Some(leader_addr),
                block_interval_secs: TEST_BLOCK_INTERVAL,
            },
            temp_dir,
            port,
            rpc_port,
            rpc_client,
            child: Some(child),
            shutdown_signal,
        };
        
        // Wait for RPC to be ready
        test_node.wait_for_rpc().await;
        
        test_node
    }
    
    /// Wait for RPC server to be ready
    async fn wait_for_rpc(&self) {
        let start = tokio::time::Instant::now();
        loop {
            if let Ok(health) = self.rpc_client.health().await {
                info!("RPC ready on port {}: status={}", self.rpc_port, health.status);
                return;
            }
            if start.elapsed() > Duration::from_secs(10) {
                panic!("RPC server failed to start on port {}", self.rpc_port);
            }
            sleep(Duration::from_millis(100)).await;
        }
    }
    
    /// Get current height via RPC
    pub async fn get_height(&self) -> u64 {
        self.rpc_client.height().await
            .expect("Failed to get height via RPC")
    }
    
    /// Get block at specific height via RPC
    pub async fn get_block(&self, height: u64) -> Option<BlockResponse> {
        self.rpc_client.block(height).await.ok()
    }
    
    /// Get latest block via RPC
    pub async fn get_latest(&self) -> Option<BlockResponse> {
        self.rpc_client.latest().await.ok()
    }
    
    /// Submit a message via RPC
    pub async fn submit_message(&self, sender: &str, content: &str) -> String {
        self.rpc_client.submit_message(sender, content).await
            .expect("Failed to submit message via RPC")
    }
    
    /// Stop the node
    pub async fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            info!("Stopping node on port {}...", self.rpc_port);
            // Try graceful shutdown first
            let _ = child.kill();
            let _ = child.wait();
            sleep(Duration::from_millis(200)).await;
            info!("Node stopped");
        }
    }
}

impl Drop for TestNode {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            // Kill the process
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// Wait for RPC port to be printed by the node process
async fn wait_for_rpc_port(child: &mut Child, max_wait: Duration) -> Option<u16> {
    let start = tokio::time::Instant::now();
    
    // For now, use a simple heuristic: try common RPC ports
    // In a real implementation, we'd parse stdout/stderr
    // The RPC server binds to port 0, so we need to parse the actual port
    
    // Try to read from stderr/stdout to find the port
    // This is a simplified version - in practice we'd parse the output
    
    // For testing, we'll try ports in a range
    // This is a workaround until we implement proper output parsing
    sleep(Duration::from_millis(500)).await;
    
    // Try to find an open RPC port by checking health endpoint
    for port in 8000..9000 {
        let client = RpcClient::new(port);
        if timeout(Duration::from_millis(50), client.health()).await.is_ok() {
            return Some(port);
        }
    }
    
    None
}

/// Wait for a specific block height via RPC polling
pub async fn wait_for_height_rpc(
    node: &TestNode,
    target_height: u64,
    max_wait: Duration,
) -> Result<u64, String> {
    let start = tokio::time::Instant::now();
    loop {
        let height = node.get_height().await;
        if height >= target_height {
            return Ok(height);
        }
        if start.elapsed() > max_wait {
            return Err(format!(
                "Timeout waiting for height {}. Current: {}",
                target_height, height
            ));
        }
        sleep(Duration::from_millis(100)).await;
    }
}

/// Verify that two chains are identical via RPC
pub async fn verify_chain_equality_rpc(
    node1: &TestNode,
    node2: &TestNode,
) -> Result<(), String> {
    let height1 = node1.get_height().await;
    let height2 = node2.get_height().await;

    if height1 != height2 {
        return Err(format!(
            "Chain heights don't match: {} vs {}",
            height1, height2
        ));
    }

    for h in 0..=height1 {
        let block1 = node1.get_block(h).await.ok_or_else(|| format!("Block {} not found on node1", h))?;
        let block2 = node2.get_block(h).await.ok_or_else(|| format!("Block {} not found on node2", h))?;

        if block1.height != block2.height {
            return Err(format!("Height mismatch at {}", h));
        }
        if block1.prev_hash != block2.prev_hash {
            return Err(format!("Prev hash mismatch at {}", h));
        }
        if block1.message_count != block2.message_count {
            return Err(format!("Message count mismatch at {}", h));
        }
    }

    Ok(())
}

/// Test 1: Leader produces blocks via RPC verification
#[tokio::test]
async fn test_leader_produces_blocks_rpc() {
    info!("Starting test_leader_produces_blocks_rpc");

    let leader = TestNode::new_leader(TEST_BASE_PORT).await;

    // Wait for blocks to be produced
    let height = wait_for_height_rpc(&leader, 2, Duration::from_secs(10))
        .await
        .expect("Should have at least 2 blocks");
    
    assert!(height >= 2, "Should have produced at least 2 blocks");

    // Verify blocks exist
    let block1 = leader.get_block(1).await.expect("Block 1 should exist");
    let block2 = leader.get_block(2).await.expect("Block 2 should exist");

    assert_eq!(block1.height, 1, "Block 1 height mismatch");
    assert_eq!(block2.height, 2, "Block 2 height mismatch");

    info!("test_leader_produces_blocks_rpc completed successfully");
}

/// Test 2: Peer receives and validates blocks via RPC
#[tokio::test]
async fn test_peer_receives_blocks_rpc() {
    info!("Starting test_peer_receives_blocks_rpc");

    // Start leader
    let leader = TestNode::new_leader(TEST_BASE_PORT + 10).await;

    // Wait for leader to produce initial blocks
    let initial_height = wait_for_height_rpc(&leader, 2, Duration::from_secs(10))
        .await
        .expect("Leader should produce blocks");

    // Start peer
    let peer = TestNode::new_peer(TEST_BASE_PORT + 11, TEST_BASE_PORT + 10).await;

    // Wait for peer to receive NEW blocks (published after connection)
    let target_height = initial_height + 2;
    let peer_height = wait_for_height_rpc(&peer, target_height, Duration::from_secs(15))
        .await
        .expect("Peer should receive blocks from leader");

    info!(
        "Chain heights - Leader: {}, Peer: {}",
        leader.get_height().await, peer_height
    );

    assert!(
        peer_height >= target_height,
        "Peer should have received new blocks"
    );

    info!("test_peer_receives_blocks_rpc completed successfully");
}

/// Test 3: Multi-peer synchronization via RPC
#[tokio::test]
async fn test_multi_peer_sync_rpc() {
    info!("Starting test_multi_peer_sync_rpc");

    // Start leader
    let leader = TestNode::new_leader(TEST_BASE_PORT + 20).await;
    
    // Wait for initial blocks
    let initial_height = wait_for_height_rpc(&leader, 2, Duration::from_secs(10))
        .await
        .expect("Leader should produce blocks");

    // Start peers
    let peer1 = TestNode::new_peer(TEST_BASE_PORT + 21, TEST_BASE_PORT + 20).await;
    let peer2 = TestNode::new_peer(TEST_BASE_PORT + 22, TEST_BASE_PORT + 20).await;

    // Wait for new blocks to be produced after all peers connect
    let target_height = initial_height + 3;
    
    wait_for_height_rpc(&leader, target_height, Duration::from_secs(10))
        .await
        .expect("Leader should continue producing blocks");
    wait_for_height_rpc(&peer1, target_height, Duration::from_secs(15))
        .await
        .expect("Peer1 should receive new blocks");
    wait_for_height_rpc(&peer2, target_height, Duration::from_secs(15))
        .await
        .expect("Peer2 should receive new blocks");

    info!(
        "Chain heights - Leader: {}, Peer1: {}, Peer2: {}",
        leader.get_height().await,
        peer1.get_height().await,
        peer2.get_height().await
    );

    info!("test_multi_peer_sync_rpc completed successfully");
}

/// Test 4: Message inclusion via RPC submission
#[tokio::test]
async fn test_message_inclusion_rpc() {
    info!("Starting test_message_inclusion_rpc");

    // Start leader
    let leader = TestNode::new_leader(TEST_BASE_PORT + 30).await;

    // Wait for genesis
    wait_for_height_rpc(&leader, 0, Duration::from_secs(5))
        .await
        .expect("Should have genesis");

    // Submit messages via RPC
    let msg_id1 = leader.submit_message("test_sender", "Hello Kimura!").await;
    let msg_id2 = leader.submit_message("test_sender", "Second message").await;

    info!("Submitted messages: {}, {}", msg_id1, msg_id2);

    // Wait for block with messages
    sleep(Duration::from_secs(TEST_BLOCK_INTERVAL + 1)).await;

    // Query blocks to find messages
    let height = leader.get_height().await;
    let mut found_message_count = 0;

    for h in 0..=height {
        if let Some(block) = leader.get_block(h).await {
            found_message_count += block.message_count;
            if block.message_count > 0 {
                info!("Block {} contains {} messages", h, block.message_count);
            }
        }
    }

    assert!(
        found_message_count >= 2,
        "Should have at least 2 messages included in blocks, found {}",
        found_message_count
    );

    info!("test_message_inclusion_rpc completed successfully");
}

/// Test 5: Chain continuity validation via RPC
#[tokio::test]
async fn test_chain_continuity_rpc() {
    info!("Starting test_chain_continuity_rpc");

    // Start leader and produce some blocks
    let leader = TestNode::new_leader(TEST_BASE_PORT + 40).await;

    // Wait for multiple blocks
    wait_for_height_rpc(&leader, 5, Duration::from_secs(10))
        .await
        .expect("Should produce at least 5 blocks");

    let height = leader.get_height().await;

    // Verify chain continuity via RPC
    let mut prev_hash = String::new();
    let mut prev_timestamp: u64 = 0;

    for h in 0..=height {
        let block = leader.get_block(h).await
            .expect(&format!("Block {} should exist", h));

        assert_eq!(block.height, h, "Block height mismatch at {}", h);

        if h > 0 {
            // Verify previous hash link (prev_hash is truncated to 8 bytes in response)
            let prev_hash_truncated = &prev_hash[..16.min(prev_hash.len())];
            assert_eq!(
                block.prev_hash, prev_hash_truncated,
                "Previous hash mismatch at height {}", h
            );
        }

        // Verify timestamp increases
        assert!(
            block.timestamp >= prev_timestamp,
            "Timestamp should increase monotonically at height {}",
            h
        );

        prev_hash = block.hash.clone();
        prev_timestamp = block.timestamp;
    }

    info!("test_chain_continuity_rpc completed successfully");
}
