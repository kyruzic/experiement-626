pub mod p2p;
pub mod protocol;
pub mod transport;

pub use p2p::P2PNetwork;
pub use protocol::NetworkProtocol;
pub use transport::NetworkTransport;

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
