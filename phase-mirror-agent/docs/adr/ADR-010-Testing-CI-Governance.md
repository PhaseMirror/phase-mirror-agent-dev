# ADR-010: Testing & CI Governance for phase-mirror-agent

- Status: proposed
- Date: 2026-08-08
- Owners: Multiplicity Foundation
- Tags: #testing, #ci, #rust, #typescript, #kani, #release
- Phase: phases 0 & 7 (master plan ADR-004)
- Related: ADR-004 (master), ADR-001/002 (ALP-NLP contract), ADR-005-009 (each defines acceptance criteria)

## 1. Context

`phase-mirror-agent` has no automated verification gate of its own:

- **Zero Rust tests** in the package; the server is untested.
- TS ALP-NLP has 21 tests but nothing runs them in CI for this package, and `tsc` build
  (`npm run build`) is not CI-gated.
- There is no coverage baseline, so regressions in the governance path are easy to ship.
- No e2e verification of the deployment surface (health/readiness, authn, WAL durability,
  graceful shutdown).
- The invariant core (`pirtm` crates) is the correctness foundation; its proofs (Kani)
  are not wired into this package's CI story.

## 2. Decision

Establish a **test pyramid + CI gate** so every promotion is machine-verified.

### 2.1 Test pyramid

| Level | Scope | Tooling | Location |
| :-- | :-- | :-- | :-- |
| Unit | CNL bridge, audit store, tools registry, auth, ratelimit, ws | `cargo test` | `src/**` `#[cfg(test)]` |
| Unit | ALP-NLP modules (lexer, parser, policy, witness, archivum) | `vitest` | `tests/alp-nlp/*.test.ts` |
| Integration | HTTP routes, authn/ratelimit middleware, WAL durability (kill -9), chain integrity | `cargo test` + `tests/` integration dir | `tests/` (Rust) |
| E2E | compose up → `/ready` → authenticated command → receipt witness → stop → integrity | shell + `curl` script | `scripts/e2e.sh` |
| Property/Kani | Invariant-core contracts (optional, best-effort) | `cargo kani` | `Prime/packages/rust/pirtm-*/tests/kani_*.rs` |

### 2.2 CI gate (GitHub Actions)

- On PR and push to `main`: `cargo fmt --check`, `cargo clippy -- -D warnings`,
  `cargo test --manifest-path packages/phase-mirror-agent/Cargo.toml`,
  `npm ci`, `npm run build` (tsc), `vitest run`.
- Add `packages/phase-mirror-agent` to the existing `.github/workflows` (align with
  `ci-governance.yml` / `release.yml` patterns already in the repo).
- `release.yml`: build release binary, build Docker image, push to GHCR, attach checksums.
- Optional job: `cargo kani` on the pirtm invariant crates consumed by the agent
  (time-boxed; can be nightly).

### 2.3 Coverage & governance assertions

- Gate on Rust test **coverage** via `llvm-cov` (target ≥ 70% of `src/`).
- Encode governance invariants as testable assertions shared across both languages via
  the JSON Schema exports (ADR-002 pending item) — e.g. witness chain algorithm has a
  Rust test and a TS test asserting identical output on the same fixture.
- Promote only when: all tests green, `clippy -D warnings`, coverage ≥ threshold,
  `docker build` green, `scripts/e2e.sh` green.

## 3. Implementation Plan

**Phase:** Phases 0 (baseline) and 7 (gate closure) of ADR-004.

**Target Artifacts:**
- Rust unit + integration tests (`src/*#[cfg(test)]`, `tests/*.rs`)
- `tests/alp-nlp/*` (exists; wire to CI)
- `scripts/e2e.sh` — compose lifecycle + authn + WAL integrity
- `.github/workflows/agent-ci.yml`, extend `release.yml`
- `deny.toml`-style policy kept in `Prime/` (no new toolchain)

**Acceptance Criteria:**
- [ ] `cargo test`, `vitest run`, `npm run build` green in CI on a clean checkout.
- [ ] `clippy -- -D warnings` and `cargo fmt --check` enforced.
- [ ] Coverage of `src/` (Rust) ≥ 70% measured and reported.
- [ ] Rust and TS witness-chain fixtures produce identical hashes.
- [ ] `scripts/e2e.sh` passes end-to-end on CI runner (or documented local equivalent).
- [ ] Release workflow builds image + artifacts with checksums.

## 4. Consequences

### Positive
- Regressions in the governance path cannot ship silently.
- Cross-language schema drift (ADR-002 concern) is caught by fixture parity tests.
- Promotion to GA is purely mechanical: green gates.

### Negative / Tradeoff
- CI time increases (rust + ts + optional kani); mitigated by caching and nightly-only kani.
- Coverage gate can incentivize superficial tests; mitigated by requiring behavioral
  assertions (fixture parity, durability) over line-count.

### Neutral
- Kani remains best-effort at package level; the invariant core retains its own proofs.

## 5. Security & Governance

1. **Non-Bypassability** — the CI gate is the only promotion path; no manual bypass.
2. **Zero Drift** — cross-language fixture tests pin the witness/audit algorithms.
3. **Immutable Audit** — e2e verifies the WAL chain end-to-end after restart.

## 6. Dependencies

- ADR-005 (chain fixtures), ADR-006 (authn fixtures), ADR-007 (e2e readiness),
  ADR-008 (receipt fixtures), ADR-009 (e2e compose).
- Root `.github/workflows/ci-governance.yml`, `release.yml` for patterns.

## 7. Promotion Criteria

| Criteria | Target / Threshold | Status |
| :--- | :--- | :--- |
| Unit+Integration | `cargo test` + `vitest` green | ⬜ |
| Lint | `clippy -D warnings`, `fmt --check` | ⬜ |
| Coverage | Rust `src/` ≥ 70% | ⬜ |
| Cross-language parity | Witness/chain fixture hashes identical | ⬜ |
| E2E | `scripts/e2e.sh` green | ⬜ |
| Release | Image + checksums built | ⬜ |
