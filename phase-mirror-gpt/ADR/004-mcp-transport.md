# ADR-004: MCP Transport Protocol

## Status
Accepted

## Context
The Phase Mirror kernel needs to interact with AI agents (such as Claude Desktop or custom agents) while maintaining absolute control over the execution bounds. We need a standardized transport layer that prevents untrusted inputs from reaching the critical kernel.

## Decision
We will expose the `L0Validator` and `SyncManager` kernels via a Model Context Protocol (MCP) tool transport wrapper over JSON-RPC 2.0.
- **Safe Bridging**: We will parse untrusted dynamic JSON types strictly into statically verified `EvaluationContext` boundaries. Any mismatch instantly halts processing.
- **Strict Schema Enforcement**: We will provide a definitive `mcp-contract.json` that advertises the tools (`validate_l0_invariants` and `reconcile_merkle_sync`) and defines the precise types, bounds, and enum constants allowable.
- **Asynchronous Execution**: We will use Tokio's async I/O lines (stdin/stdout) to pipe the JSON-RPC streams efficiently without blocking.

## MCP FFI Integration Layer (Phase Mirror MCP)
The `phase-mirror-mcp` crate implements the FFI integration layer with Sedona Spine witness preservation:
- **Guard Wrappers**: All tool calls (`try_get_stability_metric`, `try_verify_ledger_internal`, `try_evaluate_esi_risk`, `try_check_governed_bridge`, `try_get_metrics`) evaluate `λ_p * L_p < 1.0` before JSON serialization
- **Witness Atoms**: `LambdaTrace` and `ContractivityReceipt` structures provide auditable state snapshots
- **In-Process Library Call**: The MCP server links against `phase-mirror-gpt` crate to eliminate JSON serialization boundary before Sedona Spine evaluation

## Consequences
- AI agents are structurally constrained to only invoke well-typed, pre-approved governance and synchronization mechanisms.
- Malformed data from the LLM or network is caught safely at the transport boundary before any heap allocation or complex processing in the hot path.
- Facilitates immediate integration with existing MCP host environments.
