# PM-LL-PIRTM-003 — Reconciliation re-run

Status: receipt. Re-ran the tree. Did not re-execute the MCP tool.
Date: 2026-10-12
Published anchor: PhaseMirror/phase-mirror-agent-dev `f7da9c446377bf35d5be31ed986cdd2a81d67b7b`

## Holds

Header line 5 of `crates/lexlean-stratum/tests/stratification_witness.rs` now reads `1/6`.

SnapAddr is asserted and the witness passes:

`6b16a39b5e4751a8950d30e013c6bff7d1b0eaee91f8a8f49fee2d20ee546766`

`docs/PM-LL-PIRTM-001_stratification.md` carries that hash, not the earlier `3612d46e…`.

`cargo test --workspace` on this host: 187 passed, 0 failed.

- `lexlean-stratum`: 54 + 12 + 1 = 67
- `phase-mirror-agent`: 88
- `phase-mirror-gpt`: 8
- `phase-mirror-mcp`: 17
- `phase-mirror-surface`: 3
- `verification-harness`: 4

## Still a constant

`PIRTM_MCP_SEAL_HASH` is a string literal. The only assertion is `len() == 64`. The test does not call `pirtm_verify_ensemble`, and the hash is not compared with a receipt the test produced. The live call remains a claim in the docs. The request the docs record sent the factors as decimal floats; SPEC.md §34.0 says the compiler reconstructs those into ℚ, which is consistent with a returned pair of `[1, 6]`, and is not a LexLean receipt.

## Foundry session

The copies under this session's `artifacts/` were still the pre-reconciliation files. They are now the published `docs/` versions from `eecadb06`.
