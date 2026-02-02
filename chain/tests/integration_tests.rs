//! Integration tests for the Kimura blockchain using RPC
//!
//! These tests verify end-to-end functionality by spinning up actual nodes
//! with RPC servers and testing via HTTP calls.

use kimura_node::{Node, NodeConfig};
use std::path::PathBuf;

mod rpc_client;
use rpc_client::{RpcClient, BlockResponse};

use tempfile::TempDir;
use tokio::time::{sleep, Duration};
use tracing::info;
/// Short block interval for faster tests (1 second)
const TEST_BLOCK_INTERVAL: u64 = 1;
/// Test network base port
const TEST_BASE_PORT: u16 = 15000;

/// Test wrapper around Node for easier test management with RPC
pub struct TestNode {
    /// Node configuration
    pub config: NodeConfig,
    /// Temporary directory for database (auto-cleaned)
    pub temp_dir: TempDir,
    /// Network port for this node
    pub port: u16,
    /// RPC port (assigned at runtime)
    pub rpc_port: u16,
    /// RPC client for making HTTP calls
    pub rpc_client: RpcClient,
    /// Node handle for shutdown
    node_handle: Option<tokio::task::JoinHandle<Result<(), kimura_node::NodeError>>>,
}

impl TestNode {
    /// Create and start a new test node as leader with RPC
    pub async fn new_leader(port: u16) -> Self {
        let temp_dir = TempDir::new().unwrap();
        let config = NodeConfig {
            is_leader: true,
            db_path: temp_dir.path().join("db"),
            listen_addr: format!("/ip4/127.0.0.1/tcp/{}", port),
            leader_addr: None,
            block_interval_secs: TEST_BLOCK_INTERVAL,
            log_level: "debug".to_string(),
        };

        // Create node with RPC
        let (node, rpc_port) = Node::new_with_rpc(config.clone()).await
            .expect("Failed to create leader node with RPC");

        let rpc_client = RpcClient::new(rpc_port);

        let mut test_node = Self {
            config,
            temp_dir,
            port,
            rpc_port,
            rpc_client,
            node_handle: None,
        };

        // Start the node
        test_node.node_handle = Some(tokio::spawn(async move {
            node.run().await
        }));

        // Wait for RPC to be ready
        test_node.wait_for_rpc().await;

        test_node
    }

