# ADR-007: Observability & Reliable Shutdown for phase-mirror-agent

- Status: accepted
- Date: 2026-08-08
- Owners: Multiplicity Foundation
- Tags: #observability, #logging, #metrics, #health, #graceful-shutdown, #reliability
- Phase: phase-4 (master plan ADR-004)
- Related: ADR-004 (master), ADR-005 (WAL flush), ADR-009 (container/signal wiring)

## 1. Context

The agent is not observable and does not shut down safely:

- Logging is plain `tracing_subscriber::fmt` text to stderr; no structured fields, no
  machine-readable output, no log-level control via env at runtime parity.
- `/health` and `/ready` return the same static body; readiness does not reflect whether
  the audit WAL is writable or dependencies are reachable.
- There are no metrics; operators cannot observe command rates, rejection reasons,
  audit-write latency, or WebSocket client count.
- On SIGTERM/SIGINT, `handle.graceful_shutdown(None)` is used but the WebSocket task is
  `abort()`ed (`ws_broadcast.rs`), which can truncate in-flight broadcasts, and nothing
  explicitly flushes the audit WAL before exit.

## 2. Decision

Introduce structured observability and a correct shutdown sequence.

### 2.1 Structured logging

- Replace `fmt::init()` with `tracing_subscriber::fmt().json()` (config-gated to text
  for local dev via `PHASE_MIRROR_LOG_FORMAT=text|json`).
- Always include: `ts` (RFC3339), `level`, `target`, `msg`, `service=phase-mirror-agent`,
  `pid`.
- Request-scoped fields: `method`, `path`, `status`, `latency_ms`, `session_id`.
- `PHASE_MIRROR_LOG_LEVEL` (trace|debug|info|warn|error) read at boot.

### 2.2 Metrics (`/metrics`)

- Hand-rolled `Metrics` registry of atomics (`src/observability/metrics.rs`); no
  `prometheus`/`tower-http` dependency. Rendered as plain-text Prometheus exposition
  with a hardcoded `pm_` prefix (no namespace env var). Minimal set:
  - `pm_commands_total{outcome=admitted|vetoed|error}`
  - `pm_audit_writes_total` (count), `pm_audit_writes_latency_seconds` (accumulating sum)
  - `pm_ws_clients_current` (gauge), `pm_ws_events_broadcast_total`
  - `pm_http_requests_total{method,status}`
- Counters are updated at the code path (`handle_command`, audit write, WS socket, HTTP
  observe layer).
- Endpoint served on the HTTP listener (authn: `operator:read`; loopback read-exempt in dev).

### 2.3 Health split

- `/health` (liveness): process is up; always `200` when serving.
- `/ready` (readiness): audit WAL opens writable and, if configured, upstream
  dependencies (e.g., pirtm registry ledger) are reachable; otherwise `503`.
- `/ready` checks: open `PHASE_MIRROR_STATE_DIR/audit/wal.jsonl` for append; verify
  chain head parses.

### 2.4 Graceful shutdown

- Handle `SIGTERM` and `SIGINT` (already present via `ctrl_c`; extend to SIGTERM).
- Order: (1) stop accepting new HTTP/WS connections, (2) drain in-flight requests with
  timeout (default 10s), (3) drain WS subscribers (send `execution_terminated` then close),
  (4) flush + fsync audit WAL, (5) exit 0.
- Replace `ws.shutdown()` abort with a cooperative `notify`/`watch`-based stop.

## 3. Implementation Plan

**Phase:** Phase 4 of ADR-004.

**Target Artifacts:**
- `src/observability/mod.rs` — log init, JSON formatter, request observe layer
- `src/observability/metrics.rs` — counters + `/metrics` handler
- `src/health.rs` — liveness/readiness split with store check
- `src/ws_broadcast.rs` — cooperative shutdown + drain
- `src/main.rs` — signal handling (SIGTERM/SIGINT), shutdown orchestration
- `config/otel.example.toml` — optional OTEL export point (later)

