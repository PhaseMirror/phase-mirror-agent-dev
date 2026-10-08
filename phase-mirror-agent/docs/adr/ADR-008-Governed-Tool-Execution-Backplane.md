# ADR-008: Governed Tool-Execution Backplane for phase-mirror-agent

- Status: accepted
- Date: 2026-08-08
- Owners: Multiplicity Foundation
- Tags: #tools, #execution, #allowlist, #dry-run, #idempotency, #receipts, #cnl
- Phase: phase-2 (master plan ADR-004)
- Related: ADR-004 (master), ADR-006 (authn gate), ADR-005 (receipt witnesses)

## 1. Context

Two defects undermine the governance promise of the agent:

1. **Hardcoded compilation** — `cnl.rs::compile_command` ignores most of the input and
   returns fixed `c: 0.84`, `rsc: 209.3` and fixed action parameters
   (`service = "web-service"`, `target = "cluster"`, `replicas = 3`). Metrics are
   not derived from the invariant core, so "zero drift" is violated.
2. **Stub execution** — `tools.rs` registers `deploy`/`scale`/`destroy` as closures that
   return fake receipts (`"deployed {service} to {target} (receipt REC-...)"`). No real
   side effect exists, and a fake success is indistinguishable from a real one.

Additionally, `main.rs` fabricates `witness_id = W-{timestamp}` without any hash
anchoring.

## 2. Decision

Introduce a **governed tool-execution backplane** with three layers:

### 2.1 Real CNL compilation (pirtm glue)

- Extend `compile_command` (or add `parse_action`) so that verified actions carry real
  parameter extraction: parse `<verb> <service> <target> [replicas=N]` from the token
  stream and derive `c` / `R_sc` / diagnostic from `PhaseMirrorInvariants` on the actual
  token ensemble rather than constants.
- Surface the computed values to `CommandResponse` unchanged (schema-compatible).
- Unknown/ambiguous inputs produce a deterministic `Err`/`No action parsed` — never a
  default action.

### 2.2 Governed executor trait (`src/executor/mod.rs`)

```rust
pub trait ToolExecutor: Send + Sync {
    fn name(&self) -> &'static str;
    fn allowed(&self, req: &ToolRequest) -> bool;      // allow-list policy
    fn validate(&self, req: &ToolRequest) -> Result<(), String>; // L0/param validation
    fn execute(&self, req: &ToolRequest) -> Result<Receipt, ExecError>; // real side effect
}
```

- `ToolRequest { tool, args, idempotency_key, dry_run }`.
- `Receipt { tool, status, idempotency_key, started_at, finished_at, detail, exit }`.
- `ToolRegistry::invoke` now returns a `Receipt` and requires an `idempotency_key`
  (SHA-256 of caller + args + key). Duplicate keys return the stored prior receipt
  without re-execution.

### 2.3 Adapters & allow-list

- Ship **no-op/simulated** adapters by default that are honest: they return
  `Receipt { status: "simulated", detail: "no side effect (simulated)" }` and are
  explicitly marked non-authoritative. Never format like a real receipt.
- Real adapters (`docker-compose`, `systemctl`, `kubectl`) are opt-in via
  `PHASE_MIRROR_TOOL_ALLOW` (comma list) and gated by `allowed()` policies, e.g. service
  names must match a regex allow-list, replicas bounded `1..=32`.
- `--dry-run` mode: execute `validate()` only, return a `plan` receipt with zero side effects.
- Unimplemented/unallowed tools → `Err("tool not allowed")` at admission, not a fake success.

### 2.4 Witness integrity

- Replace `W-{timestamp}` with a real hash: `witness_hash = SHA-256(canonical(action) ||
  idempotency_key || timestamp)`; store via ADR-005 chain. TS `witness.ts` uses the same
  algorithm (schema-driven).

## 3. Implementation Plan

**Phase:** Phase 2 of ADR-004.

**Target Artifacts:**
- `src/executor/mod.rs` — `ToolExecutor` trait, registry, idempotency map
- `src/executor/adapters/mod.rs` — simulated adapters (default)
- `src/executor/adapters/compose.rs`, `systemd.rs` — opt-in real adapters
- `src/cnl_bridge.rs` — real parameter extraction + invariant-derived metrics
- `src/main.rs` — wire `dry_run`/`idempotency_key` into `CommandRequest`
- `schema/tool_request.schema.json`, `schema/receipt.schema.json`
- `config/tools.toml.example` — allow-list, bounds

**Acceptance Criteria:**
- [ ] `compile_command("deploy my-service cluster 4")` yields `replicas=4`,
      `service="my-service"`, and non-constant `c`/`R_sc`.
- [ ] Simulated adapter receipts are unambiguously marked `simulated`.
- [ ] Unallowed tool → admission error, no receipt.
- [ ] Same `idempotency_key` twice → second call returns stored receipt, zero re-execution.
- [ ] `--dry-run` produces no side effects.
- [ ] `witness_hash` verifies against the documented algorithm.

## 4. Consequences

### Positive
- Governance metrics are real (zero drift), and receipts are truthful.
- Idempotency makes retries safe — critical for local deployment reliability.
- Dry-run gives operators a zero-risk preview of any admitted action.

### Negative / Tradeoff
- Real adapters introduce blast radius; mitigated by allow-lists, bounds, dry-run, and
  opt-in activation.
- Simulated mode is not useful for real automation until an adapter is enabled — an
  honest tradeoff over fake success.
- CNL parser expansion adds vocabulary scope (stays within ADR-001's controlled CNL).

### Neutral
- Backplane is trait-based; new integrations are one adapter file away.

## 5. Security & Governance

1. **Non-Bypassability** — tool execution only via `ToolRegistry::invoke` after authn
   (ADR-006) and admission; no direct handler→adapter call path.
2. **Immutable Audit** — every execution (including dry-run plans) writes a receipt
   witness to the ADR-005 chain.
3. **Zero Drift** — metrics derive from invariants; receipts reflect actual outcome
   (simulated receipts are labeled, never deceptive).

## 6. Dependencies

- `sha2` (present); `serde`/`serde_json` (present); no new heavy deps.
- ADR-005 (receipt witnesses), ADR-006 (authn), ADR-009 (tool config files).

## 7. Promotion Criteria

| Criteria | Target / Threshold | Status |
| :--- | :--- | :--- |
| Real CNL | Parameters/metrics derived, not constant | ⬜ |
| Honest receipts | `simulated` receipts labeled | ⬜ |
| Allow-list | Unallowed tools rejected | ⬜ |
| Idempotency | Duplicate keys don't re-execute | ⬜ |
| Dry-run | Zero side effects | ⬜ |
| Witness hash | Verifiable algorithm match | ⬜ |
