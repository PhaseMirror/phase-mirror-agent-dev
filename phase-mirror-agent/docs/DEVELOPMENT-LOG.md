# Development Log — Phase Mirror Agent

Phase-by-phase account of the production-grade build-out of `phase-mirror-agent`
per the ADR-004 master plan. Each phase is owned by a child ADR; acceptance is
recorded with live verification notes in the owning ADR and summarized here.

## Instance Summary

- **Window:** 2026-08-08 → 2026-08-09
- **Outcome:** Phases 1–6 of ADR-004 are **complete and verified** (ADR-005,
  ADR-006, ADR-007, ADR-008, ADR-009 all `accepted`). Phase 7 (promotion/GA:
  final audit + `v1.0.0` tag) remains open.
- **Gate status (Phase-6 close):** `cargo test` 88 passed · clippy
  `--no-deps --all-targets -- -D warnings` clean · `cargo fmt --check` clean ·
  `tsc --noEmit` clean · `vitest run` 34 passed.
- **Working agreement:** no commits made; repository staging untouched.

---

## Phase 1 — Durable, Immutable Audit Store (Owner: ADR-005)

Shipped previously; accepted. Replaced the in-memory `AuditStore` with an
append-only JSONL WAL that is fsynced on every write and SHA-256 hash-chained
entry-to-entry, with on-disk metadata (`wal.meta.json`), replay-on-open with
chain verification, runtime rotation/archival, and a TypeScript `archivum` /
`chain.ts` parity implementation. `DELETE` mutation endpoints were removed.

Key files: `src/audit/store.rs`, `src/audit_api.rs`, `src/alp-nlp/chain.ts`,
`src/alp-nlp/archivum.ts`, `scripts/audit-backup.sh`, `scripts/audit-rotate.sh`.

---

## Phase 2 — Governance Correctness (Owner: ADR-008)

Shipped previously; accepted. Removed hardcoded CNL outputs and introduced a
governed tool-execution backplane: `compile_command` (pirtm-apps) produces real
`c` / `R_sc` / invariant diagnostics, plans map verified actions to allow-listed
adapter families (`compose`, `systemd`, plus an explicit `simulated` adapter),
and every admitted action yields a real SHA-256 witness hash. Fail-closed
allow-lists guarantee no stub paths.

Key files: `src/cnl_bridge.rs`, `src/executor/*`, `config/tools.toml.example`,
`schema/*.json`.

---

## Phase 3 — Security: Authn/Authz, Rate Limiting, TLS (Owner: ADR-006)

Shipped previously; accepted. Added scoped bearer-key authentication
(`operator:read` / `operator:write`, SHA-256 digests at rest, keys file with
`0600` permissions), token-bucket rate limiting (`429` + `Retry-After`), CORS
with an explicit origin allow-list (wildcard banned in production), a validated
`.env` loader that rejects unknown `PHASE_MIRROR_*` keys, a config-gated
`--ws-auth` WebSocket upgrade, and production TLS posture (certs required or an
explicit `--no-tls`). The loopback read exemption applies to reads only and is
forced off in production. Authenticated actor attribution is enforced on every
write path.

Key files: `src/security/auth.rs`, `src/security/ratelimit.rs`,
`src/security/env.rs`, `src/security/redact.rs`, `src/ws_broadcast.rs`.

---

## Phase 4 — Observability & Reliable Shutdown (Owner: ADR-007)

Shipped this instance. Detailed account of the work and verification follows.

### Structured logging (§2.1)

- Replaced `tracing_subscriber::fmt::init()` with a config-gated initializer in
  the new `src/observability/mod.rs`: `PHASE_MIRROR_LOG_FORMAT=text|json` and
  `PHASE_MIRROR_LOG_LEVEL=trace|debug|info|warn|error`, both read at boot.
- JSON mode emits one line per event with `ts` (RFC3339), `level`, `target`,
  `msg`, `service=phase-mirror-agent`, `pid` via a custom `FormatEvent`
  (`JsonEventFormatter`). Request-scoped fields are added by the
  `observe_request` middleware: `method`, `path`, `status`, `latency_ms`,
  `session_id`.
