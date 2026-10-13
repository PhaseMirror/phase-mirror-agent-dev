# PM-MCP-002 — Option 3 joint closure

Status: applied, not a contract.
Date: 2026-10-12
Repository: PhaseMirror/phase-mirror-agent-dev
Head: `591dd42faee1d6f7cbe9952eba197b1faa6f7be0`

## Confirmed gaps, now closed

| Action | Target | Verification |
|---|---|---|
| Refuse PATH. Require `PRISMPM_BIN` to be an absolute file whose SHA-256 equals a `prismpm` inventory digest in `prismpm.lock`. Require schema `prismpm/sdk-lock/2` and a digest-pinned `sdk_image`. | `phase-mirror-mcp/src/tools/prismpm_actions.rs` | `binary_resolution_refuses_path_and_a_mismatched_digest`, `undigested_lock_is_refused` |
| Bind evidence reads. `prismpm/sdk-lock/2` does not name an evidence directory, so the directory is `PRISMPM_EVIDENCE_DIR`, not a caller argument and not an invented lock field. Accept only a single filename and resolve it inside that directory. | same file | traversal, absolute name, outside-file, and inside-file plants |
| Connect in-tree verification-harness and CNL grammar parsing. Port `verification-harness` in-tree to eliminate external relative path breakages. Implement CNL command parsing and invariant checks in `cnl_bridge`. | `phase-mirror-mcp/Cargo.toml`, `phase-mirror-mcp/src/tools/governed_bridge.rs`, `phase-mirror-agent/src/cnl_bridge.rs` | In-tree `crates/verification-harness` linked. Both MCP and Agent test suites pass. |
| Publish the workspace package keys `phase-mirror-gpt` already inherits. | root `Cargo.toml` | `rust-version`, `authors`, `license`, `repository`, `homepage`, `description` |

`cargo test -p phase-mirror-mcp --lib tools::prismpm_actions` : 7 passed, 0 failed.
`cargo test --workspace` : 114 passed, 0 failed.

## Grounded status

The MCP lib tests compile and the seven plants pass. In-tree verification-harness evaluates governed bridge transitions. CNL compilation and Phase Mirror invariants are enforced in-tree. All 114 tests across the workspace pass.
