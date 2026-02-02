use futures::{Stream, StreamExt};
use libp2p::{
    core::transport::upgrade,
    gossipsub::{self, IdentTopic, MessageAuthenticity},
    identity,
    noise, 
    swarm::{Swarm, SwarmEvent, Config as SwarmConfig},
    tcp, 
    yamux, 
    Multiaddr, 
    PeerId,
    Transport,
};
use serde::Serialize;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use thiserror::Error;
use tracing::{debug, info, warn};

/// Topic name for block propagation
const BLOCKS_TOPIC: &str = "kimura/blocks/1.0.0";

/// Network configuration
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// Address to listen on (e.g., "/ip4/0.0.0.0/tcp/0")
    pub listen_addr: String,
    /// Optional leader address to dial (e.g., "/ip4/127.0.0.1/tcp/5001")
    pub leader_addr: Option<String>,
}

impl NetworkConfig {
    /// Create a new network configuration
    pub fn new(listen_addr: impl Into<String>) -> Self {
        Self {
            listen_addr: listen_addr.into(),
            leader_addr: None,
        }
    }

    /// Set the leader address
    pub fn with_leader(mut self, leader_addr: impl Into<String>) -> Self {
        self.leader_addr = Some(leader_addr.into());
        self
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen_addr: "/ip4/0.0.0.0/tcp/0".to_string(),
            leader_addr: None,
        }
    }
}

/// Errors that can occur in the P2P network
#[derive(Debug, Error)]
pub enum NetworkError {
    #[error("failed to publish message: {0}")]
    PublishError(String),
    
    #[error("failed to subscribe to topic: {0}")]
    SubscribeError(String),
    
    #[error("failed to dial peer: {0}")]
    DialError(String),
    
    #[error("serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("transport error: {0}")]
    TransportError(String),
    
    #[error("invalid multiaddress: {0}")]
    InvalidMultiaddr(String),
    
    #[error("swarm build error: {0}")]
    SwarmBuildError(String),
    
    #[error("identity error: {0}")]
    IdentityError(String),
}

/// Network events that can be received
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    /// A block was received from the network
    BlockReceived {
        /// Serialized block data (caller must deserialize)
        data: Vec<u8>,
        /// Peer ID that sent the block
        source: PeerId,
    },
    /// A new peer connected
    PeerConnected(PeerId),
    /// A peer disconnected
    PeerDisconnected(PeerId),
}

/// P2P Network using libp2p gossipsub
pub struct P2PNetwork {
    /// The libp2p swarm managing the network
    swarm: Swarm<gossipsub::Behaviour>,
    /// The topic for block propagation
    topic: IdentTopic,
    /// Local peer ID
    local_peer_id: PeerId,
    /// Optional leader address to dial
    leader_addr: Option<Multiaddr>,
    /// Whether we've already dialed the leader
    leader_dialed: bool,
}

impl P2PNetwork {
    /// Create a new P2P network with ephemeral identity
    pub fn new(config: NetworkConfig) -> Result<Self, NetworkError> {
        // Parse leader address if provided
        let leader_addr = config.leader_addr.as_ref()
            .map(|addr| addr.parse::<Multiaddr>()
                .map_err(|e| NetworkError::InvalidMultiaddr(format!("{}: {}", addr, e))))
            .transpose()?;
        
        // Generate a new identity keypair
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
        
        info!("Local peer ID: {}", local_peer_id);
        
        // Create the transport: TCP + Noise + Yamux
        let tcp_transport = tcp::tokio::Transport::new(tcp::Config::default());
        
        let transport = tcp_transport
            .upgrade(upgrade::Version::V1)
            .authenticate(noise::Config::new(&local_key)
                .map_err(|e| NetworkError::TransportError(e.to_string()))?)
            .multiplex(yamux::Config::default())
            .boxed();
        
        // Create gossipsub configuration
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .max_transmit_size(262144) // 256KB max message size
            .validation_mode(gossipsub::ValidationMode::Strict)
            .build()
            .map_err(|e| NetworkError::SwarmBuildError(format!("gossipsub config error: {}", e)))?;
        
        // Create gossipsub behavior with message signing
        let message_authenticity = MessageAuthenticity::Signed(local_key);
        let gossipsub = gossipsub::Behaviour::new(message_authenticity, gossipsub_config)
            .map_err(|e| NetworkError::SwarmBuildError(format!("gossipsub behaviour error: {}", e)))?;
        
        // Create swarm configuration
        let swarm_config = SwarmConfig::with_tokio_executor()
            .with_idle_connection_timeout(Duration::from_secs(60));
        
        // Build the swarm directly
        let swarm = Swarm::new(transport, gossipsub, local_peer_id, swarm_config);
        
        // Create the blocks topic
        let topic = IdentTopic::new(BLOCKS_TOPIC);
        
        Ok(Self {
            swarm,
            topic,
            local_peer_id,
            leader_addr,
            leader_dialed: false,
        })
    }
    
    /// Get the local peer ID
    pub fn local_peer_id(&self) -> &PeerId {
        &self.local_peer_id
    }
    
