use crate::database::{DatabaseError, RocksDB, CF_BLOCKS, CF_MESSAGES, CF_METADATA};
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;

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

pub struct BlockStore {
    db: Arc<RocksDB>,
}

impl BlockStore {
    pub fn new(db: Arc<RocksDB>) -> Self {
        Self { db }
    }

    pub fn put_block<T: Serialize>(&self, height: u64, block: &T) -> Result<(), StorageError> {
        let key = format!("block:{}", height);
        let value = serde_json::to_vec(block)?;
        self.db.put(CF_BLOCKS, key.as_bytes(), &value)?;
        Ok(())
    }

    pub fn get_block<T: DeserializeOwned>(&self, height: u64) -> Result<Option<T>, StorageError> {
        let key = format!("block:{}", height);
        match self.db.get(CF_BLOCKS, key.as_bytes())? {
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
            let key_str = String::from_utf8_lossy(key);
            if let Some(height_str) = key_str.strip_prefix("block:") {
                return height_str
                    .parse()
                    .map_err(|_| StorageError::ParseError("invalid height in key".to_string()));
            }
        }

        Ok(0)
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
