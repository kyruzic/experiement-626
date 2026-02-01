# Kimura Blockchain - MVP Implementation Plan

## Overview

A **single-leader messaging blockchain** using libp2p and RocksDB. One designated leader produces blocks containing messages every 5 seconds and pushes them to peers via gossipsub.

**Key Design Principles:**
- Simple, working implementation over perfect architecture
- Single leader (no consensus needed for MVP)
- Message-based (not value/transaction based)
- RocksDB persistence from day one
- MVP first, iterate later

---

## Architecture

### Single-Leader Model
- **One leader** designated via config flag
- Leader produces blocks every 5 seconds (fixed timer)
- Leader pushes blocks to peers via libp2p gossipsub
- Peers validate chain continuity (prev_hash checks)
- Any node can theoretically become leader (election logic deferred)

### Database Schema (RocksDB)

**Column Families:**

```
blocks:       "block:{height}" -> Block
messages:     "msg:{hash}"     -> Message  
metadata:     "meta:last_height" -> u64
              "meta:last_hash"   -> Hash
              "meta:genesis_hash" -> Hash
pending:      "pending:{hash}"   -> Message (leader only)
```

**Message ID:** `blake3(sender_pubkey ++ nonce)`

**Genesis Block:** Hardcoded (hash = 0x000...)

### Block Structure

```rust
struct Block {
    header: BlockHeader {
        height: u64,
        timestamp: u64,
        prev_hash: [u8; 32],       // blake3
        message_root: [u8; 32],    // Merkle root (placeholder for now)
        producer_sig: Signature,   // secp256k1 (added in M3)
    },
    message_ids: Vec<[u8; 32]>,
}
```

### Network
- libp2p with gossipsub protocol
- Topic: "blocks"
- Hardcoded leader address in config
- No discovery (static configuration)

---

## Milestones

### M1: Block Production & Propagation (Week 1-2)

**Goal:** Produce a block and have it propagate to peers

#### Week 1

**Day 1-2: Storage Layer (kimura-storage)**
- [ ] Create `RocksDB` wrapper struct
- [ ] Define 4 column families (blocks, messages, metadata, pending)
- [ ] Implement `BlockStore` with `put_block(height, block)` and `get_block(height)`
- [ ] Implement `MessageStore` with `put_message(id, message)` and `get_message(id)`
- [ ] Implement `MetadataStore` with `get_last_height()`, `set_last_height()`, `get_last_hash()`, `set_last_hash()`
- [ ] Add batch write support (atomic operations)
- [ ] Write unit tests
- [ ] **Commit:** "Add RocksDB storage layer with column families"

**Day 3-4: Core Blockchain (kimura-blockchain)**
- [ ] Define `Block`, `BlockHeader`, `Message` structs with serde serialization
- [ ] Implement `Block::hash()` using blake3
- [ ] Implement `Block::verify(prev_block)` - check prev_hash and height continuity
- [ ] Create hardcoded genesis block (height 0, prev_hash = 0x000...)
- [ ] Implement basic chain validation
- [ ] Write unit tests
- [ ] **Commit:** "Add block structure and validation"

**Day 5-7: Network Layer (kimura-network)**
- [ ] Set up libp2p transport (TCP + Noise + Yamux)
- [ ] Configure gossipsub with "blocks" topic
- [ ] Implement `P2PNetwork::new()` with peer ID generation
- [ ] Implement `P2PNetwork::publish_block(block)` - serialize and publish
- [ ] Implement `P2PNetwork::subscribe_blocks()` - return stream of blocks
- [ ] Add config for leader address (hardcoded for MVP)
- [ ] Write tests with mock network
- [ ] **Commit:** "Add libp2p gossipsub networking"

#### Week 2

**Day 8-10: Node Binary (kimura-node)**
- [ ] Create config struct: `is_leader: bool`, `leader_address: String`, `listen_addr: String`, `db_path: String`
- [ ] Leader mode:
  - [ ] 5-second timer using tokio::time::interval
  - [ ] Collect pending messages from "pending" CF
  - [ ] Create block with message IDs
  - [ ] Save block to DB
  - [ ] Publish block via gossipsub
- [ ] Peer mode:
  - [ ] Connect to leader
  - [ ] Subscribe to "blocks" topic
  - [ ] Validate received blocks (continuity check)
  - [ ] Save valid blocks to DB
  - [ ] Accept messages via RPC (placeholder for M4)
- [ ] Graceful shutdown handling (save metadata)
- [ ] **Commit:** "Add leader/peer node implementation"

**Day 11-14: Integration Testing**
- [ ] Test: Start leader, produces blocks every 5s
- [ ] Test: Start peer, receives and stores blocks
- [ ] Test: 3-node network (1 leader, 2 peers)
- [ ] Test: Chain continuity validation
- [ ] Test: Graceful shutdown and restart
- [ ] Update documentation
- [ ] **Commit:** "Add integration tests for M1"

