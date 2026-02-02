# Milestone 2 Implementation Plan: Persistence & Restart

**Status:** 🚧 IN PROGRESS  
**Duration:** Week 3 (7 days)  
**Goal:** Node can restart and resume from last block without data loss

---

## Overview

This document tracks the implementation of Milestone 2 for the Kimura blockchain. Each phase includes specific deliverables with checkboxes to mark completion.

**Key Design Decisions:**
- Message ID = blake3(sender_pubkey ++ nonce) only (no content/timestamp)
- CF_PENDING: Leader's temporary pending queue
- CF_MESSAGES: Permanent storage for all confirmed messages
- Atomic writes using RocksDB WriteBatch
- Test mode: 100ms block intervals for fast testing
- On-demand message fetching for peers

---

## Phase 1: Message System Refactor (Days 1-2)

### Day 1: Fix Message ID & Create Pending Store

**Status:** ⬜ NOT STARTED

**Goal:** Correct message ID format and separate pending from confirmed storage

**Deliverables:**

#### Task 1.1: Fix Message ID Generation
- [ ] Update `Message::calculate_id()` in `kimura-blockchain/src/message.rs`
- [ ] Remove content and timestamp from ID calculation
- [ ] ID = blake3(sender_pubkey ++ nonce) only
- [ ] Update any tests that depend on old ID format
- [ ] Run tests to verify: `cargo test -p kimura-blockchain`

**Files Modified:**
- `kimura-blockchain/src/message.rs`
- `kimura-blockchain/src/transaction.rs` (if tests there)

**Commit Message:** "Fix message ID to use sender + nonce only"

---

#### Task 1.2: Create PendingMessageStore
- [ ] Define `PendingMessageStore` struct in `kimura-storage/src/store.rs`
- [ ] Implement `put_pending(id, message)` - stores in CF_PENDING
- [ ] Implement `get_all_pending()` - returns Vec<Message>
- [ ] Implement `clear_all_pending()` - deletes all from CF_PENDING
- [ ] Implement `remove_pending(id)` - delete single message
- [ ] Ensure CF_PENDING column family is properly initialized
- [ ] Write unit tests for all methods
- [ ] Run tests: `cargo test -p kimura-storage`

**Files Modified:**
- `kimura-storage/src/store.rs`
- `kimura-storage/src/lib.rs` (export)

**Commit Message:** "Add PendingMessageStore for leader's pending queue"

---

#### Task 1.3: Update NodeServices Integration
- [ ] Add `pending_message_store: PendingMessageStore` field to NodeServices
- [ ] Initialize in `NodeServices::new()`
- [ ] Update `submit_message()` to store in CF_PENDING (if leader)
- [ ] Update `collect_pending_messages()` to use PendingMessageStore
- [ ] Update `clear_pending_messages()` to actually clear CF_PENDING
- [ ] Run tests: `cargo test -p kimura-node --lib`

**Files Modified:**
- `kimura-node/src/services.rs`

**Commit Message:** "Integrate PendingMessageStore into services"

---

### Day 2: Atomic Writes & Test Mode

**Status:** ⬜ NOT STARTED

**Goal:** Ensure atomic persistence and enable fast testing

**Deliverables:**

#### Task 2.1: Implement Atomic Block+Messages Save
- [ ] Add `save_block_atomic()` method to NodeServices
- [ ] Create WriteBatch with:
  - Block in CF_BLOCKS
  - Metadata (height + hash) in CF_METADATA
  - All messages in CF_MESSAGES
- [ ] Use `db.batch_write(batch)` for atomic commit
- [ ] Add error handling for batch failures
- [ ] Write unit test for atomic write
- [ ] Run tests: `cargo test -p kimura-node test_atomic`

**Files Modified:**
- `kimura-node/src/services.rs`
- `kimura-storage/src/store.rs` (if helper needed)

**Commit Message:** "Add atomic block+messages+metadata writes"

---

#### Task 2.2: Add Test Mode Configuration
- [ ] Add `test_mode: bool` field to `NodeConfig` in `config.rs`
- [ ] Add `--test-mode` CLI argument in `main.rs`
- [ ] Update `block_interval()` method:
  - Test mode: 100ms
  - Normal mode: 5s (from config)