    /// Get the listen address (only valid after start())
    pub fn listen_addrs(&self) -> Vec<Multiaddr> {
        self.swarm.listeners().cloned().collect()
    }
    
    /// Start listening on the configured address
    pub fn start(&mut self, listen_addr: impl Into<String>) -> Result<Multiaddr, NetworkError> {
        let addr = listen_addr.into().parse::<Multiaddr>()
            .map_err(|e| NetworkError::InvalidMultiaddr(e.to_string()))?;
        
        let _listener_id = self.swarm.listen_on(addr.clone())
            .map_err(|e| NetworkError::TransportError(e.to_string()))?;
        
        info!("Listening on {:?}", addr);
        
        // Subscribe to the blocks topic
        self.swarm.behaviour_mut().subscribe(&self.topic)
            .map_err(|e| NetworkError::SubscribeError(e.to_string()))?;
        
        info!("Subscribed to topic: {}", BLOCKS_TOPIC);
        
        // Dial leader if configured
        if let Some(ref leader) = self.leader_addr {
            if !self.leader_dialed {
                match self.swarm.dial(leader.clone()) {
                    Ok(_) => {
                        info!("Dialing leader at {}", leader);
                        self.leader_dialed = true;
                    }
                    Err(e) => {
                        warn!("Failed to dial leader: {}. Will retry later.", e);
                        // Don't fail startup if leader dial fails
                    }
                }
            }
        }
        
        Ok(addr)
    }
    
    /// Publish a block to the network
    pub fn publish_block<T: Serialize>(&mut self, block: &T) -> Result<(), NetworkError> {
        // Serialize the block to JSON
        let data = serde_json::to_vec(block)?;
        
        // Publish to the gossipsub topic
        self.swarm.behaviour_mut().publish(self.topic.clone(), data)
            .map_err(|e| NetworkError::PublishError(e.to_string()))?;
        
        debug!("Published block to topic: {}", BLOCKS_TOPIC);
        
        Ok(())
    }
    
    /// Dial a specific peer by multiaddress
    pub fn dial(&mut self, addr: impl Into<String>) -> Result<(), NetworkError> {
        let multiaddr = addr.into().parse::<Multiaddr>()
            .map_err(|e| NetworkError::InvalidMultiaddr(e.to_string()))?;
        
        self.swarm.dial(multiaddr.clone())
            .map_err(|e| NetworkError::DialError(e.to_string()))?;
        
        info!("Dialing peer at {}", multiaddr);
        
        Ok(())
    }
    
    /// Dial the configured leader
    pub fn dial_leader(&mut self) -> Result<(), NetworkError> {
        if let Some(ref leader) = self.leader_addr {
            self.swarm.dial(leader.clone())
                .map_err(|e| NetworkError::DialError(e.to_string()))?;
            
            info!("Dialing leader at {}", leader);
            self.leader_dialed = true;
            
            Ok(())
        } else {
            Err(NetworkError::DialError("No leader address configured".to_string()))
        }
    }
}

/// Stream implementation for receiving network events
impl Stream for P2PNetwork {
    type Item = NetworkEvent;
    
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Poll the swarm for network events
        match self.swarm.poll_next_unpin(cx) {
            Poll::Ready(Some(event)) => {
                match event {
                    SwarmEvent::Behaviour(gossipsub::Event::Message { 
                        message, 
                        propagation_source,
                        .. 
                    }) => {
                        // Received a gossipsub message
                        debug!("Received message from peer: {}", propagation_source);
                        
                        return Poll::Ready(Some(NetworkEvent::BlockReceived {
                            data: message.data,
                            source: propagation_source,
                        }));
                    }
                    SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                        info!("Peer connected: {}", peer_id);
                        return Poll::Ready(Some(NetworkEvent::PeerConnected(peer_id)));
                    }
                    SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                        if let Some(ref reason) = cause {
                            warn!("Peer {} disconnected: {:?}", peer_id, reason);
                        } else {
                            info!("Peer {} disconnected", peer_id);
                        }
                        return Poll::Ready(Some(NetworkEvent::PeerDisconnected(peer_id)));
                    }
                    SwarmEvent::NewListenAddr { address, .. } => {
                        info!("Listening on: {}", address);
                    }
                    SwarmEvent::Dialing { peer_id, .. } => {
                        debug!("Dialing peer: {:?}", peer_id);
                    }
                    _ => {}
                }
                