**M1 Definition of Done:**
- [ ] `python3 blockchain.py build --mode release` succeeds
- [ ] Leader produces block every 5 seconds
- [ ] Peers receive blocks via gossipsub
- [ ] All unit tests pass
- [ ] Integration tests pass

---

### M2: Persistence & Restart (Week 3)

**Goal:** Node can restart and resume from last block without data loss

**Day 15-17: Complete Persistence**
- [ ] Ensure all blocks saved to "blocks" CF atomically
- [ ] Ensure all messages saved to "messages" CF
- [ ] Save metadata after each block:
  - `meta:last_height` -> block height
  - `meta:last_hash` -> block hash
  - `meta:genesis_hash` -> genesis hash (for validation)
- [ ] Use RocksDB WriteBatch for atomic block + metadata writes
- [ ] Write crash recovery tests
- [ ] **Commit:** "Add complete persistence layer"

**Day 18-21: Restart Logic**
- [ ] On startup, load metadata:
  - [ ] Check if `meta:last_height` exists
  - [ ] Load last block from "blocks" CF using height
  - [ ] Verify `meta:last_hash` matches loaded block
- [ ] Leader mode:
  - [ ] Resume from `last_height + 1`
  - [ ] Continue 5s timer
- [ ] Peer mode:
  - [ ] Resume from last known block
  - [ ] Reconnect to leader
- [ ] Test scenarios:
  - [ ] Kill node mid-block-production, restart
  - [ ] Kill node after 100 blocks, restart
  - [ ] Verify no data loss
- [ ] **Commit:** "Add restart and recovery logic"

**M2 Definition of Done:**
- [ ] Can stop and restart leader, resumes from last block
- [ ] Can stop and restart peer, chain intact
- [ ] No data loss on unclean shutdown
- [ ] All integration tests pass

---

### M3: Signing & Verification (Week 4)

**Goal:** Cryptographic trust in messages and blocks

**Day 22-24: Message Signing**
- [ ] Add secp256k1 dependency
- [ ] Generate keypair on node startup (or load from file)
- [ ] Update `Message` struct:
  - Add `signature: Signature` field
  - Add `public_key: PublicKey` field
- [ ] Implement `Message::sign(private_key)`:
  - Sign `blake3(sender_pubkey ++ nonce ++ content)`
- [ ] Implement `Message::verify()`:
  - Verify signature against sender's public key
  - Verify message ID matches `blake3(pubkey ++ nonce)`
- [ ] Track nonce per sender in metadata CF
- [ ] Increment and save nonce after each message
- [ ] Write security tests
- [ ] **Commit:** "Add message signing with secp256k1"

**Day 25-27: Block Signing**
- [ ] Update `BlockHeader`:
  - Add `producer_pubkey: PublicKey`
  - Add `producer_sig: Signature`
- [ ] Leader signs block header before publishing:
  - Sign `blake3(height ++ timestamp ++ prev_hash ++ message_root)`
- [ ] Peers verify block signature on receive:
  - [ ] Check signature against leader's public key
  - [ ] Verify leader is authorized (config check)
- [ ] Validate all message signatures before adding to block
- [ ] Reject messages with invalid signatures
- [ ] **Commit:** "Add block signing and verification"

**Day 28: Security Testing**
- [ ] Test: Invalid message signature rejected
- [ ] Test: Tampered block detected (signature invalid)
- [ ] Test: Wrong sender public key detected
- [ ] Test: Replay attack prevented (nonce check)
- [ ] Test: Nonce must increment by 1
- [ ] Test: Out-of-order nonce rejected
- [ ] Update security documentation
- [ ] **Commit:** "Add security tests for M3"

**M3 Definition of Done:**
- [ ] All messages cryptographically signed
- [ ] All blocks signed by leader
- [ ] Signatures verified before acceptance
- [ ] Replay attacks prevented via nonces
- [ ] All security tests pass

---

### M4: JSON-RPC API & kimura.py CLI (Week 5)

**Goal:** External clients can interact with blockchain via HTTP API

**Day 29-31: HTTP JSON-RPC Server**
- [ ] Add HTTP server to kimura-node (port 8545)
- [ ] Implement JSON-RPC 2.0 request/response format
- [ ] Endpoint: `submit_message`:
  - Params: `{sender: String, content: String, signature: String, public_key: String, nonce: u64}`
  - Validate signature
  - Add to pending pool (or reject if leader is behind)
  - Return: `{message_id: String, status: String}`
- [ ] Endpoint: `get_block`:
  - Params: `{height: u64}`
  - Return: `{header: BlockHeader, message_ids: Vec<String>}`
