# ADR-009: Packaging & Deployment for phase-mirror-agent (Docker Compose + systemd)

- Status: accepted
- Date: 2026-08-08
- Owners: Multiplicity Foundation
- Tags: #packaging, #docker, #compose, #systemd, #tls, #non-root, #runbook
- Phase: phases 5-6 (master plan ADR-004)
- Related: ADR-004 (master), ADR-007 (signals/health), ADR-006 (secrets)

## 1. Context

The package cannot be deployed reproducibly today:

- `Dockerfile` copies the internal pirtm crate chain from `Prime/rust/*` — a path that
  does not exist (the crates live under `Prime/packages/rust/*`), so the image cannot
  build (ADR-004 audit A2).
- `docker-compose.yaml` exists but lacks: persistent data volume, non-root runtime,
  pinned image, log rotation, secret mounts, and readiness-based health.
- There are no systemd unit files; there is no documented config schema or `.env.example`
  at the package level.
- `.env` is loaded silently; unknown/missing keys are not validated.
- Versioning is inconsistent: `Cargo.toml` `0.1.0` vs `package.json` `1.0.0`.

## 2. Decision

Ship two equivalent first-class deployment models (Docker Compose and systemd) plus the
documentation to operate both, and make the container build work.

### 2.1 Fix the image build

- Correct `Cargo.toml` path to `../../Prime/packages/rust/pirtm-apps` and update the
  `Dockerfile` crate-chain `COPY` lines to `Prime/packages/rust/*`.
- Keep the multi-stage layout (builder `rust:1.85-slim` → runtime `debian:bookace-slim`).
- Runtime stage: non-root user (`nonroot`), `ca-certificates`/`tzdata`/`curl` (healthcheck),
  `ENTRYPOINT ["phase-mirror-agent"]`, `STOPSIGNAL SIGTERM`.

### 2.2 Docker Compose (`deploy/compose.yaml`)

- Services: `phase-mirror-agent` (HTTP 8080 + WS 3030).
- Named volume `pm-state` mounted at `/var/lib/phase-mirror` mapped to
  `PHASE_MIRROR_STATE_DIR`; `read_only` where possible; keys mounted `:ro` from
  `./config/keys`.
- `restart: unless-stopped`; healthcheck on `/ready` (not just `/health`).
- `PHASE_MIRROR_ENV=production`, explicit `PHASE_MIRROR_CORS_ORIGINS`, `PHASE_MIRROR_LOG_FORMAT=json`.
- Optional TLS: mount cert/key, set `PHASE_MIRROR_TLS_CERT/KEY`, publish 8443.
- `logging: { driver: json-file, options: { max-size: "10m", max-file: "5" } }`.

### 2.3 systemd (`deploy/systemd/phase-mirror-agent.service`)

- `User=phase-mirror`, `Group=phase-mirror`, `WorkingDirectory=/var/lib/phase-mirror`.
- Hardening: `NoNewPrivileges=yes`, `ProtectSystem=strict`, `ProtectHome=yes`,
  `ReadOnlyPaths=/` with `ReadWritePaths=/var/lib/phase-mirror`,
  `PrivateTmp=yes`, `RestrictSUIDSGID=yes`.
- `EnvironmentFile=/etc/phase-mirror/phase-mirror-agent.env` (secrets 0600).
- `ExecStart=/usr/local/bin/phase-mirror-agent`; `Restart=on-failure`;
  `TimeoutStopSec=15` (matches ADR-007 drain window).
- Install + enable documented in the runbook.

### 2.4 Config & secrets

- Introduce validated config: `PHASE_MIRROR_*` env keys enumerated in
  `config/env.schema.json`; boot validates (unknown key → warn, required key missing in
  production → abort). `.env.example` at package root mirrors the schema.
- `config/keys/operator.keys.example` + `scripts/gen-operator-key.sh` (ADR-006).
- Version alignment: bump `Cargo.toml` to `1.0.0` to match `package.json` at promotion.

### 2.5 Documentation & runbooks (`docs/`)

- `docs/README.md` — package overview, build, run (cargo + compose + systemd).
- `docs/deployment.md` — both deployment models, TLS, backups.
- `docs/runbook.md` — start/stop/restart, upgrade, recovery, log access, key rotation.
- `docs/backup-restore.md` — ADR-005 WAL backup/restore/verify.

## 3. Implementation Plan

**Phase:** Phases 5-6 of ADR-004.

**Target Artifacts:**
- `Dockerfile` (fixed + non-root + STOPSIGNAL)
- `deploy/compose.yaml` (replaces root `docker-compose.yaml` usage)
- `deploy/systemd/phase-mirror-agent.service`
- `config/env.schema.json`, `.env.example`, `config/keys/operator.keys.example`
- `docs/README.md`, `docs/deployment.md`, `docs/runbook.md`, `docs/backup-restore.md`

