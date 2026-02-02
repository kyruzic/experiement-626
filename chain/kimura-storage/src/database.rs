use rocksdb::{ColumnFamilyDescriptor, DB, Options};
use std::path::Path;
use thiserror::Error;

pub const CF_BLOCKS: &str = "blocks";
pub const CF_MESSAGES: &str = "messages";
pub const CF_METADATA: &str = "metadata";
pub const CF_PENDING: &str = "pending";

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("rocksdb error: {0}")]
    RocksDB(#[from] rocksdb::Error),
    #[error("column family not found: {0}")]
    ColumnFamilyNotFound(String),
}

pub struct RocksDB {
    db: DB,
}

impl RocksDB {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, DatabaseError> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let cfs = vec![
            ColumnFamilyDescriptor::new(CF_BLOCKS, Options::default()),
            ColumnFamilyDescriptor::new(CF_MESSAGES, Options::default()),
            ColumnFamilyDescriptor::new(CF_METADATA, Options::default()),
            ColumnFamilyDescriptor::new(CF_PENDING, Options::default()),
        ];

        let db = DB::open_cf_descriptors(&opts, path, cfs)?;

        Ok(Self { db })
    }

    pub fn put(&self, cf_name: &str, key: &[u8], value: &[u8]) -> Result<(), DatabaseError> {
        let cf = self
            .db
            .cf_handle(cf_name)
            .ok_or_else(|| DatabaseError::ColumnFamilyNotFound(cf_name.to_string()))?;
        self.db.put_cf(cf, key, value)?;
        Ok(())
    }

    pub fn get(&self, cf_name: &str, key: &[u8]) -> Result<Option<Vec<u8>>, DatabaseError> {
        let cf = self
            .db
            .cf_handle(cf_name)
            .ok_or_else(|| DatabaseError::ColumnFamilyNotFound(cf_name.to_string()))?;
        let value = self.db.get_cf(cf, key)?;
        Ok(value)
    }

    pub fn delete(&self, cf_name: &str, key: &[u8]) -> Result<(), DatabaseError> {
        let cf = self
            .db
            .cf_handle(cf_name)
            .ok_or_else(|| DatabaseError::ColumnFamilyNotFound(cf_name.to_string()))?;
        self.db.delete_cf(cf, key)?;
        Ok(())
    }

    pub fn batch_write(&self, batch: rocksdb::WriteBatch) -> Result<(), DatabaseError> {
        self.db.write(batch)?;
        Ok(())
    }

    pub fn inner(&self) -> &DB {
        &self.db
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_rocksdb_open() {
        let tmp_dir = TempDir::new().unwrap();
        let db = RocksDB::new(tmp_dir.path());
        assert!(db.is_ok());
    }

    #[test]
    fn test_put_and_get() {
        let tmp_dir = TempDir::new().unwrap();
        let db = RocksDB::new(tmp_dir.path()).unwrap();

        db.put(CF_BLOCKS, b"key1", b"value1").unwrap();
        let result = db.get(CF_BLOCKS, b"key1").unwrap();

        assert_eq!(result, Some(b"value1".to_vec()));
    }

    #[test]
    fn test_get_nonexistent() {
        let tmp_dir = TempDir::new().unwrap();
        let db = RocksDB::new(tmp_dir.path()).unwrap();

        let result = db.get(CF_BLOCKS, b"nonexistent").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_delete() {
        let tmp_dir = TempDir::new().unwrap();
        let db = RocksDB::new(tmp_dir.path()).unwrap();

        db.put(CF_BLOCKS, b"key1", b"value1").unwrap();
        db.delete(CF_BLOCKS, b"key1").unwrap();

        let result = db.get(CF_BLOCKS, b"key1").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_multiple_column_families() {
        let tmp_dir = TempDir::new().unwrap();
        let db = RocksDB::new(tmp_dir.path()).unwrap();

        db.put(CF_BLOCKS, b"key", b"block_value").unwrap();
        db.put(CF_MESSAGES, b"key", b"message_value").unwrap();
        db.put(CF_METADATA, b"key", b"metadata_value").unwrap();
        db.put(CF_PENDING, b"key", b"pending_value").unwrap();

        assert_eq!(
            db.get(CF_BLOCKS, b"key").unwrap(),
            Some(b"block_value".to_vec())
        );
        assert_eq!(
            db.get(CF_MESSAGES, b"key").unwrap(),
            Some(b"message_value".to_vec())
        );
        assert_eq!(
            db.get(CF_METADATA, b"key").unwrap(),
            Some(b"metadata_value".to_vec())
        );
        assert_eq!(
            db.get(CF_PENDING, b"key").unwrap(),
            Some(b"pending_value".to_vec())
        );
    }

    #[test]
    fn test_batch_write() {
        let tmp_dir = TempDir::new().unwrap();
        let db = RocksDB::new(tmp_dir.path()).unwrap();

        let mut batch = rocksdb::WriteBatch::default();
        let cf = db.inner().cf_handle(CF_BLOCKS).unwrap();
        batch.put_cf(cf, b"key1", b"value1");
        batch.put_cf(cf, b"key2", b"value2");

        db.batch_write(batch).unwrap();

        assert_eq!(
            db.get(CF_BLOCKS, b"key1").unwrap(),
            Some(b"value1".to_vec())
        );
        assert_eq!(
            db.get(CF_BLOCKS, b"key2").unwrap(),
            Some(b"value2".to_vec())
        );
    }
}
