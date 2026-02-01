pub mod p2p;
pub mod transport;
pub mod protocol;

pub use p2p::P2PNetwork;
pub use transport::NetworkTransport;
pub use protocol::NetworkProtocol;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_p2p_network() {
        // TODO: Implement test
    }

    #[test]
    fn test_transport() {
        // TODO: Implement test
    }

    #[test]
    fn test_protocol() {
        // TODO: Implement test
    }
}