# ADR-005: Persistent, Immutable Audit Store for phase-mirror-agent

- Status: accepted
- Date: 2026-08-08
- Owners: Multiplicity Foundation
- Tags: #audit, #archivum, #append-only, #wal, #immutability, #l0
- Phase: phase-1 (master plan ADR-004)
- Related: ADR-004 (master), ADR-002 (witness schema), ADR-007 (flush on shutdown)

## 1. Context

The audit surface of `phase-mirror-agent` is not durable or immutable:

- `AuditStore` lives in memory (`RwLock<Vec<AuditEntry>>` in `src/audit_api.rs`); every
  entry is lost on restart.
- `DELETE /audit/entries/{id}` mutates history, contradicting the append-only audit
  intent required by the Sedona Spine Mandate and L0 invariants.
- Entry IDs come from an atomic counter that resets on restart → duplicate IDs after
  recovery.
- On the TS side, `archivum.ts` appends witnesses to JSONL with no fsync, no hash
  chaining, and `generateWitnessId()` uses `Math.random()`.
- `.gitignore` excludes `state/`, so the archive is neither versioned nor backed up.

## 2. Decision

Replace the in-memory store with a **durable, append-only, hash-chained JSONL WAL**
backed by the filesystem, and mirror that behavior in the TS `archivum` module.

### 2.1 Rust audit store (`src/audit/store.rs`)

- `AuditEntry` gains: `prev_hash` (hex), `entry_hash` (hex, SHA-256 over
  `prev_hash || canonical_json(entry_without_hash)`), `sequence` (monotonic).
- Storage: append-only JSONL under `PHASE_MIRROR_STATE_DIR/audit/wal.jsonl` (default
  `./state/audit/wal.jsonl`).
- Every write: serialize → `fsync` (flush) → append → `fsync` (directory on open). A
  write is acknowledged only after durable append.
- On boot: replay WAL, rebuild index, continue sequence (no counter reset).
- Immutability: no update/delete operations exist. A `DELETE` route is **removed**;
  documented `X-Audit-Retention` only via rotation (archival copy + truncation script,
  never in-place edit of existing entries).

### 2.2 API changes (`src/audit_api.rs`)

- Keep: `GET /audit/entries`, `GET /audit/entries/{id}`, `POST /audit/entries`,
  `GET /audit/summary`, `GET /audit/health`.
- Remove: `DELETE /audit/entries/{id}`.
- Add: `GET /audit/integrity` → returns `{ valid: bool, entries: n, head_hash }` by
  recomputing the chain.
- Add: `POST /audit/verify` (or extend integrity) to verify a range against the chain.

### 2.3 TS parity (`alp-nlp/archivum.ts`, `alp-nlp/witness.ts`)

- `generateWitnessId()` switches to `crypto.randomUUID()` / webcrypto CSPRNG.
- `writeWitness()` fsyncs (via `filehandle.sync()`) and stores `prev_hash` + `entry_hash`
  computed with SHA-256 (`node:crypto`), matching the Rust chain algorithm exactly.
- Chain algorithm is the single source of truth defined in a JSON Schema (see ADR-004
  Phase 1) so Rust and TS cannot drift.

### 2.4 Backup & rotation

- `scripts/audit-backup.sh`: copy + hash-verify `wal.jsonl` to a timestamped archive.
- `scripts/audit-rotate.sh`: archive current WAL atomically, start a new one whose first
  entry carries the previous head hash (chain continuity across rotation).

## 3. Implementation Plan

**Phase:** Phase 1 of ADR-004.

**Target Artifacts:**
- `src/audit/store.rs` — WAL store with chaining + fsync
- `src/audit_api.rs` — wire store, remove DELETE, add `/audit/integrity`
- `src/alp-nlp/archivum.ts`, `src/alp-nlp/witness.ts` — CSPRNG + chain parity
- `scripts/audit-backup.sh`, `scripts/audit-rotate.sh`
- `schema/audit.schema.json`, `schema/witness.schema.json` — versioned contracts

**Acceptance Criteria:**
- [ ] Entries written before a simulated crash are present after restart (kill -9 test).
- [ ] `GET /audit/integrity` reports `valid: true`; tampering one line flips it to `false`.
- [ ] `DELETE /audit/entries/{id}` returns 404 (route removed).
- [ ] TS witness chain verifies against Rust algorithm on the same sample set.
- [ ] Rotation preserves chain continuity (head hash of new WAL == tail of previous).

## 4. Consequences

### Positive
- Immutable, verifiable audit trail that survives restart — core governance property.
- Duplicate-ID bug eliminated via durable monotonic sequence.
- Single chain algorithm shared by Rust and TS prevents schema drift (ADR-002).

### Negative / Tradeoff
- Every audit write pays an fsync; at expected local volumes this is negligible, but
  the batch flush strategy (fsync every N ms when under high load) may be tuned later.
- Breaking change: existing in-memory entries are not migrated; deploy clears stale state
  or migrates via script.

### Neutral
- `DELETE` removal changes the API contract; consumers must be updated.

## 5. Security & Governance

1. **Immutable Audit** — no code path can modify or delete a recorded entry.
2. **Non-Bypassability** — all admission writes go through `AuditStore::append`; handlers
   cannot touch the file directly.
3. **Zero Drift** — chain algorithm shared via schema; integrity endpoint proves it.

## 6. Dependencies

- `sha2` / `sha256` crate, `serde`, `serde_json` (already present).
- `PHASE_MIRROR_STATE_DIR` config (ADR-006/009 own the config schema).
- ADR-007 owns flush-on-shutdown interplay.

## 7. Promotion Criteria

| Criteria | Target / Threshold | Status |
| :--- | :--- | :--- |
| Durability | Entries survive restart | ⬜ |
| Immutability | No mutation endpoints | ⬜ |
| Integrity | `/audit/integrity` verifies chain; tamper detected | ⬜ |
| TS parity | Witness chains cross-verify | ⬜ |
| Backup | Backup/restore round-trip verified | ⬜ |