- An invalid `PHASE_MIRROR_LOG_FORMAT` aborts boot (exit 1) rather than
  silently degrading.

### Metrics (§2.2)

- New `src/observability/metrics.rs`: a hand-rolled atomic `Metrics` registry
  (no Prometheus dependency) exposing plain-text Prometheus exposition on
  `GET /metrics` (authenticated as `operator:read`, loopback read-exempt in dev):
  - `pm_commands_total{outcome=admitted|vetoed|error}`
  - `pm_audit_writes_total`, `pm_audit_writes_latency_seconds`
  - `pm_ws_clients_current`, `pm_ws_events_broadcast_total`
  - `pm_http_requests_total{method,status}`
- Instrumented write paths in `handle_command` (main.rs) and
  `create_audit_entry` (audit_api.rs, now stateful via `AuditApiState`),
  plus WS client/broadcast gauges in `ws_broadcast.rs`.

### Health split (§2.3)

- New `src/health.rs`: `/health` stays a pure liveness probe (always 200);
  `/ready` calls `AuditStore::check_ready()` (new in `src/audit/store.rs`),
  which re-opens `wal.jsonl` for append and verifies the chain head parses,
  returning `503` with a JSON `not_ready` body otherwise.

### Graceful shutdown (§2.4)

- `shutdown_signal()` in `main.rs` now handles both SIGINT and SIGTERM.
- HTTP: `handle.graceful_shutdown(Some(10s))` bounds the in-flight drain.
- WebSocket (`ws_broadcast.rs`): replaced the `handle.abort()` with a
  cooperative `watch`-channel stop — `WsServer::shutdown()` broadcasts an
  `execution_terminated` event, signals the serve task, and awaits it;
  `handle_socket` observes the watch, sends a clean close (1000), and exits.
- Audit: `AuditStore::flush()` (new) fsyncs the WAL, persists `wal.meta.json`,
  and fsyncs the directory; called before process exit.
- Sequence: stop accepting → drain HTTP (10 s) → drain WS → flush WAL → exit 0.

### Phase-4 verification (2026-08-09, against the debug binary)

- JSON logs parse with all required fields; request lines carry
  `method/path/status/latency_ms/session_id`; text mode still renders
  human-readable output; invalid format exits 1.
- `/metrics` exposes all six counters; after `deploy` (200), `deploy all cluster`
  (422), and a compile error (400): `admitted=1 vetoed=1 error=1`,
  `pm_audit_writes_total=1`, per-status HTTP counts present.
- `/health` 200 while serving; `/ready` 503 (unit-tested) when the chain head
  fails to parse.
- `kill -TERM` → clean ordered journal (`stopping HTTP listener...` →
  `websocket server draining subscribers` → `audit WAL flushed on shutdown` →
  `shutdown complete`) and **exit code 0**; an attached WS subscriber receives
  `execution_terminated` then a clean close.
- No truncation: the WAL replays cleanly on next boot after SIGTERM.

Full detail: ADR-007 §8 Verification Notes.

---

## Phase 5 — Packaging & Deployment (Owner: ADR-009)

Shipped this instance. Converted the package into a reproducibly deployable unit
and added both first-class deployment models.

### Image hardening (Dockerfile)

- Runtime stage now runs as **non-root** (`nonroot`, UID 65532) with the durable
  state dir `/var/lib/phase-mirror` owned by that user.
- `STOPSIGNAL SIGTERM` so `docker stop` triggers the ADR-007 drain; a
  `HEALTHCHECK` probes `/ready` (not just `/health`).
- A repo-root `.dockerignore` keeps the (repo-root) build context down to the
  agent crate + the `Prime/packages/rust` pirtm chain.

### Compose (`deploy/compose.yaml`)

- Replaces the old root `docker-compose.yaml`; the compose adapter default and
  `config/tools.toml.example` now point at `deploy/compose.yaml`.
- Named volume `pm-state` → `/var/lib/phase-mirror`, keys mounted `:ro`,
  `/ready` healthcheck, `restart: unless-stopped`, `stop_grace_period: 15s`,
  and json-file log rotation (10m × 5). Production env with explicit
  `PHASE_MIRROR_NO_TLS=true` to boot without certs (TLS path documented).

