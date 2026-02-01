pub mod config;
pub mod node;
pub mod services;

pub use config::NodeConfig;
pub use node::Node;
pub use services::NodeServices;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_creation() {
        // TODO: Implement test
    }

    #[test]
    fn test_node_config() {
        // TODO: Implement test
    }

    #[test]
    fn test_services_initialization() {
        // TODO: Implement test
    }
}
