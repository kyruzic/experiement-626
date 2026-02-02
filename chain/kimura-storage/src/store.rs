use crate::database::{CF_BLOCKS, CF_MESSAGES, CF_METADATA, DatabaseError, RocksDB};
use serde::{Serialize, de::DeserializeOwned};
use std::sync::Arc;

/// Prefix for block keys (single byte like Geth)
const BLOCK_PREFIX: u8 = b'b';

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Database(#[from] DatabaseError),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("parse error: {0}")]
    ParseError(String),
    #[error("invalid data: {0}")]
    InvalidData(String),
}

/// Encode a block height into a 9-byte key: [prefix][8-byte BE height]
/// This ensures lexicographic ordering matches numeric ordering
fn encode_block_key(height: u64) -> Vec<u8> {
    let mut key = vec![BLOCK_PREFIX];
    key.extend_from_slice(&height.to_be_bytes());
    key
}

/// Decode a block key (9 bytes) back to height
/// Returns None if key is invalid
fn decode_block_key(key: &[u8]) -> Option<u64> {
    if key.len() != 9 || key[0] != BLOCK_PREFIX {
        return None;
    }
    let height_bytes: [u8; 8] = key[1..9].try_into().ok()?;
    Some(u64::from_be_bytes(height_bytes))
}

pub struct BlockStore {
    db: Arc<RocksDB>,
}

impl BlockStore {
    pub fn new(db: Arc<RocksDB>) -> Self {
        Self { db }
    }

    pub fn put_block<T: Serialize>(&self, height: u64, block: &T) -> Result<(), StorageError> {
        let key = encode_block_key(height);
        let value = serde_json::to_vec(block)?;
        self.db.put(CF_BLOCKS, &key, &value)?;
        Ok(())
    }

    pub fn get_block<T: DeserializeOwned>(&self, height: u64) -> Result<Option<T>, StorageError> {
        let key = encode_block_key(height);
        match self.db.get(CF_BLOCKS, &key)? {
            Some(data) => {
                let block = serde_json::from_slice(&data)?;
                Ok(Some(block))
            }
            None => Ok(None),
        }
    }

    pub fn get_latest_height(&self) -> Result<u64, StorageError> {
        let cf = self
            .db
            .inner()
            .cf_handle(CF_BLOCKS)
            .ok_or_else(|| DatabaseError::ColumnFamilyNotFound(CF_BLOCKS.to_string()))?;

        let mut iter = self.db.inner().raw_iterator_cf(cf);
        iter.seek_to_last();

        if let Some((key, _)) = iter.item() {
            if let Some(height) = decode_block_key(key) {
                return Ok(height);
            }
        }

        Ok(0)
    }

    /// Get blocks in a range [start, end] inclusive
    /// Returns Vec<(height, block)>
    pub fn get_blocks_range<T: DeserializeOwned>(
        &self,
        start: u64,
        end: u64,
    ) -> Result<Vec<(u64, T)>, StorageError> {
        let cf = self
            .db
            .inner()
            .cf_handle(CF_BLOCKS)
            .ok_or_else(|| DatabaseError::ColumnFamilyNotFound(CF_BLOCKS.to_string()))?;

        let mut iter = self.db.inner().raw_iterator_cf(cf);
        let start_key = encode_block_key(start);
        iter.seek(&start_key);

        let mut results = Vec::new();
        while let Some((key, value)) = iter.item() {
            if let Some(height) = decode_block_key(key) {
                if height > end {
                    break;
                }
                let block: T = serde_json::from_slice(value)?;
                results.push((height, block));
            }
            iter.next();
        }

        Ok(results)
    }
}

pub struct MessageStore {
    db: Arc<RocksDB>,
}

impl MessageStore {
    pub fn new(db: Arc<RocksDB>) -> Self {
        Self { db }
    }

    pub fn put_message<T: Serialize>(
        &self,
        id: &[u8; 32],
        message: &T,
    ) -> Result<(), StorageError> {
        let key = format!("msg:{}", hex::encode(id));
        let value = serde_json::to_vec(message)?;
        self.db.put(CF_MESSAGES, key.as_bytes(), &value)?;
        Ok(())
    }