### systemd (`deploy/systemd/phase-mirror-agent.service`)

- Hardened unit: `NoNewPrivileges`, `ProtectSystem=strict`, `PrivateTmp`,
  `RestrictSUIDSGID`, read-only root with only `/var/lib/phase-mirror` writable,
  `TimeoutStopSec=15`, `Restart=on-failure`, `EnvironmentFile` for secrets.

### Config & versioning

- New `config/env.schema.json` (JSON Schema reference for every `PHASE_MIRROR_*`
  key with production-required annotations) and `.env.example` mirroring it.
- `PHASE_MIRROR_NO_TLS` added as an env alias for `--no-tls`; the `KNOWN_ENV_KEYS`
  allow-list gained `PHASE_MIRROR_LOG_FORMAT` / `PHASE_MIRROR_LOG_LEVEL` /
  `PHASE_MIRROR_NO_TLS`.
- `Cargo.toml`/`Cargo.lock` bumped to `1.0.0`, aligned with `package.json`.

### Bug found & fixed

- `scripts/gen-operator-key.sh` emitted 64-hex keys (`openssl rand -hex 32`)
  that `matches_key_format()` rejects (it requires `pmr_op_` + exactly 32 hex).
  Fixed to `openssl rand -hex 16`; regenerated keys now authenticate.

### Phase-5 verification (2026-08-09, against image `phase-mirror-agent:1.0.0`)

- `docker build -f packages/phase-mirror-agent/Dockerfile .` succeeds from the
  repo root (runtime image 154 MB).
- `docker run` with the Compose env boots as `uid=65532(nonroot)`; `/ready` and
  `/health` return 200; unauthenticated `/metrics` is 401 (production).
- `docker stop` → ordered drain journal + **exit code 0**.
- State persists: a governed command is audited, the container is stopped and
  restarted with the `pm-state` volume, and the same chain head hash is returned.
- Production config gate: unreadable `operator.keys` → clean abort with a clear
  message; missing/unknown env keys rejected.
- `systemd-analyze verify` passes for the unit (stub `ExecStart`; binary not
  installed on this host).
- The docker compose v2 plugin was unavailable here, so the Compose wiring was
  exercised via the equivalent `docker run` matrix.

Full detail: ADR-009 §8 Verification Notes.

---

## Phase 6 — Documentation & Runbooks (Owner: ADR-009)

Shipped this instance. Everything an operator needs to deploy, run, and recover:

- `README.md` (package) — status table now shows Phases 1–6 ✅, expanded repo
  layout, a Deploy quick start, and `PHASE_MIRROR_NO_TLS`.
- `docs/README.md` — documentation index.
- `docs/deployment.md` — both deployment models (Compose + systemd), TLS
  variants, health/logs/state table.
- `docs/runbook.md` — start/stop/restart, logs, readiness, metrics, key
  rotation, upgrade, recovery, security notes.
- `docs/backup-restore.md` — WAL backup (`scripts/audit-backup.sh`), verify
  (`/audit/integrity`, `/audit/verify`), restore, rotation, recovery table.
- `docs/adr/README.md` — ADR index + phase status updated; ADR-009 accepted.

---

## Test Inventory

| Suite | Count | Scope |
| :-- | :-- | :-- |
| `cargo test` | 88 | store/chain, audit API, auth, ratelimit, health, observability, ws, CLI/security end-to-end |
| `vitest run` | 34 | ALP lexer/parser/policy/compiler/chain/witness (TS core) |
| `tsc --noEmit` | — | TypeScript core type-safety |

The Rust and TypeScript test suites both verify the shared chain-hash algorithm
against a checked-in fixture (`tests/fixtures/ts-witnesses.jsonl`), keeping the
single-source-of-truth contract honest.

## Next Steps

- Phase 7 (promotion/GA): final promotion-criteria audit across ADR-004 §7
  (all rows now ✅ except Phase-7 owner rows), align remaining loose ends, and
  tag `v1.0.0`.
- Optional hardening: pin the builder base image to a digest for bit-reproducible
  builds; wire `docker compose` verification into CI once the plugin is available.