    /// Create and start a new test node as peer with RPC
    pub async fn new_peer(port: u16, leader_port: u16) -> Self {
        let temp_dir = TempDir::new().unwrap();
        let config = NodeConfig {
            is_leader: false,
            db_path: temp_dir.path().join("db"),
            listen_addr: format!("/ip4/127.0.0.1/tcp/{}", port),
            leader_addr: Some(format!("/ip4/127.0.0.1/tcp/{}", leader_port)),
            block_interval_secs: TEST_BLOCK_INTERVAL,
            log_level: "debug".to_string(),
        };

        // Create node with RPC
        let (node, rpc_port) = Node::new_with_rpc(config.clone()).await
            .expect("Failed to create peer node with RPC");

        let rpc_client = RpcClient::new(rpc_port);

        let mut test_node = Self {
            config,
            temp_dir,
            port,
            rpc_port,
            rpc_client,
            node_handle: None,
        };

        // Start the node
        test_node.node_handle = Some(tokio::spawn(async move {
            node.run().await
        }));

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
            if start.elapsed() > Duration::from_secs(5) {
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
    pub async fn get_latest(&self) -> BlockResponse {
        self.rpc_client.latest().await
            .expect("Failed to get latest block via RPC")
    }

    /// Submit a message via RPC
    pub async fn submit_message(&self, sender: &str, content: &str) -> String {
        self.rpc_client.submit_message(sender, content).await
            .expect("Failed to submit message via RPC")
    }

    /// Stop the node
    pub async fn stop(&mut self) {
        if let Some(handle) = self.node_handle.take() {
            handle.abort();
            sleep(Duration::from_millis(200)).await;
        }
    }
}

impl Drop for TestNode {
    fn drop(&mut self) {
        if let Some(handle) = self.node_handle.take() {
            handle.abort();
        }
    }
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
///
/// Verifies that a leader node:
/// - Initializes correctly with genesis block (height = 0)
/// - Produces blocks at regular intervals
/// - Maintains sequential height
#[tokio::test]
async fn test_leader_produces_blocks_rpc() {
    info!("Starting test_leader_produces_blocks_rpc");

    let leader = TestNode::new_leader(TEST_BASE_PORT).await;

    // Wait for genesis block to be produced
    let height = wait_for_height_rpc(&leader, 0, Duration::from_secs(5))
        .await
        .expect("Should have genesis block");
    assert_eq!(height, 0, "Initial height should be 0 (genesis)");

    // Wait for block 1
    wait_for_height_rpc(&leader, 1, Duration::from_secs(5))
        .await
        .expect("Should produce block 1");

    // Wait for block 2
    wait_for_height_rpc(&leader, 2, Duration::from_secs(5))
        .await
        .expect("Should produce block 2");

    // Verify block continuity via RPC
    let block1 = leader.get_block(1).await.expect("Block 1 should exist");
    let block2 = leader.get_block(2).await.expect("Block 2 should exist");

    assert_eq!(block1.height, 1, "Block 1 height mismatch");
    assert_eq!(block2.height, 2, "Block 2 height mismatch");

    // Verify previous hash links
    assert!(!block1.prev_hash.is_empty(), "Block 1 should have prev_hash");
    assert!(!block2.prev_hash.is_empty(), "Block 2 should have prev_hash");

    info!("test_leader_produces_blocks_rpc completed successfully");
}

/// Test 2: Peer receives and validates blocks via RPC
///
/// Verifies that a peer node:
/// - Connects to leader successfully
/// - Receives blocks via gossipsub
/// - Stores valid blocks in local database
/// - Chain matches leader via RPC queries
#[tokio::test]
async fn test_peer_receives_blocks_rpc() {
    info!("Starting test_peer_receives_blocks_rpc");

    // Start leader
    let leader = TestNode::new_leader(TEST_BASE_PORT + 10).await;

    // Wait for leader to produce some blocks
    wait_for_height_rpc(&leader, 2, Duration::from_secs(5))
        .await
        .expect("Leader should produce blocks");

    // Start peer
    let peer = TestNode::new_peer(TEST_BASE_PORT + 11, TEST_BASE_PORT + 10).await;

    // Give nodes time to connect and sync
    sleep(Duration::from_secs(4)).await;

    // Wait for peer to catch up
    let peer_height = wait_for_height_rpc(&peer, 2, Duration::from_secs(10))
        .await
        .expect("Peer should receive blocks from leader");

    let leader_height = leader.get_height().await;

    info!(
        "Chain heights - Leader: {}, Peer: {}",
        leader_height, peer_height
    );

    assert!(
        peer_height >= 2,
        "Peer should have at least genesis + 2 blocks. Leader: {}, Peer: {}",
        leader_height,
        peer_height
    );

    // Verify chains match via RPC
    verify_chain_equality_rpc(&leader, &peer)
        .await
        .expect("Chains should be equal");

    info!("test_peer_receives_blocks_rpc completed successfully");
}

/// Test 3: Multi-peer synchronization via RPC
///
/// Verifies that multiple peers:
/// - Can connect to the same leader
/// - All receive the same blocks
/// - Late-joining peer can catch up
/// - All chains remain consistent
#[tokio::test]
async fn test_multi_peer_sync_rpc() {
    info!("Starting test_multi_peer_sync_rpc");

    // Start leader
    let leader = TestNode::new_leader(TEST_BASE_PORT + 20).await;

    // Wait for leader to start
    sleep(Duration::from_millis(500)).await;

    // Start first peer (early joiner)
    let peer1 = TestNode::new_peer(TEST_BASE_PORT + 21, TEST_BASE_PORT + 20).await;

    // Wait for some blocks to be produced
    wait_for_height_rpc(&leader, 3, Duration::from_secs(5))
        .await
        .expect("Leader should produce blocks");

    // Start second peer (late joiner)
    let peer2 = TestNode::new_peer(TEST_BASE_PORT + 22, TEST_BASE_PORT + 20).await;

    // Wait for more blocks (give peer2 time to catch up)
    sleep(Duration::from_secs(4)).await;

    // Wait for all nodes to sync
    wait_for_height_rpc(&peer1, 3, Duration::from_secs(5))
        .await
        .expect("Peer1 should sync");
    wait_for_height_rpc(&peer2, 3, Duration::from_secs(8))
        .await
        .expect("Peer2 should catch up");

    // Query all chain heights via RPC
    let leader_height = leader.get_height().await;
    let peer1_height = peer1.get_height().await;
    let peer2_height = peer2.get_height().await;

    info!(
        "Chain heights - Leader: {}, Peer1: {}, Peer2: {}",
        leader_height, peer1_height, peer2_height
    );

    assert_eq!(leader_height, peer1_height, "Peer1 should match leader");
    assert_eq!(leader_height, peer2_height, "Peer2 should match leader");

    // Verify all chains match via RPC
    verify_chain_equality_rpc(&leader, &peer1)
        .await
        .expect("Leader and peer1 should match");
    verify_chain_equality_rpc(&leader, &peer2)
        .await
        .expect("Leader and peer2 should match");

    info!("test_multi_peer_sync_rpc completed successfully");
}

/// Test 4: Message inclusion via RPC submission
///
/// Verifies that:
/// - Messages can be submitted via RPC
/// - Messages are included in blocks
/// - Message IDs are stored correctly
#[tokio::test]
async fn test_message_inclusion_rpc() {
    info!("Starting test_message_inclusion_rpc");

    // Start leader
    let leader = TestNode::new_leader(TEST_BASE_PORT + 30).await;

    // Wait for genesis
    wait_for_height_rpc(&leader, 0, Duration::from_secs(3))
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
///
/// Verifies that:
/// - Blocks form a continuous chain
/// - Previous hash links are correct
/// - Timestamps increase monotonically
#[tokio::test]
async fn test_chain_continuity_rpc() {
    info!("Starting test_chain_continuity_rpc");

    // Start leader and produce some blocks
    let leader = TestNode::new_leader(TEST_BASE_PORT + 40).await;

    // Wait for multiple blocks
    wait_for_height_rpc(&leader, 5, Duration::from_secs(8))
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
            // Verify previous hash link
            assert_eq!(
                block.prev_hash, prev_hash,
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

/// Test 6: Graceful shutdown and restart via RPC
///
/// Verifies that:
/// - Node can shut down cleanly
/// - Metadata is persisted to database
/// - Node can restart and resume from last block
/// - No data loss occurs
#[tokio::test]
async fn test_graceful_shutdown_rpc() {
    info!("Starting test_graceful_shutdown_rpc");

    // Create a persistent directory (not temp, so it survives)
    let db_path = PathBuf::from("/tmp/kimura_test_shutdown_rpc");
    std::fs::remove_dir_all(&db_path).ok(); // Clean up if exists
    std::fs::create_dir_all(&db_path).expect("Failed to create test directory");

    // Phase 1: Start leader, produce some blocks
    let config1 = NodeConfig {
        is_leader: true,
        db_path: db_path.clone(),
        listen_addr: format!("/ip4/127.0.0.1/tcp/{}", TEST_BASE_PORT + 50),
        leader_addr: None,
        block_interval_secs: TEST_BLOCK_INTERVAL,
        log_level: "debug".to_string(),
    };

    let (node1, rpc_port1) = Node::new_with_rpc(config1.clone()).await
        .expect("Failed to create node 1");
    let rpc1 = RpcClient::new(rpc_port1);

    let node1_handle = tokio::spawn(async move {
        node1.run().await
    });

    // Wait for RPC to be ready
    let start = tokio::time::Instant::now();
    loop {
        if rpc1.health().await.is_ok() {
            break;
        }
        if start.elapsed() > Duration::from_secs(5) {
            panic!("RPC failed to start");
        }
        sleep(Duration::from_millis(100)).await;
    }

    // Wait for blocks to be produced
    wait_for_height_via_client(&rpc1, 3, Duration::from_secs(8))
        .await
        .expect("Should produce blocks");

    let height_before_shutdown = rpc1.height().await.expect("Failed to get height");
    info!("Node shut down at height {}", height_before_shutdown);

    // Stop node
    node1_handle.abort();
    sleep(Duration::from_millis(300)).await;

    // Phase 2: Restart node
    let config2 = NodeConfig {
        is_leader: true,
        db_path: db_path.clone(),
        listen_addr: format!("/ip4/127.0.0.1/tcp/{}", TEST_BASE_PORT + 52),
        leader_addr: None,
        block_interval_secs: TEST_BLOCK_INTERVAL,
        log_level: "debug".to_string(),
    };

    let (node2, rpc_port2) = Node::new_with_rpc(config2.clone()).await
        .expect("Failed to create node 2");
    let rpc2 = RpcClient::new(rpc_port2);

    // Verify height persisted
    let height_after_restart = rpc2.height().await.expect("Failed to get height after restart");

    assert_eq!(
        height_before_shutdown, height_after_restart,
        "Height should persist across restarts. Before: {}, After: {}",
        height_before_shutdown, height_after_restart
    );

    // Run node briefly to verify it continues from correct height
    let node2_handle = tokio::spawn(async move {
        node2.run().await
    });

    // Wait for RPC to be ready
    let start = tokio::time::Instant::now();
    loop {
        if rpc2.health().await.is_ok() {
            break;
        }
        if start.elapsed() > Duration::from_secs(5) {
            panic!("RPC failed to start after restart");
        }
        sleep(Duration::from_millis(100)).await;
    }

    // Wait for new blocks to be produced
    sleep(Duration::from_secs(4)).await;

    let final_height = rpc2.height().await.expect("Failed to get final height");

    assert!(
        final_height > height_after_restart,
        "Node should have produced more blocks after restart. Initial: {}, Final: {}",
        height_after_restart, final_height
    );

    // Cleanup
    node2_handle.abort();
    std::fs::remove_dir_all(&db_path).ok();
    info!("test_graceful_shutdown_rpc completed successfully");
}

/// Helper to wait for height using RPC client
async fn wait_for_height_via_client(
    client: &RpcClient,
    target_height: u64,
    max_wait: Duration,
) -> Result<u64, String> {
    let start = tokio::time::Instant::now();
    loop {
        if let Ok(height) = client.height().await {
            if height >= target_height {
                return Ok(height);
            }
        }
        if start.elapsed() > max_wait {
            return Err(format!("Timeout waiting for height {}", target_height));
        }
        sleep(Duration::from_millis(100)).await;
    }
}
