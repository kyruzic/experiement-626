# Milestone 1 Implementation Plan: Block Production & Propagation

**Status:** In Progress  
**Duration:** Week 1-2 (14 days)  
**Goal:** Produce a block and have it propagate to peers

---

## Overview

This document tracks the implementation of Milestone 1 for the Kimura blockchain. Each phase includes specific deliverables that will be marked as complete when finished.

---

## Phase 1: Storage Layer (Days 1-2) - `kimura-storage`

**Status:** ✅ COMPLETED

**Deliverables:**
- [x] `RocksDB` wrapper struct with 4 column families (blocks, messages, metadata, pending)
- [x] `BlockStore` with `put_block(height, block)` and `get_block(height)` methods
- [x] `MessageStore` with `put_message(id, message)` and `get_message(id)` methods
- [x] `MetadataStore` with `get_last_height()`, `set_last_height()`, `get_last_hash()`, `set_last_hash()`
- [x] Batch write support using RocksDB WriteBatch for atomic operations
- [x] Unit tests for all storage operations (12 tests passing)

**Files to modify:**
- `kimura-storage/src/database.rs`
- `kimura-storage/src/store.rs`
- `kimura-storage/src/lib.rs`

**Commit message:** "Add RocksDB storage layer with column families"

---

## Phase 2: Core Blockchain (Days 3-4) - `kimura-blockchain`

**Status:** ✅ COMPLETED

**Deliverables:**
- [x] `BlockHeader` struct with fields: height, timestamp, prev_hash [u8; 32], message_root [u8; 32]
- [x] `Block` struct with header and message_ids: Vec<[u8; 32]>
- [x] `Message` struct with id, sender, content, timestamp, nonce
- [x] `Block::hash()` implementation using blake3
- [x] `Block::verify(prev_block)` method checking prev_hash and height continuity
- [x] Hardcoded genesis block (height 0, prev_hash = 0x000...)
- [x] Unit tests for block creation, hashing, and validation (15 tests passing)

**Files to modify:**
- `kimura-blockchain/src/block.rs`
- `kimura-blockchain/src/transaction.rs` (rename to message.rs or adapt)
- `kimura-blockchain/src/chain.rs`
- `kimura-blockchain/src/lib.rs`

**Commit message:** "Add block structure and validation"

---

## Phase 3: Network Layer (Days 5-7) - `kimura-network`

**Status:** ✅ COMPLETED