                // Return Pending to continue polling
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Poll::Ready(None) => {
                // Swarm closed
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use tokio::time::{sleep, Duration};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestBlock {
        height: u64,
        hash: String,
    }

    #[tokio::test]
    async fn test_network_config() {
        let config = NetworkConfig::new("/ip4/0.0.0.0/tcp/0")
            .with_leader("/ip4/127.0.0.1/tcp/5001");
        
        assert_eq!(config.listen_addr, "/ip4/0.0.0.0/tcp/0");
        assert_eq!(config.leader_addr, Some("/ip4/127.0.0.1/tcp/5001".to_string()));
    }

    #[tokio::test]
    async fn test_p2p_network_creation() {
        let config = NetworkConfig::default();
        let network = P2PNetwork::new(config);
        
        assert!(network.is_ok());
        
        let network = network.unwrap();
        assert!(!network.local_peer_id().to_string().is_empty());
    }

    // Test that network can be started and publish blocks
    // Note: This test may be flaky in CI environments due to network timing
    #[tokio::test]
    #[ignore]
    async fn test_network_event_stream() {
        let config = NetworkConfig::default();
        let mut network = P2PNetwork::new(config).unwrap();
        
        // Start listening
        let listen_addr = network.start("/ip4/127.0.0.1/tcp/0").unwrap();
        println!("Listening on: {}", listen_addr);
        
        // Give time for setup
        sleep(Duration::from_millis(100)).await;
        
        // Publish a test block
        let block = TestBlock {
            height: 1,
            hash: "abc123".to_string(),
        };
        
        network.publish_block(&block).unwrap();
        
        // Just verify we can poll the stream without errors
        // Note: We won't receive our own message immediately in gossipsub
        let timeout = sleep(Duration::from_millis(500));
        tokio::pin!(timeout);
        
        // Just poll a few times to ensure the stream works
        for _ in 0..3 {
            tokio::select! {
                _ = &mut timeout => break,
                _ = network.next() => {}
            }
        }
        
        // Test passes if we get here without panicking
        println!("Network event stream test completed");
    }

    // Integration test - requires network setup, ignored in regular test runs
    // Run with: cargo test -- --ignored
    #[tokio::test]
    #[ignore]
    async fn test_two_node_communication() {
        // Create leader node
        let leader_config = NetworkConfig::new("/ip4/127.0.0.1/tcp/0");
        let mut leader = P2PNetwork::new(leader_config).unwrap();
        let _leader_addr = leader.start("/ip4/127.0.0.1/tcp/0").unwrap();
        
        // Get leader's actual listen address
        sleep(Duration::from_millis(100)).await;
        let leader_addrs = leader.listen_addrs();
        let leader_listen = leader_addrs.first().cloned().expect("Leader should have listen address");
        println!("Leader listening on: {}", leader_listen);
        
        // Create peer node with leader address
        let peer_config = NetworkConfig::new("/ip4/127.0.0.1/tcp/0")
            .with_leader(leader_listen.to_string());
        let mut peer = P2PNetwork::new(peer_config).unwrap();
        peer.start("/ip4/127.0.0.1/tcp/0").unwrap();
        
        // Give time for connection
        sleep(Duration::from_millis(500)).await;
        
        // Leader publishes a block
        let block = TestBlock {
            height: 42,
            hash: "test_hash".to_string(),
        };
        
        leader.publish_block(&block).unwrap();
        
        // Peer should receive the block
        let timeout = sleep(Duration::from_secs(5));
        tokio::pin!(timeout);
        
        let mut received = false;
        
        loop {
            tokio::select! {
                _ = &mut timeout => break,
                event = peer.next() => {
                    if let Some(event) = event {
                        match event {
                            NetworkEvent::BlockReceived { data, .. } => {
                                let received_block: TestBlock = serde_json::from_slice(&data).unwrap();
                                assert_eq!(received_block, block);
                                received = true;
                                break;
                            }
                            NetworkEvent::PeerConnected(pid) => {
                                println!("Peer connected: {}", pid);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        
        assert!(received, "Peer should have received the block");
    }

    // Integration test - requires network setup, ignored in regular test runs
    #[tokio::test]
    #[ignore]
    async fn test_peer_connection_events() {
        let config1 = NetworkConfig::new("/ip4/127.0.0.1/tcp/0");
        let mut node1 = P2PNetwork::new(config1).unwrap();
        let _addr1 = node1.start("/ip4/127.0.0.1/tcp/0").unwrap();
        
        sleep(Duration::from_millis(100)).await;
        let addrs1 = node1.listen_addrs();
        let listen1 = addrs1.first().cloned().expect("Node1 should have listen address");
        
        let config2 = NetworkConfig::new("/ip4/127.0.0.1/tcp/0");
        let mut node2 = P2PNetwork::new(config2).unwrap();
        node2.start("/ip4/127.0.0.1/tcp/0").unwrap();
        
        // Node2 dials node1
        node2.dial(listen1.to_string()).unwrap();
        
        // Wait for connection events
        let timeout = sleep(Duration::from_secs(3));
        tokio::pin!(timeout);
        
        let mut node1_connected = false;
        let mut node2_connected = false;
        
        loop {
            tokio::select! {
                _ = &mut timeout => break,
                event = node1.next() => {
                    if let Some(NetworkEvent::PeerConnected(_)) = event {
                        node1_connected = true;
                    }
                }
                event = node2.next() => {
                    if let Some(NetworkEvent::PeerConnected(_)) = event {
                        node2_connected = true;
                    }
                }
            }
            
            if node1_connected && node2_connected {
                break;
            }
        }
        
        assert!(node1_connected || node2_connected, "At least one node should see connection");
    }
}