    pub fn get_message<T: DeserializeOwned>(
        &self,
        id: &[u8; 32],
    ) -> Result<Option<T>, StorageError> {
        let key = format!("msg:{}", hex::encode(id));
        match self.db.get(CF_MESSAGES, key.as_bytes())? {
            Some(data) => {
                let message = serde_json::from_slice(&data)?;
                Ok(Some(message))
            }
            None => Ok(None),
        }
    }
}

pub struct MetadataStore {
    db: Arc<RocksDB>,
}

impl MetadataStore {
    pub fn new(db: Arc<RocksDB>) -> Self {
        Self { db }
    }

    pub fn get_last_height(&self) -> Result<Option<u64>, StorageError> {
        match self.db.get(CF_METADATA, b"meta:last_height")? {
            Some(data) => {
                let height =
                    u64::from_be_bytes(data.try_into().map_err(|_| {
                        StorageError::InvalidData("invalid height bytes".to_string())
                    })?);
                Ok(Some(height))
            }
            None => Ok(None),
        }
    }

    pub fn set_last_height(&self, height: u64) -> Result<(), StorageError> {
        self.db
            .put(CF_METADATA, b"meta:last_height", &height.to_be_bytes())?;
        Ok(())
    }

    pub fn get_last_hash(&self) -> Result<Option<[u8; 32]>, StorageError> {
        match self.db.get(CF_METADATA, b"meta:last_hash")? {
            Some(data) => {
                let hash: [u8; 32] = data
                    .try_into()
                    .map_err(|_| StorageError::InvalidData("invalid hash length".to_string()))?;
                Ok(Some(hash))
            }
            None => Ok(None),
        }
    }

    pub fn set_last_hash(&self, hash: &[u8; 32]) -> Result<(), StorageError> {
        self.db.put(CF_METADATA, b"meta:last_hash", hash)?;
        Ok(())
    }

    pub fn get_genesis_hash(&self) -> Result<Option<[u8; 32]>, StorageError> {
        match self.db.get(CF_METADATA, b"meta:genesis_hash")? {
            Some(data) => {
                let hash: [u8; 32] = data
                    .try_into()
                    .map_err(|_| StorageError::InvalidData("invalid hash length".to_string()))?;
                Ok(Some(hash))
            }
            None => Ok(None),
        }
    }