- [ ] Update `NodeConfig::leader()` and `NodeConfig::peer()` constructors
- [ ] Update `NodeConfig::default()` to include test_mode: false
- [ ] Run build: `cargo build -p kimura-node`

**Files Modified:**
- `kimura-node/src/config.rs`
- `kimura-node/src/main.rs`

**Commit Message:** "Add test mode with 100ms block intervals"

---

#### Task 2.3: Update Block Production Flow
- [ ] Update `produce_block()` to use `save_block_atomic()`
- [ ] After saving block, call `clear_pending_messages()`
- [ ] Add logging: "Produced block X with Y messages"
- [ ] Verify flow: Collect pending → Create block → Save atomically → Clear pending
- [ ] Run integration test: `cargo test -p kimura-node --test integration_tests test_leader_produces_blocks_rpc`

**Files Modified:**
- `kimura-node/src/node.rs`

**Commit Message:** "Update block production to use atomic writes"

---

## Phase 2: Integrity & Shutdown (Days 3-4)

### Day 3: Chain Integrity Verification

**Status:** ⬜ NOT STARTED

**Goal:** Verify chain correctness on startup

**Deliverables:**

#### Task 3.1: Implement Chain Integrity Check
- [ ] Add `verify_chain_integrity()` to NodeServices
- [ ] Iterate all blocks from 0 to last_height
- [ ] Verify each block exists
- [ ] Verify prev_hash links correctly (block N prev_hash == block N-1 hash)
- [ ] Return detailed error on first mismatch
- [ ] Add logging: "Chain integrity verified: X blocks"
- [ ] Write unit tests:
  - Valid chain passes
  - Missing block detected
  - Hash mismatch detected
- [ ] Run tests: `cargo test -p kimura-node test_integrity`

**Files Modified:**
- `kimura-node/src/services.rs`

**Commit Message:** "Add chain integrity verification on startup"

---

#### Task 3.2: Integrate Integrity Check into Node Startup
- [ ] Call `verify_chain_integrity()` in `Node::new()` after loading metadata
- [ ] Exit with error code if verification fails
- [ ] Add helpful error message showing what's wrong
- [ ] Test with corrupted database (manually modify one block)
- [ ] Verify node refuses to start with corruption

**Files Modified:**
- `kimura-node/src/node.rs`

**Commit Message:** "Integrate integrity check into node startup"

---

### Day 4: Graceful Shutdown

**Status:** ⬜ NOT STARTED

**Goal:** Ensure clean shutdown with data persistence

**Deliverables:**

#### Task 4.1: Implement Database Sync on Shutdown
- [ ] Add `sync()` method to RocksDB wrapper if not exists
- [ ] Implement `Node::shutdown()` method:
  - Flush WAL to disk
  - Sync all column families
  - Shutdown RPC server
  - Log shutdown completion
- [ ] Add error handling for sync failures
- [ ] Add timeout for shutdown (max 5 seconds)

**Files Modified:**
- `kimura-storage/src/database.rs` (if sync needed)
- `kimura-node/src/node.rs`

**Commit Message:** "Add graceful shutdown with database sync"

---

#### Task 4.2: Add Signal Handling
- [ ] Add Ctrl+C (SIGINT) handler in `main.rs`
- [ ] Call `node.shutdown().await` on signal
- [ ] Ensure all spawned tasks are cancelled gracefully
- [ ] Test: Start node, press Ctrl+C, verify clean exit
- [ ] Check logs show "Shutdown complete"

**Files Modified:**
- `kimura-node/src/main.rs`

**Commit Message:** "Add signal handling for graceful shutdown"

---

## Phase 3: Restart Logic (Days 5-6)

### Day 5: Leader Restart

**Status:** ⬜ NOT STARTED

**Goal:** Leader resumes block production after restart

**Deliverables:**

#### Task 5.1: Implement Leader Resume Logic
- [ ] In `LeaderState::new()`, accept `last_height: u64` parameter
- [ ] Set `next_height = last_height + 1`
- [ ] Log: "Leader resuming from height X"
- [ ] Update `Node::new()` to pass current height to LeaderState
- [ ] Test: Start leader, produce 5 blocks, restart, verify continues from 6

