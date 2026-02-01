pub mod database;
pub mod store;
pub mod cache;

pub use database::RocksDB;
pub use store::BlockStore;
pub use cache::Cache;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rocksdb() {
        // TODO: Implement test
    }

    #[test]
    fn test_block_store() {
        // TODO: Implement test
    }

    #[test]
    fn test_cache() {
        // TODO: Implement test
    }
}