    pub fn set_genesis_hash(&self, hash: &[u8; 32]) -> Result<(), StorageError> {
        self.db.put(CF_METADATA, b"meta:genesis_hash", hash)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use tempfile::TempDir;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestBlock {
        height: u64,
        hash: String,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestMessage {
        content: String,
        sender: String,
    }

    fn setup_test_db() -> (TempDir, Arc<RocksDB>) {
        let tmp_dir = TempDir::new().unwrap();
        let db = Arc::new(RocksDB::new(tmp_dir.path()).unwrap());
        (tmp_dir, db)
    }

    #[test]
    fn test_block_key_encoding() {
        // Test that height 1 and 10 sort correctly
        let key_1 = encode_block_key(1);
        let key_10 = encode_block_key(10);
        let key_2 = encode_block_key(2);

        // Verify keys are 9 bytes: 1 prefix + 8 bytes
        assert_eq!(key_1.len(), 9);
        assert_eq!(key_10.len(), 9);

        // Verify lexicographic order matches numeric order
        assert!(key_1 < key_2);
        assert!(key_2 < key_10);

        // Verify decoding works
        assert_eq!(decode_block_key(&key_1), Some(1));
        assert_eq!(decode_block_key(&key_10), Some(10));
        assert_eq!(decode_block_key(&key_2), Some(2));
    }

    #[test]
    fn test_block_key_max_value() {
        let key_max = encode_block_key(u64::MAX);
        assert_eq!(decode_block_key(&key_max), Some(u64::MAX));
    }

    #[test]
    fn test_decode_invalid_key() {
        // Wrong prefix
        let mut wrong_prefix = vec![b'x'];
        wrong_prefix.extend_from_slice(&1u64.to_be_bytes());
        assert_eq!(decode_block_key(&wrong_prefix), None);

        // Too short
        assert_eq!(decode_block_key(&[b'b']), None);

        // Too long
        let mut too_long = encode_block_key(1);
        too_long.push(0);
        assert_eq!(decode_block_key(&too_long), None);
    }

    #[test]
    fn test_block_store_put_and_get() {
        let (_tmp, db) = setup_test_db();
        let store = BlockStore::new(db);

        let block = TestBlock {
            height: 1,
            hash: "abc123".to_string(),
        };

        store.put_block(1, &block).unwrap();
        let retrieved = store.get_block::<TestBlock>(1).unwrap();

        assert_eq!(retrieved, Some(block));
    }

    #[test]
    fn test_block_store_get_nonexistent() {
        let (_tmp, db) = setup_test_db();
        let store = BlockStore::new(db);

        let result = store.get_block::<TestBlock>(999).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_get_latest_height() {
        let (_tmp, db) = setup_test_db();
        let store = BlockStore::new(db);

        // Empty database returns 0
        assert_eq!(store.get_latest_height().unwrap(), 0);

        // Add blocks 1, 2, 5, 10
        let block = TestBlock {
            height: 0,
            hash: "test".to_string(),
        };

        store.put_block(1, &block).unwrap();
        assert_eq!(store.get_latest_height().unwrap(), 1);

        store.put_block(2, &block).unwrap();
        assert_eq!(store.get_latest_height().unwrap(), 2);

        // Test the critical bug case: 10 should be > 2
        store.put_block(10, &block).unwrap();
        assert_eq!(store.get_latest_height().unwrap(), 10);

        store.put_block(5, &block).unwrap();
        // Should still be 10 (largest)
        assert_eq!(store.get_latest_height().unwrap(), 10);
    }

    #[test]
    fn test_get_blocks_range() {
        let (_tmp, db) = setup_test_db();
        let store = BlockStore::new(db);

        // Add blocks 1-5
        for i in 1..=5 {
            let block = TestBlock {
                height: i,
                hash: format!("hash{}", i),
            };
            store.put_block(i, &block).unwrap();
        }

        // Get range [2, 4]
        let range = store.get_blocks_range::<TestBlock>(2, 4).unwrap();
        assert_eq!(range.len(), 3);
        assert_eq!(range[0].0, 2);
        assert_eq!(range[1].0, 3);
        assert_eq!(range[2].0, 4);

        // Check block data
        assert_eq!(range[0].1.hash, "hash2");
        assert_eq!(range[2].1.hash, "hash4");
    }

    #[test]
    fn test_get_blocks_range_empty() {
        let (_tmp, db) = setup_test_db();
        let store = BlockStore::new(db);

        let range = store.get_blocks_range::<TestBlock>(1, 10).unwrap();
        assert!(range.is_empty());
    }

    #[test]
    fn test_message_store_put_and_get() {
        let (_tmp, db) = setup_test_db();
        let store = MessageStore::new(db);

        let message = TestMessage {
            content: "Hello".to_string(),
            sender: "Alice".to_string(),
        };
        let id = [1u8; 32];

        store.put_message(&id, &message).unwrap();
        let retrieved = store.get_message::<TestMessage>(&id).unwrap();

        assert_eq!(retrieved, Some(message));
    }

    #[test]
    fn test_metadata_store_height() {
        let (_tmp, db) = setup_test_db();
        let store = MetadataStore::new(db);

        assert_eq!(store.get_last_height().unwrap(), None);

        store.set_last_height(42).unwrap();
        assert_eq!(store.get_last_height().unwrap(), Some(42));
    }

    #[test]
    fn test_metadata_store_hash() {
        let (_tmp, db) = setup_test_db();
        let store = MetadataStore::new(db);

        let hash = [0xAB; 32];
        store.set_last_hash(&hash).unwrap();

        let retrieved = store.get_last_hash().unwrap();
        assert_eq!(retrieved, Some(hash));
    }

    #[test]
    fn test_metadata_store_genesis_hash() {
        let (_tmp, db) = setup_test_db();
        let store = MetadataStore::new(db);

        let hash = [0x00; 32];
        store.set_genesis_hash(&hash).unwrap();

        let retrieved = store.get_genesis_hash().unwrap();
        assert_eq!(retrieved, Some(hash));
    }
}
