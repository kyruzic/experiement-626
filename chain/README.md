# Kimura Blockchain

Rust blockchain implementation using libp2p and rocksdb.

## Project Structure

- `kimura-node/` - Main node executable
- `kimura-consensus/` - Consensus protocol
- `kimura-network/` - P2P networking (libp2p)
- `kimura-storage/` - Storage layer (rocksdb)
- `kimura-blockchain/` - Core blockchain logic

## Building

```bash
cargo build --release
```

## Testing

```bash
# Unit tests
cargo test

# Service tests
cargo test --test service
```