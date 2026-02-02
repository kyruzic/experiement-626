use serde::{Deserialize, Serialize};

/// A message in the blockchain
/// Message ID is calculated as: blake3(sender ++ nonce)
/// TODO(M3): Update to use actual sender public key bytes instead of string
/// Current implementation uses sender string for simplicity in M1/M2
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Message {
    /// Message ID (32-byte blake3 hash)
    pub id: [u8; 32],
    /// Sender's public key (used for verification in M3)
    pub sender: String,
    /// Message content (arbitrary string)
    pub content: String,
    /// Unix timestamp when message was created
    pub timestamp: u64,
    /// Nonce for replay protection (must be unique per sender)
    pub nonce: u64,
}

impl Message {
    /// Create a new message
    /// The ID is calculated from sender and nonce
    pub fn new(sender: String, content: String, timestamp: u64, nonce: u64) -> Self {
        let id = Self::calculate_id(&sender, nonce);

        Self {
            id,
            sender,
            content,
            timestamp,
            nonce,
        }
    }

    /// Calculate message ID: blake3(sender ++ nonce)
    /// TODO(M3): Update to use actual sender public key bytes for cryptographic security
    /// Current implementation uses sender string for simplicity in M1/M2
    pub fn calculate_id(sender: &str, nonce: u64) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();

        // Hash sender public key (or string for now)
        hasher.update(sender.as_bytes());

        // Hash nonce (8 bytes, big-endian)
        hasher.update(&nonce.to_be_bytes());

        let result = hasher.finalize();
        let mut id = [0u8; 32];
        id.copy_from_slice(result.as_bytes());
        id
    }

    /// Verify that the message ID matches the calculated value
    pub fn verify_id(&self) -> bool {
        let calculated_id = Self::calculate_id(&self.sender, self.nonce);
        self.id == calculated_id
    }

    /// Get message ID as hex string
    pub fn id_hex(&self) -> String {
        hex::encode(self.id)
    }

    /// Create a simple text message (convenience method)
    pub fn text(sender: &str, content: &str, nonce: u64) -> Self {
        Self::new(
            sender.to_string(),
            content.to_string(),
            chrono::Utc::now().timestamp() as u64,
            nonce,
        )
    }
}

/// Pending message waiting to be included in a block
/// Used by the leader to queue messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingMessage {
    /// The message itself
    pub message: Message,
    /// When the message was received
    pub received_at: u64,
}

impl PendingMessage {
    pub fn new(message: Message) -> Self {
        Self {
            received_at: chrono::Utc::now().timestamp() as u64,
            message,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = Message::new("Alice".to_string(), "Hello, World!".to_string(), 1000, 0);

        assert_eq!(msg.sender, "Alice");
        assert_eq!(msg.content, "Hello, World!");
        assert_eq!(msg.timestamp, 1000);
        assert_eq!(msg.nonce, 0);
    }

    #[test]
    fn test_message_id_deterministic() {
        // Same sender + nonce should produce same ID
        let id1 = Message::calculate_id("Alice", 42);
        let id2 = Message::calculate_id("Alice", 42);

        assert_eq!(id1, id2);
    }

    #[test]
    fn test_message_id_unique() {
        // Different nonces should produce different IDs
        let id1 = Message::calculate_id("Alice", 0);
        let id2 = Message::calculate_id("Alice", 1);

        assert_ne!(id1, id2);

        // Different senders should produce different IDs
        let id3 = Message::calculate_id("Bob", 0);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_verify_id_valid() {
        let msg = Message::new("Alice".to_string(), "Hello".to_string(), 1000, 5);

        assert!(msg.verify_id());
    }

    #[test]
    fn test_verify_id_invalid() {
        let mut msg = Message::new("Alice".to_string(), "Hello".to_string(), 1000, 5);

        // Tamper with the ID
        msg.id[0] = !msg.id[0];

        assert!(!msg.verify_id());
    }

    #[test]
    fn test_id_hex() {
        let msg = Message::text("Alice", "Hello", 0);
        let hex_id = msg.id_hex();

        // Should be 64 hex characters (32 bytes * 2)
        assert_eq!(hex_id.len(), 64);

        // Should be valid hex
        assert!(hex::decode(&hex_id).is_ok());
    }

    #[test]
    fn test_pending_message() {
        let msg = Message::text("Alice", "Hello", 0);
        let pending = PendingMessage::new(msg.clone());

        assert_eq!(pending.message.id, msg.id);
        assert!(pending.received_at > 0);
    }
}
