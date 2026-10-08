# phase-mirror-agent

**High-integrity governance gateway for agentic AI workflows.**

`phase-mirror-agent` is a Rust server that accepts governed operational commands,
compiles them through the Prime-indexed ALP invariant gates (via `pirtm-apps`),
executes them through an allow-listed tool-execution backplane, and records every
decision in a durable, hash-chained, immutable audit WAL. A TypeScript core
(`src/alp-nlp/*`) mirrors the chain algorithm as a single source of truth.

## Current Status

Production-deployment series (ADR-004) — **Phases 1–6 complete and verified**:

| Phase | Deliverable | Owner ADR | Status |
| :-- | :-- | :-- | :-- |
| 1 | Durable, immutable audit store (hash-chained JSONL WAL + rotation) | ADR-005 | ✅ |
| 2 | Governance correctness: real `c`/`R_sc`/witnesses, tool backplane, no stubs | ADR-008 | ✅ |
| 3 | Security: authn/authz (scoped bearer keys), rate limiting, TLS posture | ADR-006 | ✅ |
| 4 | Observability & reliable shutdown: JSON logs, `/metrics`, health split, SIGTERM drain | ADR-007 | ✅ |
| 5 | Packaging & deployment (Compose + systemd, non-root image) | ADR-009 | ✅ |
| 6 | Documentation & runbooks (deployment, runbook, backup-restore) | ADR-009 | ✅ |
| 7 | Promotion & GA (v1.0.0 tag) | all | ✅ |

Every gate is green: `cargo test` (88), clippy `-D warnings`, `cargo fmt --check`,
`tsc --noEmit`, `vitest run` (34). The `phase-mirror-agent:1.0.0` image builds and
boots as a non-root user with state persisted on a named volume. See
[docs/adr](docs/adr/README.md) for the full ADR index and verification notes.

## Architecture

```
Client (Claude/GPT, operator CLI, web UI)
   │  HTTPS / WSS
   ▼
┌─────────────────────────── phase-mirror-agent ───────────────────────────┐
│                                                                           │
│  HTTP router (axum 0.7, axum-server + optional TLS)                       │
│   ├─ observe layer      → request log + pm_http_requests_total            │
│   ├─ CORS (explicit allow-list; wildcard banned in production)            │
│   ├─ /health  (liveness, open)                                            │
│   ├─ /ready   (readiness: audit WAL writable + chain head parses)         │
│   ├─ /metrics (Prometheus text; authn operator:read)                      │
│   ├─ /api/command (authn → authz → rate-limit → CNL pipeline)             │
│   ├─ /audit/*  (append-only store; verify / rotate / query)               │
│   └─ /ws       (operator event stream, config-gated auth)                 │
│                                                                           │
│  CNL pipeline: compile_command (pirtm-apps) → invariants → plan → invoke   │
│  Tool backplane: allow-listed adapters (compose, systemd, simulated)       │
│  Audit store: append-only JSONL WAL, SHA-256 hash chain, fsync per write   │
│  Observability: tracing-subscriber (json|text), atomic metrics registry    │
└───────────────────────────────────────────────────────────────────────────┘
```

## Repository Layout

```
src/
├── main.rs               # CLI, wiring, SIGTERM/SIGINT orchestration
├── audit_api.rs          # /audit/* HTTP surface (append, query, verify, rotate)
├── health.rs             # /health (liveness) + /ready (readiness)
├── ws_broadcast.rs       # operator WebSocket stream + cooperative shutdown
├── cnl_bridge.rs         # VerifiedAction → ToolRequest / ExecutionPlan / witness
├── observability/
│   ├── mod.rs            # log init (json|text) + request observer
│   └── metrics.rs        # counter registry + /metrics Prometheus text
├── security/
│   ├── auth.rs           # scoped bearer keys (SHA-256 at rest), middleware
│   ├── ratelimit.rs      # token-bucket rate limiter (429 + Retry-After)
│   ├── env.rs            # validated .env loader (unknown keys rejected)
│   └── redact.rs         # secret redaction for logs
├── audit/
│   └── store.rs          # immutable JSONL WAL, hash chain, rotation, replay
├── executor/
│   ├── mod.rs            # ToolRequest / Receipt / ExecError / ToolRegistry
│   ├── config.rs         # tools.toml allow-lists and bounds
│   └── adapters/         # compose, systemd, simulated executors
└── alp-nlp/              # TypeScript core (lexer, parser, policy, witness, chain, archivum)
deploy/                   # compose.yaml + systemd unit (ADR-009)
docs/                     # deployment guide, runbook, backup-restore, ADRs, dev log
config/                   # env.schema.json, keys/ (0600), tools.toml.example
schema/                   # audit / receipt / tool_request / witness JSON schemas
scripts/                  # audit-backup, audit-rotate, export-schema, gen-operator-key
.env.example              # environment template (mirrors config/env.schema.json)
```

## Build & Verify

```bash
# Rust server
cargo test
cargo clippy --no-deps --all-targets -- -D warnings
cargo fmt --check

# TypeScript core (single source of truth for the chain algorithm)
npx tsc --noEmit
npx vitest run

# Release binary
cargo build --release
```

## Deploy

Two equivalent deployment models (ADR-009), documented in full:

- **[docs/deployment.md](docs/deployment.md)** — Docker Compose (`deploy/compose.yaml`,
  non-root image, persistent `pm-state` volume, `/ready` healthcheck) and systemd
  (hardened unit in `deploy/systemd/`).
- **[docs/runbook.md](docs/runbook.md)** — start/stop/restart, logs, key rotation, upgrades.
- **[docs/backup-restore.md](docs/backup-restore.md)** — audit WAL backup/restore/verify.

Quick start with Compose:

```bash
cp .env.example .env            # optional local tweaks
scripts/gen-operator-key.sh alice operator:write   # provision a real key
docker compose -f deploy/compose.yaml up -d
curl -fsS http://localhost:8080/ready
```

## Run

```bash
cargo run -- \
  --state-dir ./state \
  --operator-keys "alice:operator:write:pmr_op_<32-hex>" \
  --bind 0.0.0.0:8080 --ws-port 3030
```

Configuration is environment-first (`PHASE_MIRROR_*`, loaded from `.env` when
present; unknown keys abort boot). Selected variables:

| Variable | Default | Purpose |
| :-- | :-- | :-- |
| `PHASE_MIRROR_BIND` | `0.0.0.0:8080` | HTTP listen address |
| `PHASE_MIRROR_WS_PORT` | `3030` | WebSocket broadcast port |
| `PHASE_MIRROR_STATE_DIR` | `./state` | Durable runtime state (audit WAL) |
| `PHASE_MIRROR_ENV` | `dev` | `dev` \| `production` (TLS + CORS enforcement) |
| `PHASE_MIRROR_TLS_CERT` / `PHASE_MIRROR_TLS_KEY` | — | PEM cert/key (production requires TLS or `--no-tls`) |
| `PHASE_MIRROR_OPERATOR_KEYS` / `_FILE` | — | `id:scope:secret` keyring (scopes: `operator:read`, `operator:write`) |
| `PHASE_MIRROR_RATE_LIMIT_RPS` / `_BURST` | `10` / `20` | Per-client rate limiting |
| `PHASE_MIRROR_WS_AUTH` | `false` | Require bearer key on WS upgrades |
| `PHASE_MIRROR_LOOPBACK_EXCEPTION` | `true` (dev) | Unauthenticated loopback reads (never writes; forced off in production) |
| `PHASE_MIRROR_LOG_FORMAT` | `text` | `text` \| `json` |
| `PHASE_MIRROR_LOG_LEVEL` | `info` | `trace` \| `debug` \| `info` \| `warn` \| `error` |
| `PHASE_MIRROR_NO_TLS` | `false` | Explicitly allow plain HTTP in production (loopback/local only) |
| `PHASE_MIRROR_TOOL_CONFIG` / `PHASE_MIRROR_TOOL_ALLOW` | — | `tools.toml` bounds and enabled adapter families |

## HTTP API

| Method | Path | Auth | Purpose |
| :-- | :-- | :-- | :-- |
| GET | `/health` | open | Liveness — always 200 while serving |
| GET | `/ready` | open | Readiness — 503 when the audit WAL is unwritable or the chain head fails to parse |
| GET | `/metrics` | read | Prometheus text metrics |
| POST | `/api/command` | write | Governed command → receipt + witness (idempotency-key dedup, dry-run) |
| GET/POST | `/audit/entries` | read / write | List / append audit entries |
| GET | `/audit/entries/{id}`, `/audit/summary`, `/audit/health`, `/audit/integrity` | read | Query & integrity |
| POST | `/audit/verify`, `/audit/rotate` | write | Range verification / WAL rotation |
| GET | `/ws` | config-gated | Operator event stream (user_input → … → execution_success/failure) |

## Observability (ADR-007)

- **Logs** — `PHASE_MIRROR_LOG_FORMAT=json` emits one JSON line per event with
  `ts` (RFC3339), `level`, `target`, `msg`, `service=phase-mirror-agent`, `pid`; every HTTP
  request adds `method`, `path`, `status`, `latency_ms`, `session_id`.
- **Metrics** (`GET /metrics`) — `pm_commands_total{outcome=admitted|vetoed|error}`,
  `pm_audit_writes_total`, `pm_audit_writes_latency_seconds`, `pm_ws_clients_current`,
  `pm_ws_events_broadcast_total`, `pm_http_requests_total{method,status}`.
- **Shutdown** — SIGTERM/SIGINT: stop accepting → drain HTTP (10 s grace) → broadcast
  `execution_terminated` + drain WebSocket subscribers → flush + fsync the audit WAL →
  exit `0`. No truncated audit entries.

## Security & Integrity (ADR-005/006/008)

- Append-only audit: every entry is SHA-256 hash-chained to its predecessor and fsynced
  before acknowledgement; replay on boot rejects a broken chain.
- Writes require an `operator:write` bearer key; the caller-supplied `actor` is never trusted
  on the write path. Rate limits apply per client; loopback exemption covers reads only.
- Tool execution is fail-closed: only adapter families in `PHASE_MIRROR_TOOL_ALLOW` (bounded by
  `tools.toml`) can run; everything else returns an explicit rejection.

## Development Log

See [docs/DEVELOPMENT-LOG.md](docs/DEVELOPMENT-LOG.md) for a phase-by-phase account of the
Phases 1–6 build-out (2026-08-08 → 2026-08-09), including live verification for each phase.