**Files Modified:**
- `kimura-node/src/node.rs`

**Commit Message:** "Add leader restart resume logic"

---

#### Task 5.2: Add RPC Endpoint for Message Fetching
- [ ] Add `GET /message/:id` endpoint to RPC server
- [ ] Query CF_MESSAGES by ID
- [ ] Return full message data
- [ ] Return 404 if message not found
- [ ] Test endpoint manually with curl

**Files Modified:**
- `kimura-node/src/rpc.rs`

**Commit Message:** "Add RPC endpoint to fetch messages by ID"

---

### Day 6: Peer Restart & On-Demand Fetching

**Status:** ⬜ NOT STARTED

**Goal:** Peer maintains chain and can fetch messages on demand

**Deliverables:**

#### Task 6.1: Peer Chain Persistence Verification
- [ ] Verify peer stores blocks with message IDs on receive
- [ ] Ensure peer can query all historical blocks via RPC
- [ ] Verify integrity check works for peer mode
- [ ] Test: Peer syncs 20 blocks, verify integrity passes

**Files Modified:**
- `kimura-node/src/node.rs` (verify process_received_block)

**Commit Message:** "Verify peer stores blocks with message IDs"

---

#### Task 6.2: Implement On-Demand Message Fetching
- [ ] When peer needs message content, fetch from local CF_MESSAGES
- [ ] If not in local DB, fetch from leader via RPC (`GET /message/:id`)
- [ ] Cache fetched messages locally
- [ ] Update `BlockResponse` to optionally include full messages
- [ ] Add integration test for on-demand fetching

**Files Modified:**
- `kimura-node/src/services.rs`
- `kimura-node/src/rpc.rs`
- `tests/rpc_client.rs` (add fetch method)

**Commit Message:** "Add on-demand message fetching for peers"

---

## Phase 4: Testing (Day 7)

### Day 7: Comprehensive Test Suite

**Status:** ⬜ NOT STARTED

**Goal:** Full test coverage for persistence and restart

**Deliverables:**

#### Task 7.1: Unit Tests
- [ ] `test_message_id_format` - Verify blake3(sender ++ nonce)
- [ ] `test_pending_message_lifecycle` - Submit → Pending → Block → Clear
- [ ] `test_atomic_write_all_or_nothing` - Verify atomicity
- [ ] `test_chain_integrity_valid` - Happy path
- [ ] `test_chain_integrity_missing_block` - Detect missing
- [ ] `test_chain_integrity_hash_mismatch` - Detect corruption
- [ ] Run all unit tests: `cargo test -p kimura-node --lib`

**Files Modified:**
- `kimura-node/src/services.rs` (tests)
- `kimura-storage/src/store.rs` (tests)

**Commit Message:** "Add comprehensive unit tests for persistence"

---

#### Task 7.2: Integration Tests
- [ ] `test_leader_restart_continuity`:
  - Start leader, produce 10 blocks
  - Shutdown gracefully
  - Restart with same DB
  - Verify height = 10
  - Verify continues to height 15
  
- [ ] `test_peer_restart_replay`:
  - Start leader + peer
  - Sync 20 blocks
  - Shutdown peer
  - Restart peer
  - Verify all 20 blocks accessible
  - Verify can fetch messages on-demand
  
- [ ] `test_crash_recovery_no_data_loss`:
  - Start leader with test mode
  - Produce blocks rapidly
  - Kill process mid-operation (simulate crash)
  - Restart
  - Verify last block before crash exists
  - Verify no corruption

**Files Modified:**
- `tests/integration_tests.rs`

**Commit Message:** "Add integration tests for restart and recovery"

---

#### Task 7.3: Update Existing Tests for Test Mode
- [ ] Update `test_leader_produces_blocks_rpc` to use `--test-mode`
- [ ] Update `test_peer_receives_blocks_rpc` to use `--test-mode`
- [ ] Update `test_multi_peer_sync_rpc` to use `--test-mode`
- [ ] Update `test_message_inclusion_rpc` to use `--test-mode`
- [ ] Update `test_chain_continuity_rpc` to use `--test-mode`
- [ ] Run all integration tests: `cargo test -p kimura-node --test integration_tests`
- [ ] Verify all tests pass in under 10 seconds

