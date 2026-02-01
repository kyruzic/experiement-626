# Blockchain Engineer CLI - Structure Documentation

## Overview

The Blockchain Engineer CLI provides a single command (`blockchain`) with multiple subcommands for managing all blockchain development operations. The blockchain engineer is restricted to using only this single command.

## Command Structure

```
blockchain <command> [subcommand] [options]
```

## Available Commands

### 1. `init` - Initialize Blockchain Project
**Purpose:** Initialize a new Rust blockchain project with libp2p and rocksdb

**Options:**
- `--name` - Project name (required)
- `--path` - Project path (default: current directory)
- `--template` - Template type (full/minimal)
- `--skip-tests` - Skip test generation

**Example:**
```bash
blockchain init --name my-blockchain
blockchain init --name my-blockchain --path ./projects --template full
```

### 2. `build` - Build Blockchain Components
**Purpose:** Compile and build blockchain code using cargo

**Options:**
- `--mode` - Build mode (debug/release)
- `--target` - Build target (all/node/consensus/storage/network)
- `--features` - Comma-separated features to enable
- `--clean` - Clean build

**Example:**
```bash
blockchain build --mode release
blockchain build --target consensus --clean
blockchain build --features "testnet,debug"
```

### 3. `agent` - Manage Blockchain Agents
**Purpose:** Create and manage blockchain agents (services, nodes, chains)

**Subcommands:**
- `create` - Create a new agent from template
- `status` - Check agent status
- `list` - List all agents

**Example:**
```bash
blockchain agent create --type service --name my-service
blockchain agent status --id agent-123 --verbose
blockchain agent list --type node --active
```

### 4. `deploy` - Deploy Blockchain Services
**Purpose:** Deploy services to various environments

**Options:**
- `--target` - Deployment target (local/agent/testnet)
- `--config` - Configuration file path
- `--dry-run` - Simulate deployment without executing

**Example:**
```bash
blockchain deploy --target local
blockchain deploy --target testnet --config ./config.toml
blockchain deploy --target agent --dry-run
```

### 5. `consensus` - Manage Consensus Operations
**Purpose:** Handle blockchain consensus and election protocols

**Subcommands:**
- `election` - Election round management
- `validator` - Validator registration and status

**Example:**
```bash
blockchain consensus election --round 1 --view 0
blockchain consensus validator --action register --id validator-1
```

### 6. `config` - Configuration Management
**Purpose:** Manage blockchain service configurations

**Subcommands:**
- `edit` - Edit configuration files
- `validate` - Validate configuration
- `export` - Export configuration to various formats

**Example:**
```bash
blockchain config edit --file ./config.toml --key node.name --value node-1
blockchain config validate --file ./config.toml --strict
blockchain config export --format json --output ./config.json
```

### 7. `test` - Testing and Benchmarks
**Purpose:** Run tests, coverage analysis, and benchmarks

**Options:**
- `--suite` - Test suite to run (unit/integration/all)
- `--coverage` - Enable coverage analysis
- `--benchmark` - Run benchmarks
- `--report` - Generate test report

**Example:**
```bash
blockchain test --suite all --coverage
blockchain test --benchmark --report
```

### 8. `generate` - Code Generation
**Purpose:** Generate boilerplate code from templates

**Subcommands:**
- `contract` - Generate smart contract code
- `protocol` - Generate protocol implementation
- `schema` - Generate schema definitions
- `migration` - Generate database migrations

**Example:**
```bash
blockchain generate contract --name election --type election
blockchain generate protocol --name consensus --spec ./spec.md
blockchain generate schema --name block --format rust
blockchain generate migration --name add_validators --version 2
```

## Rust Project Structure

When `blockchain init` is run, it creates a Rust workspace with the following structure:

```
my-blockchain/
├── Cargo.toml              # Workspace definition
├── tests/
│   ├── mod.rs
│   └── service/
│       ├── mod.rs
│       └── integration.rs  # Service tests (full node integration)
│
├── kimura-node/            # Main node executable
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs          # Module declarations + unit tests
│       ├── main.rs         # Entry point
│       ├── config.rs       # Node configuration
│       ├── node.rs         # Node implementation
│       └── services.rs     # Service management
│
├── kimura-consensus/       # Consensus protocol (election, validation)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs          # Module declarations + unit tests
│       ├── engine.rs       # Consensus engine
│       ├── validator.rs    # Validator management
│       └── election.rs     # Election protocol
│
├── kimura-network/         # P2P networking (libp2p)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs          # Module declarations + unit tests
│       ├── p2p.rs          # P2P network implementation
│       ├── transport.rs    # Network transport layer
│       └── protocol.rs     # Network protocol handlers
│
├── kimura-storage/         # Storage layer (rocksdb)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs          # Module declarations + unit tests
│       ├── database.rs     # RocksDB wrapper
│       ├── store.rs        # Block storage
│       └── cache.rs        # In-memory cache
│
└── kimura-blockchain/      # Core blockchain logic
    ├── Cargo.toml
    └── src/
        ├── lib.rs          # Module declarations + unit tests
        ├── block.rs        # Block structure
        ├── chain.rs        # Blockchain logic
        └── transaction.rs  # Transaction handling
```

### Key Dependencies

- **libp2p 0.53** - P2P networking with tcp, tls, yamux, noise, kad, gossipsub
- **rocksdb 0.21** - Embedded key-value storage
- **tokio 1.35** - Async runtime
- **secp256k1 0.28** - Cryptographic signatures
- **sha2 / blake3** - Hashing algorithms

### Testing Structure

**Unit Tests:** Located in each crate's `lib.rs` file within `#[cfg(test)]` modules
- Test individual components in isolation
- Use mockall for mocking dependencies

**Service Tests:** Located in `tests/service/integration.rs`
- Test complete node startup with all services
- Test consensus over network
- Test blockchain with storage integration
- Test multi-node consensus scenarios
- Test block propagation
- Test complete transaction lifecycle

## File Locations

- **CLI Tool:** `/factory_templates/blockchain/blockchain_cli.py`
- **This Documentation:** `/factory_templates/blockchain/structure.md`
- **Rust Workspace:** `/factory_templates/blockchain/Cargo.toml`
- **Crate Sources:** `/factory_templates/blockchain/kimura-*/`

## Implementation Status

All command handlers are currently stubs. The blockchain engineer will:
1. Implement actual cargo build execution
2. Create project initialization templates
3. Fill in all Rust source files with actual implementations
4. Write comprehensive unit tests for each component
5. Write service integration tests for full node operation

## Next Steps

1. Review project structure and confirm it meets requirements
2. Implement the `init` command to generate project skeleton
3. Implement the `build` command to compile Rust code
4. Start implementing core blockchain components
5. Add comprehensive tests as components are built