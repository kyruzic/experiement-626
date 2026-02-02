pub mod p2p;

pub use libp2p::PeerId;
pub use p2p::{NetworkConfig, NetworkError, NetworkEvent, P2PNetwork};