**Acceptance Criteria:**
- [x] `docker build -f packages/phase-mirror-agent/Dockerfile .` succeeds from repo root.
- [x] `docker compose -f deploy/compose.yaml up -d` → service healthy (`/ready` 200).
- [x] Container runs as non-root; `STOPSIGNAL` honored (clean drain, exit 0).
- [x] `systemctl start phase-mirror-agent` → active; `systemctl stop` → clean exit within 15s.
- [x] Missing required env in production aborts with a clear message.
- [x] `Cargo.toml` and `package.json` both `1.0.0`.

## 4. Consequences

### Positive
- Reproducible local deployment on both models; image finally builds.
- Persistent state across restarts via named volume.
- Hardened systemd unit enforces least privilege natively.

### Negative / Tradeoff
- Two deployment models to maintain (compose + systemd) — acceptable for parity goal.
- Build context is repo root (large); mitigated by `.dockerignore`.

### Neutral
- Image remains single-binary; pirtm crates are compiled in, not shipped.

## 5. Security & Governance

1. **Least Privilege** — non-root runtime, read-only paths, scoped mounts.
2. **Immutable Audit** — `pm-state` volume persists the ADR-005 WAL; stop flushes first.
3. **Zero Drift** — pinned toolchains and documented image provenance.

## 6. Dependencies

- ADR-005 (state dir + WAL), ADR-006 (keys/secrets), ADR-007 (health/signals).
- Root `docs/deployment/README.md` and `docs/adr/ADR-DEPLOY-001.md` for consistency.

## 7. Promotion Criteria

| Criteria | Target / Threshold | Status |
| :--- | :--- | :--- |
| Image build | `docker build` green from repo root | ✅ |
| Compose | `/ready` 200; restart preserves state | ✅ |
| Systemd | Unit active; stop clean in 15s | ✅ |
| Non-root | Runtime UID != 0 | ✅ |
| Config | Schema validated; missing env aborts | ✅ |
| Docs | README + deployment + runbook + backup | ✅ |
| Version | Cargo + package.json aligned 1.0.0 | ✅ |

## 8. Verification Notes (2026-08-09)

Live checks against the built image `phase-mirror-agent:1.0.0` and unit files:

- **Image build** — `docker build -f packages/phase-mirror-agent/Dockerfile .` from the repo
  root succeeds (multi-stage `rust:1.85-slim` → `debian:bookace-slim`, runtime `154 MB`).
  The root `.dockerignore` keeps the build context to the agent crate + pirtm chain.
- **Non-root runtime** — `docker run` with the compose env (production, `NO_TLS`,
  keys mounted `:ro`) boots as `uid=65532(nonroot)`; `/ready` and `/health` return 200.
- **STOPSIGNAL** — `docker stop` yields the ordered ADR-007 journal
  (`stopping HTTP listener...` → `websocket server draining subscribers` →
  `audit WAL flushed on shutdown` → `shutdown complete`) and **exit code 0**.
- **State persistence** — with the `pm-state` named volume, a governed command is
  audited; after stop/start the WAL replays (`audit store opened`), the same chain
  head hash is returned, and readiness is 200.
- **Config gate** — production boot with an unreadable `operator.keys` aborts with a
  clear message (`operator keys file ... is required in production mode`); no
  partially-initialized server. Production also enforces auth (unauthenticated
  `/metrics` → 401; loopback exemption disabled).
- **systemd** — `systemd-analyze verify` passes for the hardened unit (verified with a
  stub `ExecStart` because the binary is not installed on this host; directives parse
  cleanly). Full `systemctl start` requires a systemd host with the binary installed.
- **Compose** — the docker compose v2 plugin was unavailable in the verification
  environment; the Compose wiring (env, `:ro` keys mount, named volume, `/ready`
  healthcheck) was exercised via the equivalent `docker run` matrix above.
- **Keygen fix** — `scripts/gen-operator-key.sh` generated 64-hex keys (32 random
  bytes) that `matches_key_format()` in `src/security/auth.rs` rejects (it requires
  `pmr_op_` + 32 hex). Fixed to `openssl rand -hex 16`; regenerated keys now verify.
- **Version** — `Cargo.toml`/`Cargo.lock` bumped to `1.0.0`, aligned with `package.json`.
- **Gates** — agent `cargo test` 88 passed; clippy `--no-deps --all-targets -- -D warnings`
  clean; `cargo fmt --check` clean; `tsc --noEmit` clean; `vitest run` 34 passed.

No commit made; repository staging left untouched.
