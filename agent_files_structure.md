# AI Agent File Structure

## Overview
Military AI hierarchy with tiered agents communicating via blockchain.

## Agent Files

### Kimimura Blockchain Service (Tier 1 - Rust)
**File:** `kimura_blockchain_agent.rs`
- Purpose: Blockchain production and message validation
- Version: Kimi 2.5 Rust agent
- Configuration: Blockchain tooling, consensus mechanisms, election protocols

### General Agent (Tier 1 - AI)
**File:** `general_ai_agent.md`
- Model: GLM-4.7-flash capable
- Role: Central strategic command
- Dependencies: blockchain_service, command_protocol

### Lieutenant Agent (Tier 2 - AI)
**File:** `lieutenant_ai_agent.md`
- Model: M-series Mac mini capable
- Role: Regional/functional sector management
- Dependencies: blockchain_service @tier1

### Tier 3 Agent (Tier 3 - AI)
**File:** `tier3_agent.md`
- Hardware: Cheap low-end systems
- Intelligence: Remote LLM (Kimi 2.5 URL provided)
- Role: Active infiltration, tactical operations
- Dependencies: blockchain_service, offensive_tools

## Communication Flow
User → Kimura Service → General AI → Lieutenant AI → Tier 3 Agents

## Election Protocol
Blockchain consensus for General leadership succession