**Files Modified:**
- `tests/integration_tests.rs`

**Commit Message:** "Enable test mode for fast integration tests"

---

## Definition of Done

### Phase 1 Complete
- [ ] Message ID = blake3(sender ++ nonce) only
- [ ] PendingMessageStore implemented and tested
- [ ] Atomic writes using WriteBatch
- [ ] Test mode with 100ms intervals

### Phase 2 Complete
- [ ] Chain integrity verification on startup
- [ ] Graceful shutdown with database sync
- [ ] Signal handling (Ctrl+C)

### Phase 3 Complete
- [ ] Leader resumes from last block
- [ ] Peer maintains full chain
- [ ] On-demand message fetching via RPC
- [ ] `GET /message/:id` endpoint

### Phase 4 Complete
- [ ] 6+ unit tests passing
- [ ] 3 integration tests passing
- [ ] All existing tests still pass
- [ ] Tests complete in < 10 seconds

### Final Checks
- [ ] `python3 blockchain.py build --mode release` succeeds
- [ ] `python3 blockchain.py test --suite unit` passes
- [ ] `python3 blockchain.py test --suite integration` passes
- [ ] No compiler warnings
- [ ] Clean clippy linting
- [ ] Documentation updated

---

## Progress Log

| Date | Phase | Task | Status | Notes |
|------|-------|------|--------|-------|
| | Phase 1 | Day 1 Task 1.1 | ⬜ | |
| | Phase 1 | Day 1 Task 1.2 | ⬜ | |
| | Phase 1 | Day 1 Task 1.3 | ⬜ | |
| | Phase 1 | Day 2 Task 2.1 | ⬜ | |
| | Phase 1 | Day 2 Task 2.2 | ⬜ | |
| | Phase 1 | Day 2 Task 2.3 | ⬜ | |
| | Phase 2 | Day 3 Task 3.1 | ⬜ | |
| | Phase 2 | Day 3 Task 3.2 | ⬜ | |
| | Phase 2 | Day 4 Task 4.1 | ⬜ | |
| | Phase 2 | Day 4 Task 4.2 | ⬜ | |
| | Phase 3 | Day 5 Task 5.1 | ⬜ | |
| | Phase 3 | Day 5 Task 5.2 | ⬜ | |
| | Phase 3 | Day 6 Task 6.1 | ⬜ | |
| | Phase 3 | Day 6 Task 6.2 | ⬜ | |
| | Phase 4 | Day 7 Task 7.1 | ⬜ | |
| | Phase 4 | Day 7 Task 7.2 | ⬜ | |
| | Phase 4 | Day 7 Task 7.3 | ⬜ | |

---

## Technical Notes

### Atomic Write Pattern
```rust
let mut batch = WriteBatch::default();

// Add block
batch.put_cf(cf_blocks, &block_key, &block_value);

// Add metadata
batch.put_cf(cf_meta, b"meta:last_height", &height.to_be_bytes());
batch.put_cf(cf_meta, b"meta:last_hash", &hash);

// Add messages
for msg in messages {
    batch.put_cf(cf_messages, &msg_key, &msg_value);
}

// Atomic commit - all succeed or all fail
db.write(batch)?;
```

### Test Mode Benefits
- Integration tests complete in seconds instead of minutes
- Faster feedback loop during development
- Can still test with 5s intervals in specific test if needed

### Integrity Check Algorithm
```rust
for height in 0..=last_height {
    let block = get_block(height)?;
    if height > 0 {
        let prev = get_block(height - 1)?;
        assert!(block.prev_hash == prev.hash());
    }
}
```

### Shutdown Sequence
1. Stop accepting new RPC requests
2. Finish current operation (if any)
3. Sync database to disk
4. Close database connection
5. Exit

---

**Start Date:** _______________  
**Estimated Completion:** _______________  
**Actual Completion:** _______________

**Status:** Ready to begin Phase 1, Day 1
