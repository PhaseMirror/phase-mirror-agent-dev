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

## Scaffolding removed

`PIRTM_MCP_SEAL_HASH` and its `len() == 64` check were removed from `stratification_witness.rs`.

The witness test binds the receipt to its own domain:

```rust
assert_eq!(ContractivityReceipt::HASH_AUTHORITY, "lexlean-contractivity-v1");
```

and asserts the matching JSON field. That grounds the SPEC.md §34.0 claim: the compiler's `seal_hash` preimage is sovereign to PIRTM and unrecoverable by LexLean. The crate does not pretend to verify a PIRTM seal. Verification across the process boundary to `pirtm-mcp` is an external evidence check (Option 3 pattern), not an internal crate proof.

## Foundry session

The copies under Foundry project `docs/`, `Phase Mirror/docs/`, and `Phase Mirror/artifacts/` are synchronized with `f7da9c4`.
