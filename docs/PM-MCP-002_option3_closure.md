# PM-MCP-002 — Option 3 joint closure

Status: applied, not a contract.
Date: 2026-10-12
Repository: PhaseMirror/phase-mirror-agent-dev
Head: `0b94b87737a935e2a235e9be1645a134b7062ff5`

## Confirmed gaps, now closed

| Action | Target | Verification |
|---|---|---|
| Refuse PATH. Require `PRISMPM_BIN` to be an absolute file whose SHA-256 equals a `prismpm` inventory digest in `prismpm.lock`. Require schema `prismpm/sdk-lock/2` and a digest-pinned `sdk_image`. | `phase-mirror-mcp/src/tools/prismpm_actions.rs` | `binary_resolution_refuses_path_and_a_mismatched_digest`, `undigested_lock_is_refused` |
| Bind evidence reads. `prismpm/sdk-lock/2` does not name an evidence directory, so the directory is `PRISMPM_EVIDENCE_DIR`, not a caller argument and not an invented lock field. Accept only a single filename and resolve it inside that directory. | same file | traversal, absolute name, outside-file, and inside-file plants |
| Drop the external path dependencies that made a root check fail with os error 2. | `phase-mirror-mcp/Cargo.toml`, `phase-mirror-agent/Cargo.toml`, `phase-mirror-mcp/src/tools/governed_bridge.rs`, `phase-mirror-agent/src/main.rs` | neither `../../../Foundry/tests/verification-harness` nor `../../Prime/packages/rust/pirtm-apps` remains. Both calls refuse until the crates are in-tree. |
| Publish the workspace package keys `phase-mirror-gpt` already inherits. | root `Cargo.toml` | `rust-version`, `authors`, `license`, `repository`, `homepage`, `description` |

`cargo test -p phase-mirror-mcp --lib tools::prismpm_actions` : 7 passed, 0 failed.

## Not claimed

A full `cargo check --workspace` was not the acceptance gate of this receipt. The MCP lib tests compile and the seven plants pass. CNL compilation and the governed bridge are closed refusals, not implementations.
