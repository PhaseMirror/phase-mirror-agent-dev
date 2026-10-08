# phase-mirror-agent — Deployment Guide

This guide covers the two equivalent first-class deployment models for the
phase-mirror-agent governance gateway (ADR-009): **Docker Compose** and
**systemd**. Both run the same Rust binary from a clean build.

- [1. Prerequisites](#1-prerequisites)
- [2. Build](#2-build)
- [3. Configuration](#3-configuration)
- [4. Deploy with Docker Compose](#4-deploy-with-docker-compose)
- [5. Deploy with systemd](#5-deploy-with-systemd)
- [6. TLS](#6-tls)
- [7. Health, logs & state](#7-health-logs--state)
- [8. Next steps](#8-next-steps)

## 1. Prerequisites

- Rust 1.85+ (`cargo`, for a native build) **or** Docker Engine 24+ with
  Docker Compose v2 (for the container).
- systemd (systemd-based distro) for the native unit deployment.
- 1 GB RAM / 1 GB disk minimum; the release build needs ~2 GB free for the
  dependency graph.

## 2. Build

### Native binary

```bash
cd packages/phase-mirror-agent
cargo build --release
# binary at target/release/phase-mirror-agent
```

### Container image

The build context is the repository root (the image compiles the internal
`Prime/packages/rust` pirtm crates in). A root `.dockerignore` keeps the context
small:

```bash
cd <repo-root>
docker build -t phase-mirror-agent:1.0.0 \
  -f packages/phase-mirror-agent/Dockerfile .
```

The runtime image runs as a **non-root** user (UID 65532) and uses
`STOPSIGNAL SIGTERM` so `docker stop` triggers the ADR-007 drain sequence.

## 3. Configuration

All configuration is via `PHASE_MIRROR_*` environment keys (see
`config/env.schema.json` and `.env.example`). Start from the template:

```bash
cd packages/phase-mirror-agent
cp .env.example .env        # local dev; edit as needed
```

Operator keys are required for protected routes. Generate real keys with:

```bash
scripts/gen-operator-key.sh alice operator:write
scripts/gen-operator-key.sh carol operator:read
```

Append the printed hashed lines to `config/keys/operator.keys` and `chmod 600` it.

> **Production minimums:** `PHASE_MIRROR_ENV=production` plus either a TLS
> cert/key pair or an explicit `PHASE_MIRROR_NO_TLS=true`, and a non-empty
> operator keyring. The agent aborts at boot otherwise.

## 4. Deploy with Docker Compose

`deploy/compose.yaml` runs the agent with a named volume (`pm-state`) for durable
state, a `/ready` healthcheck, log rotation, and `restart: unless-stopped`:

```bash
cd packages/phase-mirror-agent
# 1. provision operator keys
#    (cp config/keys/operator.keys.example config/keys/operator.keys && chmod 600 ...)
#    or generate real ones with scripts/gen-operator-key.sh

# 2. start
docker compose -f deploy/compose.yaml up -d

# 3. verify readiness
curl -fsS http://localhost:8080/ready
```

The Compose deployment intentionally sets `PHASE_MIRROR_NO_TLS=true` so it boots
out of the box. See [§6 TLS](#6-tls) to enable TLS properly.

State persists across container restarts in the `pm-state` named volume
(`/var/lib/phase-mirror` inside the container).

## 5. Deploy with systemd

```bash
cd packages/phase-mirror-agent

# 1. create the service user and state dir
sudo useradd --system --home /var/lib/phase-mirror --shell /usr/sbin/nologin phase-mirror
sudo install -d -o phase-mirror -g phase-mirror /var/lib/phase-mirror

# 2. install the binary
sudo install -m 0755 target/release/phase-mirror-agent /usr/local/bin/phase-mirror-agent

# 3. env file (0600, root-owned) — fill in operator keys + TLS
sudo install -d -m 0700 /etc/phase-mirror
sudo install -m 0600 .env.example /etc/phase-mirror/phase-mirror-agent.env
# edit /etc/phase-mirror/phase-mirror-agent.env: set PHASE_MIRROR_ENV=production,
# PHASE_MIRROR_STATE_DIR=/var/lib/phase-mirror/state,
# PHASE_MIRROR_OPERATOR_KEYS_FILE=/etc/phase-mirror/operator.keys

# 4. install + enable the unit
sudo install -m 0644 deploy/systemd/phase-mirror-agent.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now phase-mirror-agent

# 5. verify
systemctl status phase-mirror-agent
curl -fsS http://localhost:8080/ready
```

The unit is hardened: `NoNewPrivileges`, `ProtectSystem=strict`, read-only root
with only `/var/lib/phase-mirror` writable, and `TimeoutStopSec=15` to match the
ADR-007 drain window.

## 6. TLS

### Compose

```yaml
# deploy/compose.yaml (TLS variant)
environment:
  PHASE_MIRROR_ENV: production
  PHASE_MIRROR_TLS_CERT: /certs/tls.crt
  PHASE_MIRROR_TLS_KEY: /certs/tls.key
  # remove PHASE_MIRROR_NO_TLS
volumes:
  - ./certs:/certs:ro
ports:
  - "8443:8080"
healthcheck:
  test: ["CMD", "curl", "-kfsS", "https://localhost:8080/ready"]
```

### systemd

```ini
# /etc/phase-mirror/phase-mirror-agent.env
PHASE_MIRROR_ENV=production
PHASE_MIRROR_TLS_CERT=/etc/phase-mirror/tls.crt
PHASE_MIRROR_TLS_KEY=/etc/phase-mirror/tls.key
```

Keep the key at 0600 root-owned; the unit's hardening keeps everything but the
state dir read-only.

## 7. Health, logs & state

| Concern | Compose | systemd |
| :--- | :--- | :--- |
| Readiness probe | `docker compose -f deploy/compose.yaml ps` (healthcheck on `/ready`) | `curl -fsS http://localhost:8080/ready` |
| Logs | `docker compose -f deploy/compose.yaml logs -f` (json-file, 10m×5) | `journalctl -u phase-mirror-agent -f` |
| State dir | named volume `pm-state` → `/var/lib/phase-mirror` | `/var/lib/phase-mirror` (ReadWritePaths) |
| Stop | `docker compose -f deploy/compose.yaml stop` (SIGTERM, 15s grace) | `systemctl stop phase-mirror-agent` |

The audit WAL lives under `<state_dir>/audit/` and is flushed+fsynced on
shutdown (ADR-007). Protect it with the [backup & restore guide](backup-restore.md).

## 8. Next steps

- Operational procedures: [Runbook](runbook.md)
- Audit WAL protection: [Backup & restore](backup-restore.md)
- Architecture decisions: [docs/adr/README.md](adr/README.md)
