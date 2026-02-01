pub mod block;
pub mod chain;
pub mod transaction;

pub use block::Block;
pub use chain::Blockchain;
pub use transaction::Transaction;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_creation() {
        // TODO: Implement test
    }

    #[test]
    fn test_blockchain() {
        // TODO: Implement test
    }

    #[test]
    fn test_transaction() {
        // TODO: Implement test
    }
}