**Deliverables:**
- [x] `NetworkError` enum with proper error types (PublishError, SubscribeError, DialError, SerializationError)
- [x] `NetworkConfig` struct with listen_addr and leader_addr
- [x] Ephemeral peer identity (Ed25519 keypair generated on startup)
- [x] `P2PNetwork::new()` with libp2p transport (TCP + Noise + Yamux)
- [x] Gossipsub protocol configured with "kimura/blocks/1.0.0" topic
- [x] `NetworkEvent` enum (BlockReceived, PeerConnected, PeerDisconnected)
- [x] `P2PNetwork::subscribe_blocks()` returning Stream<NetworkEvent>
- [x] `P2PNetwork::publish_block(block)` for JSON serialization and publishing
- [x] `P2PNetwork::dial_leader()` for connecting to configured leader
- [x] Unit tests for config and network creation (integration tests marked #[ignore])

**Files modified:**
- `kimura-network/src/p2p.rs` - Complete P2P implementation
- `kimura-network/src/lib.rs` - Updated exports
- `kimura-network/src/transport.rs` - Placeholder
- `kimura-network/src/protocol.rs` - Placeholder
- `Cargo.toml` - Added "tokio" feature to libp2p

**Commit message:** "Add libp2p gossipsub networking"

---

## Phase 4: Node Binary (Days 8-10) - `kimura-node`

**Status:** Not Started

**Deliverables:**
- [ ] Config struct: `is_leader`, `leader_address`, `listen_addr`, `db_path`
- [ ] Leader mode implementation:
  - [ ] 5-second timer using tokio::time::interval
  - [ ] Collect pending messages from "pending" column family
  - [ ] Create new block with message IDs
  - [ ] Save block to DB atomically
  - [ ] Publish block via gossipsub
- [ ] Peer mode implementation:
  - [ ] Connect to leader
  - [ ] Subscribe to "blocks" topic
  - [ ] Validate received blocks (continuity check)
  - [ ] Save valid blocks to DB
- [ ] Graceful shutdown handling with metadata save

**Files to modify:**
- `kimura-node/src/config.rs`
- `kimura-node/src/node.rs`
- `kimura-node/src/services.rs`
- `kimura-node/src/main.rs`

**Commit message:** "Add leader/peer node implementation"

---

## Phase 5: Integration Testing (Days 11-14)

**Status:** Not Started

**Deliverables:**
- [ ] Test: Start leader, verify produces blocks every 5 seconds
- [ ] Test: Start peer, verify receives and stores blocks
- [ ] Test: 3-node network (1 leader, 2 peers) - all nodes receive all blocks
- [ ] Test: Chain continuity validation (reject blocks with invalid prev_hash)
- [ ] Test: Graceful shutdown and restart scenario
- [ ] Update documentation with M1 details

**Files to modify:**
- `tests/service/integration.rs`
- `tests/mod.rs`

**Commit message:** "Add integration tests for M1"

---

## Definition of Done

- [ ] `python3 blockchain.py build --mode release` succeeds without errors
- [ ] Leader produces a new block every 5 seconds consistently
- [ ] Peers receive blocks via gossipsub within 1 second
- [ ] All unit tests pass (`python3 blockchain.py test --suite unit`)
- [ ] All integration tests pass (`python3 blockchain.py test --suite integration`)
- [ ] No compiler warnings
- [ ] Clean clippy linting

---

## Progress Log

| Date | Phase | Deliverable | Status |
|------|-------|-------------|--------|
| TBD  | Phase 1 | RocksDB wrapper | Not Started |
| TBD  | Phase 1 | BlockStore | Not Started |
| TBD  | Phase 1 | MessageStore | Not Started |
| TBD  | Phase 1 | MetadataStore | Not Started |
| TBD  | Phase 1 | Batch writes | Not Started |
| TBD  | Phase 1 | Unit tests | Not Started |
| TBD  | Phase 2 | BlockHeader struct | Not Started |
| TBD  | Phase 2 | Block struct | Not Started |
| TBD  | Phase 2 | Message struct | Not Started |
| TBD  | Phase 2 | blake3 hashing | Not Started |
| TBD  | Phase 2 | Block verification | Not Started |
| TBD  | Phase 2 | Genesis block | Not Started |
| TBD  | Phase 2 | Unit tests | Not Started |
| TBD  | Phase 3 | P2PNetwork::new() | Not Started |
| TBD  | Phase 3 | Gossipsub setup | Not Started |
| TBD  | Phase 3 | publish_block() | Not Started |
| TBD  | Phase 3 | subscribe_blocks() | Not Started |
| TBD  | Phase 3 | Leader config | Not Started |
| TBD  | Phase 3 | Unit tests | Not Started |
| TBD  | Phase 4 | Config struct | Not Started |
| TBD  | Phase 4 | Leader mode (timer) | Not Started |
| TBD  | Phase 4 | Leader mode (collect) | Not Started |
| TBD  | Phase 4 | Leader mode (create) | Not Started |
| TBD  | Phase 4 | Leader mode (save) | Not Started |
| TBD  | Phase 4 | Leader mode (publish) | Not Started |
| TBD  | Phase 4 | Peer mode (connect) | Not Started |
| TBD  | Phase 4 | Peer mode (subscribe) | Not Started |
| TBD  | Phase 4 | Peer mode (validate) | Not Started |
| TBD  | Phase 4 | Peer mode (save) | Not Started |
| TBD  | Phase 4 | Shutdown handling | Not Started |
| TBD  | Phase 5 | Leader test | Not Started |
| TBD  | Phase 5 | Peer test | Not Started |
| TBD  | Phase 5 | 3-node test | Not Started |
| TBD  | Phase 5 | Continuity test | Not Started |
| TBD  | Phase 5 | Restart test | Not Started |
| TBD  | Phase 5 | Documentation | Not Started |

---

## Notes

- Build incrementally and test each component as it's written
- Use `python3 blockchain.py build --mode release` to verify compilation
- Use `python3 blockchain.py test --suite unit` to run unit tests
- Commit meaningful chunks with descriptive messages
- Update this document as deliverables are completed

## Next Milestone Preview

**M2: Persistence & Restart (Week 3)**
- Complete persistence to all column families
- Implement restart logic (load metadata, resume from last block)
- Test crash recovery scenarios