- [ ] Endpoint: `get_height`:
  - Return: `{height: u64, hash: String}`
- [ ] Add CORS headers for browser clients
- [ ] Add rate limiting (basic)
- [ ] Write endpoint tests
- [ ] **Commit:** "Add JSON-RPC API"

**Day 32-33: kimura.py - RPC Testing CLI**
- [ ] Create `kimura.py` in chain/ directory
- [ ] Config: read RPC endpoint from file (default: http://localhost:8545)
- [ ] Command: `python3 kimura.py message send --content "hello" --sender "node1"`:
  - Load sender's private key from file
  - Sign message
  - Call `submit_message` RPC
  - Print message ID
- [ ] Command: `python3 kimura.py block get --height 42`:
  - Call `get_block` RPC
  - Pretty-print block info
- [ ] Command: `python3 kimura.py height get`:
  - Call `get_height` RPC
  - Print current chain height
- [ ] Error handling for connection issues
- [ ] **Commit:** "Add kimura.py RPC testing CLI"

**Day 34-35: End-to-End Testing**
- [ ] Start leader node with RPC enabled
- [ ] Test: Send 100 messages via kimura.py
- [ ] Test: Verify all messages appear in blocks
- [ ] Test: Query blocks via kimura.py
- [ ] Test: Restart node, verify RPC still works
- [ ] Performance test: Message throughput (target: 1000+ msg/block)
- [ ] Documentation: API reference in README.md
- [ ] Final **Commit:** "Complete MVP implementation"

**M4 Definition of Done:**
- [ ] Can submit messages via kimura.py
- [ ] Can query blocks and height via kimura.py
- [ ] RPC endpoints work after node restart
- [ ] End-to-end tests pass
- [ ] API documentation complete

---

## Success Criteria (End of MVP)

### Functionality
- [ ] Single leader produces blocks every 5 seconds
- [ ] Messages submitted via RPC appear in next block
- [ ] Peers receive and validate all blocks
- [ ] Node restarts without data loss
- [ ] All messages and blocks cryptographically signed
- [ ] RPC API accessible on port 8545

### Performance
- [ ] Handle 1000+ messages per block
- [ ] Block propagation < 1 second to all peers
- [ ] RPC response time < 100ms

### Quality
- [ ] 90%+ test coverage
- [ ] All tests passing (`python3 blockchain.py test --suite all`)
- [ ] No compiler warnings
- [ ] Clean clippy linting

### Engineering Standards
- [ ] Regular commits with descriptive messages
- [ ] Pull requests for substantial work
- [ ] Code reviewed via PRs
- [ ] Documentation updated as code changes

---

## Engineer Workflow

### Daily Workflow

1. **Start of day:**
   ```bash
   cd /home/kyruzic/dev/army/chain
   python3 blockchain.py build --mode release
   python3 blockchain.py test --suite unit
   ```

2. **Development:**
   - Write code in appropriate crate
   - Test locally
   - Write tests alongside code

3. **Committing work:**
   ```bash
   python3 blockchain.py git commit --message "Add feature X" --all
   ```

4. **End of milestone/substantial work:**
   ```bash
   python3 blockchain.py git pr --title "M1: Add storage layer" --base main
   ```

### Weekly Reviews

- Review milestone progress against plan
- Update MVP_PLAN.md if timeline changes
- Create PRs for completed milestones
- Retrospective: what worked, what didn't

---

## Getting Started

### For the Blockchain Engineer

1. Read `ENGINEER_GUIDE.md` completely
2. Review this MVP_PLAN.md
3. Start with M1 Week 1 Day 1
4. Build, test, commit, repeat
5. Create PR at end of each milestone

### Key Files

- `blockchain.py` - Your CLI tool (build, test, git)
- `kimura.py` - RPC testing tool (for testing after M4)
- `MVP_PLAN.md` - This document
- `ENGINEER_GUIDE.md` - Persona and coding guidelines
- `Cargo.toml` - Rust workspace
- `kimura-node/src/` - Main node implementation
- `kimura-storage/src/` - Database layer
- `kimura-network/src/` - P2P networking
- `kimura-blockchain/src/` - Core blockchain logic

---

## Notes

### Technical Decisions

- **No Merkle trees yet:** Use simple list of message IDs in block
- **No ancient/freezer DB:** Not needed for single-leader chain
- **No consensus algorithm:** Single leader produces all blocks
- **No smart contracts:** Simple messaging only
- **No sharding/partitioning:** Single chain, all nodes have full state

### Future Improvements (Post-MVP)

- Leader election algorithm (raft/pbft)
- Merkle tree for message verification
- Ancient DB for old blocks
- Configurable block time
- Message size limits
- Transaction fees
- Smart contract support
- Sharding

---

**End of MVP Plan**