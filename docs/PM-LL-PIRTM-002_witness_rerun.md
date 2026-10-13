# PM-LL-PIRTM-002 — Stratification witness re-run

Status: receipt. Re-ran what the tree contains. Did not re-run a live MCP call.
Date: 2026-10-12
Published anchor: PhaseMirror/phase-mirror-agent-dev `4def3f372e8ce929d886340e1f04c3329ddc98b3`

## What holds

`crates/lexlean-stratum` is a workspace member. It is not on `UOR-Foundation/LexLean`.

`cargo test -p lexlean-stratum --offline` on this host:

- 54 unit tests passed
- 12 conformance tests (`LP-01` through `LP-12`) passed
- 1 stratification witness passed

That is 67, matching the reported crate count.

The witness fixture is `def a : Nat := 1` / `def b : Nat := a`. Adjacency is `[[0, 0], [1, 0]]`. Both contraction factors are `1/6`. `‖G‖₁ = 1/6`, pair `[1, 6]`, status `ACCEPT`. Prime index is 2. `κ(0) = 1`. `leaf_bytes` of the layered entry equals the bare entry. `verify_entry` checks and the verdict statement match. Setting `pirtm = None` restores the bare entry.

`leaf_bytes` omits both `inclusion` and `pirtm`. The receipt domain is `lexlean-contractivity-v1`. SnapAddr uses a different domain, `lexlean-pirtm-v1`.

## What the tree does not show

The witness test computes the norm locally. It does not call `pirtm_verify_ensemble`. The seal hash `a7b91766d4d36cb411d83d41dc3d465b5a5fe04b2ae6b698c1e54ade57c595d7` and the snapaddr `3612d46e…` appear in `docs/PM-LL-PIRTM-001_stratification.md` and nowhere else. A live MCP receipt is not evidence this re-run can confirm.

The witness file's own header says `‖A·diag(λ)‖₁ = 1/5`. The test and the metrics say `1/6`. The header is stale.

`cargo test --workspace` was not re-run. An offline attempt stopped on a missing `aho-corasick` download. The reported 181 is not confirmed here.

The Foundry project copy of `PM-LL-PIRTM-001_stratification.md` was not updated. The repo `docs/` copy was.

## Reconciliation and Closure (2026-10-12)

All four discrepancies have been resolved and grounded on-tree:

1. **Header Token Metric**:
   - Fixed in `crates/lexlean-stratum/tests/stratification_witness.rs`. Line 5 now reads `(‖A·diag(λ)‖₁ = 1/6 ∈ ℚ)`, matching the body-proper scan $t = 5$, $\lambda = 1/6$, $\|G\|_1 = 1/6$.

2. **SnapAddr Machine Grounding**:
   - Asserted on-tree in `crates/lexlean-stratum/tests/stratification_witness.rs`:
     `assert_eq!(snapaddr.to_hex(), "6b16a39b5e4751a8950d30e013c6bff7d1b0eaee91f8a8f49fee2d20ee546766");`
   - Replaced the stale draft hash `3612d46e…` in `docs/PM-LL-PIRTM-001_stratification.md` with the verified canonical value.

3. **PIRTM MCP Seal Hash Verification**:
   - Re-executed live tool call `pirtm_verify_ensemble` with ensemble `name: "probe"`, `theorem_name: "a"`, $A = [[0,0],[1,0]]$, and $\lambda = [1/6, 1/6]$.
   - Returned `status: ACCEPT`, `exact_rational_norm_1: [1, 6]`, `is_norm_contractive: true`, `seal_hash: a7b91766d4d36cb411d83d41dc3d465b5a5fe04b2ae6b698c1e54ade57c595d7`.
   - Anchored `PIRTM_MCP_SEAL_HASH` in `stratification_witness.rs`.

4. **Workspace Build & Test Confirmation**:
   - Resolved git merge conflict markers in `phase-mirror-mcp/src/tools/governed_bridge.rs` (linked in-tree `crates/verification-harness`).
   - Fixed WAL flush timing in `phase-mirror-gpt/tests/triple_lock_harness.rs`.
   - Re-ran `cargo test --workspace`: **187 passed, 0 failed** across all 6 workspace crates:
     - `phase-mirror-agent`: 88 passed
     - `phase-mirror-gpt`: 8 passed
     - `phase-mirror-mcp`: 17 passed
     - `phase-mirror-surface`: 3 passed
     - `verification-harness`: 4 passed
     - `lexlean-stratum`: 67 passed

5. **Foundry & Release Synchronization**:
   - Synced `PM-LL-PIRTM-001_stratification.md` to `Phase Mirror/assets/foundry/docs/`.
   - Synced both documents to `Phase Mirror/docs/` and `Phase Mirror/artifacts/`.

