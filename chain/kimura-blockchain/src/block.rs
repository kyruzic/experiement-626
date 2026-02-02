use serde::{Deserialize, Serialize};

/// Block header containing metadata about the block
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BlockHeader {
    /// Block height (index in the chain)
    pub height: u64,
    /// Unix timestamp when block was created
    pub timestamp: u64,
    /// Hash of the previous block (32 bytes)
    pub prev_hash: [u8; 32],
    /// Merkle root of message IDs (placeholder for now, 32 bytes)
    pub message_root: [u8; 32],
}

impl BlockHeader {
    /// Create a new block header
    pub fn new(height: u64, timestamp: u64, prev_hash: [u8; 32], message_root: [u8; 32]) -> Self {
        Self {
            height,
            timestamp,
            prev_hash,
            message_root,
        }
    }

    /// Get the genesis block header (height 0, prev_hash = zeros)
    pub fn genesis() -> Self {
        Self {
            height: 0,
            timestamp: 0,            // Genesis has timestamp 0
            prev_hash: [0u8; 32],    // All zeros for genesis
            message_root: [0u8; 32], // All zeros for genesis
        }
    }
}

/// Complete block with header and message references
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Block {
    /// Block header with metadata
    pub header: BlockHeader,
    /// List of message IDs (32-byte hashes) included in this block
    pub message_ids: Vec<[u8; 32]>,
}

/// Hash result containing the hash bytes
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hash([u8; 32]);

impl Hash {
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

impl Block {
    /// Create a new block
    pub fn new(header: BlockHeader, message_ids: Vec<[u8; 32]>) -> Self {
        Self {
            header,
            message_ids,
        }
    }

    /// Create the genesis block (height 0)
    pub fn genesis() -> Self {
        Self {
            header: BlockHeader::genesis(),
            message_ids: vec![],
        }
    }

    /// Calculate the block hash using blake3
    /// Hashes: height || timestamp || prev_hash || message_root || len(message_ids) || message_ids
    pub fn hash(&self) -> Hash {
        let mut hasher = blake3::Hasher::new();

        // Hash height (8 bytes, big-endian)
        hasher.update(&self.header.height.to_be_bytes());

        // Hash timestamp (8 bytes, big-endian)
        hasher.update(&self.header.timestamp.to_be_bytes());

        // Hash prev_hash (32 bytes)
        hasher.update(&self.header.prev_hash);

        // Hash message_root (32 bytes)
        hasher.update(&self.header.message_root);

        // Hash number of messages (8 bytes, big-endian)
        hasher.update(&(self.message_ids.len() as u64).to_be_bytes());

        // Hash each message ID
        for msg_id in &self.message_ids {
            hasher.update(msg_id);
        }

        let result = hasher.finalize();
        let mut hash_bytes = [0u8; 32];
        hash_bytes.copy_from_slice(result.as_bytes());
        Hash::new(hash_bytes)
    }

    /// Verify this block against the previous block
    /// Checks:
    /// 1. Height is previous height + 1
    /// 2. prev_hash matches the hash of the previous block
    pub fn verify(&self, prev_block: &Block) -> Result<(), BlockError> {
        // Check height continuity
        let expected_height = prev_block.header.height + 1;
        if self.header.height != expected_height {
            return Err(BlockError::InvalidHeight {
                expected: expected_height,
                actual: self.header.height,
            });
        }

        // Check prev_hash matches previous block's hash
        let prev_hash = prev_block.hash();
        if self.header.prev_hash != *prev_hash.as_bytes() {
            return Err(BlockError::InvalidPrevHash);
        }

        Ok(())
    }

    /// Verify this block against a known previous hash
    /// Used when we only have the hash, not the full block
    pub fn verify_with_hash(
        &self,
        prev_hash: &[u8; 32],
        expected_height: u64,
    ) -> Result<(), BlockError> {
        // Check height
        if self.header.height != expected_height {
            return Err(BlockError::InvalidHeight {
                expected: expected_height,
                actual: self.header.height,
            });
        }

        // Check prev_hash
        if self.header.prev_hash != *prev_hash {
            return Err(BlockError::InvalidPrevHash);
        }

        Ok(())
    }
}

/// Errors that can occur during block validation
#[derive(Debug, thiserror::Error)]
pub enum BlockError {
    #[error("invalid block height: expected {expected}, got {actual}")]
    InvalidHeight { expected: u64, actual: u64 },

