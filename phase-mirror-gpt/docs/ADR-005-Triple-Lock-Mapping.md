# ADR-005 to Triple-Lock Enforcement Mapping

## Format: Matrix Table

| Triple-Lock Phase | ADR-005 Enforcement Target | Validator Component | Fail-Closed Mechanism |
| :--- | :--- | :--- | :--- |
| **Genius (Draft)** | Input Integrity | MCP Transport (`transport.rs`) | JSON parse failures trigger immediate HTTP/RPC -32700 blocks before reaching the kernel. |
| **Guardian (L1)** | Semantic Integrity | `domain_invariants.rs` | Scans draft against hot-reloadable `policy.toml`. If pattern matched, immediately aborts and logs `mirror_semantic_violation`. |
| **Guardian (L0-Tel)** | Compliance Floor | `telemetry.rs` | Checks `LiveTelemetryOracle` for 100% artifact verification. Below 100% = Hard Block. |
| **Guardian (L0-Bit)** | Structural Integrity | `validator.rs` | Bitmask comparison (`ctx.permission_bits & PERM_WRITE`). Missing bits = `GovernanceOutcome::Block`. |
| **Examiner (Cert)** | Lineage & Registry | `archivum.rs` / `qwen_sovereignty_gate.rs` | Verifies `prev_hash`. If chain breaks, returns `Chain continuity broken`. Writes atomic `witness_hash` to `MASTER_REGISTRY.md`. |
