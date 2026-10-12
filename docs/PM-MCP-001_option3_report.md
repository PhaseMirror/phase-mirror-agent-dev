# PM-MCP-001 — Phase mirror of the Option 3 tool report

Status: receipt, not a contract. No pull request.
Date: 2026-10-08
Published anchor checked: PhaseMirror/PhaseMirror `main` `d3b384cb65b0a133919b1238cfb5061e9738dbf6` (2026-09-13)

## Reported, not on that ref

The report says the root `Cargo.toml` members were restored to `["crates/*", "xtask"]`, `phase-mirror-agent/Cargo.toml` again has a `[workspace]` directive, and `phase-mirror-mcp/src/tools/prismpm_actions.rs` wraps a `prismpm` process for:

- `prismpm_model_check`
- `prismpm_generate`
- `prismpm_verify`
- `prismpm_evidence_read`

routed from `legacy_tool_handler` and advertised in both initialize responses.

On the published tree, `phase-mirror-mcp/src/tools/` contains `evaluate_esi_risk.rs`, `governed_bridge.rs`, `litigation_scan.rs`, `llm_generate.rs`, `mod.rs`, `spoliation_check.rs`, `stability_witness.rs`, and `verify_ledger.rs`. `prismpm_actions.rs` is absent. A code search for `prismpm_model_check` and `prismpm_evidence_read` returns nothing on UOR-Foundation or PhaseMirror. There is no root `Cargo.toml` on that ref. `phase-mirror-agent/Cargo.toml` does contain an empty `[workspace]` table; that line alone does not show a revert.

The published MCP crate also depends on `../../../Foundry/tests/verification-harness`, a path outside the repository. That tree does not build as a closed workspace.

## What would close the joint

Process invocation is the right shape only if the unpublished file does all of the following:

- The `prismpm` binary is the one named by `prismpm.lock` in the digest-pinned devcontainer, not the first `prismpm` on `PATH`.
- The argument vector is a closed set. A tool does not accept a free-form subcommand.
- `prismpm_evidence_read` reads only the evidence directory the lock names. An arbitrary path is a socket the joint does not have.
- The MCP binary links no PrismPM crate and contains no compiler methodology of its own.
- A plant that points the tool at a different binary, or that asks `evidence_read` for a path outside the evidence directory, fails.

Until that file is on a published ref and those plants exist, Option 3 is the adopted joint and not a landed implementation.