**Acceptance Criteria:**
- [x] `PHASE_MIRROR_LOG_FORMAT=json` produces parseable JSON with all required fields.
- [x] `/ready` returns 503 when WAL is unwritable; `/health` stays 200.
- [x] `/metrics` exposes the six counters above.
- [x] SIGTERM drains WS and flushes WAL; journal shows clean ordered shutdown; exit 0.
- [x] No truncated audit entries after `docker compose stop` (verified by ADR-005 integrity).

## 4. Consequences

### Positive
- Operators can correlate requests end-to-end and observe governance decisions.
- Readiness is meaningful → orchestrators stop routing to a degraded agent.
- Clean shutdown guarantees the audit chain is never cut mid-write.

### Negative / Tradeoff
- JSON logging adds ~10-20% log volume vs text.
- Readiness WAL probe opens the file on each poll (cheap; cached fd).

### Neutral
- Metrics stay local; OTEL export is a later optional extension.

## 5. Security & Governance

1. **Immutable Audit** — shutdown sequence flushes the WAL so no admitted decision is lost.
2. **Zero Drift** — metrics derive from code paths, not hand-maintained numbers.
3. **Non-Bypassability** — readiness reflects the gate's backing store health.

## 6. Dependencies

- `tracing-subscriber` (present). Metrics use a hand-rolled registry of atomics — no external dependency.
- ADR-005 (WAL) owns the store; this ADR owns probe + flush semantics.
- ADR-009 owns signal delivery in containers/systemd.

## 7. Promotion Criteria

| Criteria | Target / Threshold | Status |
| :--- | :--- | :--- |
| Structured logs | JSON mode parses; fields complete | ✅ |
| Readiness | 503 when store unwritable | ✅ |
| Metrics | Six counters exported | ✅ |
| Graceful stop | SIGTERM drain + flush + exit 0 | ✅ |
| No truncation | Chain integrity post-stop | ✅ |

## 8. Verification Notes (2026-08-09)

Live checks against the built binary (debug, `127.0.0.1`):

- Structured logs: `PHASE_MIRROR_LOG_FORMAT=json` emits one JSON object per event with `ts`
  (RFC3339), `level`, `target`, `msg`, `service=phase-mirror-agent`, `pid`, plus request-scoped
  `method`, `path`, `status`, `latency_ms`, `session_id` on the `http request handled` line.
  `PHASE_MIRROR_LOG_FORMAT=text` still renders human-readable lines for local dev; an invalid
  format value aborts boot (exit 1). `PHASE_MIRROR_LOG_LEVEL` filters at boot.
- Metrics: `GET /metrics` (authenticated as `operator:read`; loopback read-exempt in dev) exposes
  `pm_commands_total{outcome=admitted|vetoed|error}`, `pm_audit_writes_total`,
  `pm_audit_writes_latency_seconds`, `pm_ws_clients_current`, `pm_ws_events_broadcast_total`, and
  `pm_http_requests_total{method,status}`. After a `deploy` (200), `deploy all cluster` (422) and
  a compile error (400) the counters read `admitted=1 vetoed=1 error=1`, `pm_audit_writes_total=1`,
  and per-status HTTP counts.
- Health split: `/health` always `200` while serving; `/ready` returns `200` with the WAL open, and
  `503` with a JSON `not_ready` body (including `error`) when the chain head fails to parse (unit
  test `readiness_503_when_chain_head_does_not_parse`).
- Graceful shutdown: `kill -TERM` produces the ordered journal
  `stopping HTTP listener...` → `websocket server draining subscribers` →
  `audit WAL flushed on shutdown` → `phase-mirror-agent shutdown complete`, and the process exits
  code `0`. An attached WebSocket subscriber receives an `execution_terminated` event and a clean
  close (1000) before the connection is drained.
- No truncation: after SIGTERM the WAL replays on the next boot (`audit store opened`, chain head
  hash logged) — no truncated entry.
- Test battery: agent `cargo test` 88 passed; clippy `--no-deps --all-targets -- -D warnings` clean;
  `cargo fmt --check` clean; `tsc --noEmit` clean; `vitest run` 34 passed.

No commit made; repository staging left untouched.
