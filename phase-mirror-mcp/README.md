# Phase Mirror MCP Server

The Phase Mirror Model Context Protocol (MCP) server is a high-integrity gateway for agentic domain-specific reasoning, integrating the PIRTM Kernel and enforcing Sedona Spine governance logic.

## Architecture
- **PIRTM Kernel**: Integrated via `sigmatics-core`.
- **Governance Floor**: Enforces Experimental vs. Authoritative tiers.
- **Sedona Spine**: Evaluates ESI (Electronically Stored Information) risk and retention.

## MCP Tools

### `verify_ledger`
Verifies the mathematical integrity of the transition ledger and checks L0 invariants (schema version, drift magnitude, nonce freshness).

### `evaluate_esi_risk`
Evaluates the spoliation risk and retention requirements for data artifacts. Routes through the Sedona Spine engine to provide litigation hold advisory and preservation recommendations.

## Installation
Run the provided installation script:
```bash
./install.sh
```

## Configuration
The server communicates via JSON-RPC over `stdio`. Integrate it into your MCP client configuration (e.g., Claude Desktop, Gemini CLI) by pointing to the compiled binary.
