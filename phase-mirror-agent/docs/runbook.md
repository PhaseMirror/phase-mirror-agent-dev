# phase-mirror-agent — Runbook

Day-to-day operations for the phase-mirror-agent governance gateway. Companion
to the [deployment guide](deployment.md); back up the audit chain with the
[backup & restore guide](backup-restore.md).

## 1. Start / stop / restart

### Docker Compose

```bash
cd packages/phase-mirror-agent
docker compose -f deploy/compose.yaml up -d          # start
docker compose -f deploy/compose.yaml ps             # status (healthcheck on /ready)
docker compose -f deploy/compose.yaml restart        # restart
docker compose -f deploy/compose.yaml stop           # graceful stop (SIGTERM, drain)
```

`docker compose stop` sends SIGTERM; the agent drains HTTP and WS, flushes the
audit WAL, and exits 0 before the 15s grace expires.

### systemd

```bash
sudo systemctl start phase-mirror-agent
sudo systemctl restart phase-mirror-agent
sudo systemctl stop phase-mirror-agent
sudo systemctl status phase-mirror-agent
```

## 2. Log access

### Compose

```bash
docker compose -f deploy/compose.yaml logs -f phase-mirror-agent
docker compose -f deploy/compose.yaml logs --tail=200 phase-mirror-agent
```

### systemd

```bash
journalctl -u phase-mirror-agent -f
journalctl -u phase-mirror-agent --since "10 min ago"
```

In production the agent logs JSON (`PHASE_MIRROR_LOG_FORMAT=json`); pipe through
`jq` for filtering:

```bash
docker compose -f deploy/compose.yaml logs | jq -r 'select(.level=="error") | .msg'
```

## 3. Readiness & health

```bash
curl -fsS http://localhost:8080/health    # liveness: 200 whenever serving
curl -fsS http://localhost:8080/ready     # readiness: 200 or 503 with JSON error body
```

`/ready` reflects whether the audit WAL opens writable and the chain head parses.
A 503 means the orchestrator should stop routing to this instance.

## 4. Metrics

```bash
curl -fsS -H "Authorization: Bearer <read-key>" http://localhost:8080/metrics
```

Exposes `pm_commands_total{outcome=...}`, `pm_audit_writes_total`,
`pm_audit_writes_latency_seconds`, `pm_ws_clients_current`,
`pm_ws_events_broadcast_total`, `pm_http_requests_total{method,status}`
(ADR-007 §2.2). Point Prometheus at it, or scrape ad hoc.

## 5. Operator key rotation

Keys are stored hashed (SHA-256 digests only); rotating is cheap and online.

```bash
scripts/gen-operator-key.sh alice operator:write    # generates a fresh key+digest
# add the new digest line to the keys file, keep the old line until all clients rotate
```

The agent reads the keys file at boot; restart to pick up changes
(`docker compose ... restart` / `sudo systemctl restart`). After all clients use
the new key, remove the old line and restart again. Never put raw keys in the
keys file or logs.

## 6. Upgrading

1. **Back up the audit WAL first** (see backup-restore.md).
2. Build the new image/binary and check the changelog for schema migrations.
3. Roll: compose `docker compose -f deploy/compose.yaml pull && up -d`; systemd
   `sudo install` the new binary and `systemctl restart`.
4. Verify `/ready` is 200, `/metrics` is reachable, and the WAL replays:
   boot logs show `audit store opened` with the chain head hash.
5. Spot-check an audit-integrity read (`GET /audit/integrity` with a read key).

## 7. Recovery

The durable audit WAL is the source of truth. See [backup-restore.md](backup-restore.md)
for restore. General recovery flow:

- **Degraded readiness (503):** check the state dir is writable and the WAL head
  parses (`audit` files under `<state_dir>/audit/`). Rotate or restore the WAL if
  the head is corrupt (ADR-005).
- **Lost keys:** regenerate operator keys and update clients; the audit chain is
  unaffected.
- **Full data loss:** restore the WAL from the latest backup (backup-restore.md)
  and restart; the chain hash validates the restore.

## 8. Security notes

- All writes require `operator:write`; reads require any authenticated scope.
- Rate limiting (default 10 rps / 20 burst) applies to protected surfaces.
- TLS is mandatory in production unless `PHASE_MIRROR_NO_TLS` is explicitly set.
- The container and systemd unit run unprivileged with a read-only filesystem
  except the state dir.
