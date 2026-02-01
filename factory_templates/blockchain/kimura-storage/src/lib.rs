pub mod cache;
pub mod database;
pub mod store;

pub use cache::Cache;
pub use database::RocksDB;
pub use store::BlockStore;

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