    #[error("previous hash mismatch")]
    InvalidPrevHash,

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genesis_block() {
        let genesis = Block::genesis();
        assert_eq!(genesis.header.height, 0);
        assert_eq!(genesis.header.prev_hash, [0u8; 32]);
        assert_eq!(genesis.header.message_root, [0u8; 32]);
        assert!(genesis.message_ids.is_empty());
    }

    #[test]
    fn test_block_hash_deterministic() {
        let block = Block::new(
            BlockHeader::new(1, 1000, [0u8; 32], [0u8; 32]),
            vec![[1u8; 32], [2u8; 32]],
        );

        let hash1 = block.hash();
        let hash2 = block.hash();

        // Hash should be deterministic
        assert_eq!(hash1.as_bytes(), hash2.as_bytes());
    }

    #[test]
    fn test_block_hash_changes_with_data() {
        let block1 = Block::new(BlockHeader::new(1, 1000, [0u8; 32], [0u8; 32]), vec![]);

        let block2 = Block::new(
            BlockHeader::new(1, 1000, [0u8; 32], [0u8; 32]),
            vec![[1u8; 32]],
        );

        let hash1 = block1.hash();
        let hash2 = block2.hash();

        // Different data should produce different hashes
        assert_ne!(hash1.as_bytes(), hash2.as_bytes());
    }

    #[test]
    fn test_verify_valid_chain() {
        let genesis = Block::genesis();
        let genesis_hash = genesis.hash();

        let block1 = Block::new(
            BlockHeader::new(1, 1000, *genesis_hash.as_bytes(), [0u8; 32]),
            vec![],
        );

        // Should verify successfully
        assert!(block1.verify(&genesis).is_ok());
    }

    #[test]
    fn test_verify_invalid_height() {
        let genesis = Block::genesis();
        let genesis_hash = genesis.hash();

        // Block with wrong height (should be 1, not 2)
        let invalid_block = Block::new(
            BlockHeader::new(2, 1000, *genesis_hash.as_bytes(), [0u8; 32]),
            vec![],
        );

        let result = invalid_block.verify(&genesis);
        assert!(matches!(
            result,
            Err(BlockError::InvalidHeight {
                expected: 1,
                actual: 2
            })
        ));
    }

    #[test]
    fn test_verify_invalid_prev_hash() {
        let genesis = Block::genesis();

        // Block with wrong prev_hash
        let invalid_block = Block::new(
            BlockHeader::new(1, 1000, [1u8; 32], [0u8; 32]), // Wrong prev_hash
            vec![],
        );

        let result = invalid_block.verify(&genesis);
        assert!(matches!(result, Err(BlockError::InvalidPrevHash)));
    }

    #[test]
    fn test_verify_with_hash() {
        let genesis = Block::genesis();
        let genesis_hash = genesis.hash();

        let block1 = Block::new(
            BlockHeader::new(1, 1000, *genesis_hash.as_bytes(), [0u8; 32]),
            vec![],
        );

        // Should verify with just the hash
        assert!(block1.verify_with_hash(genesis_hash.as_bytes(), 1).is_ok());

        // Should fail with wrong height
        assert!(block1.verify_with_hash(genesis_hash.as_bytes(), 2).is_err());

        // Should fail with wrong hash
        assert!(block1.verify_with_hash(&[1u8; 32], 1).is_err());
    }

    #[test]
    fn test_hash_to_hex() {
        let block = Block::genesis();
        let hash = block.hash();

        let hex_string = hash.to_hex();
        assert_eq!(hex_string.len(), 64); // 32 bytes * 2 hex chars per byte

        // Should be valid hex
        assert!(hex::decode(&hex_string).is_ok());
    }
}
