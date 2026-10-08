# phase-mirror-agent — Backup & Restore

The audit WAL (`<state_dir>/audit/wal.jsonl`) is the agent's durable source of
truth (ADR-005). It is **append-only and hash-chained**: every entry links to the
previous entry's hash, so any truncation, reordering, or tamper is detectable.
This guide covers protecting that chain.

- [1. What to back up](#1-what-to-back-up)
- [2. Taking a backup](#2-taking-a-backup)
- [3. Verifying a backup](#3-verifying-a-backup)
- [4. Restore](#4-restore)
- [5. Rotation](#5-rotation)
- [6. Recovery scenarios](#6-recovery-scenarios)

## 1. What to back up

```
<state_dir>/audit/                 # WAL + wal.meta.json + archived segments
<state_dir>/                       # anything else you rely on (archives, keys)
```

The agent never deletes WAL data on its own; `audit/rotate` archives the current
segment to `audit/archived/` with chain continuity (the fresh WAL begins with the
archived tail's hash).

## 2. Taking a backup

`scripts/audit-backup.sh` copies the WAL to a timestamped archive and
**byte-verifies** the copy with `sha256sum`:

```bash
cd packages/phase-mirror-agent
scripts/audit-backup.sh ./state ./state/audit-backups
# audit-backup: OK ./state/audit-backups/wal-20260809T100000Z.jsonl (sha256 ...)
```

If the agent exposes an integrity endpoint, set `PHASE_MIRROR_INTEGRITY_URL` and
the script also reports a chain-integrity check through the API.

Schedule it (systemd timer or cron) to protect the chain continuously:

```bash
# crontab example — hourly
0 * * * * /path/to/phase-mirror-agent/scripts/audit-backup.sh /var/lib/phase-mirror/state /var/lib/phase-mirror/backups
```

Copy archives off the host (an object store, a second disk) for real durability.

## 3. Verifying a backup

Three independent checks:

1. **Copy integrity** — `sha256sum` of source vs archive (done by the script).
2. **Chain integrity** — ask the running agent to re-verify the chain:

   ```bash
   curl -fsS -H "Authorization: Bearer <read-key>" http://localhost:8080/audit/integrity
   # {"valid":true,"entries":N,"head_hash":"<64 hex>"}
   ```

3. **Range verification** — `POST /audit/verify` re-checks a sequence range:

   ```bash
   curl -fsS -X POST -H "Authorization: Bearer <read-key>" \
     -H "Content-Type: application/json" \
     -d '{"from":0,"to":100}' http://localhost:8080/audit/verify
   ```

## 4. Restore

The WAL is the single source of truth; restore by replacing it at the expected
path and letting the agent re-open it:

```bash
# 1. stop the agent so nothing appends mid-restore
docker compose -f deploy/compose.yaml stop            # or: sudo systemctl stop phase-mirror-agent

# 2. replace the WAL with the verified archive
STATE=./state                                        # adjust to your setup
cp backup/wal-20260809T100000Z.jsonl "$STATE/audit/wal.jsonl"

# 3. start the agent; boot must log `audit store opened` with the chain head hash
docker compose -f deploy/compose.yaml start          # or: sudo systemctl start phase-mirror-agent

# 4. confirm chain validity
curl -fsS -H "Authorization: Bearer <read-key>" http://localhost:8080/audit/integrity
```

If the restored WAL does not parse (e.g. partial copy), the agent refuses to
start rather than silently truncating — that is the fail-safe behavior of
ADR-005. Retry with a different archive.

## 5. Rotation

When the WAL grows, archive segments via the agent (never by hand):

```bash
scripts/audit-rotate.sh http://localhost:8080 "Authorization: Bearer <write-key>"
```

The agent atomically archives the current segment and starts a fresh WAL whose
first entry links to the archived tail (see the `rotate_endpoint_archives_and_continues`
test). Backup the archived segment too — `audit-backup.sh` also covers
`audit/archived/` if you back up the whole `audit/` tree.

## 6. Recovery scenarios

| Scenario | Action |
| :--- | :--- |
| Readiness 503 (WAL head unparseable) | Restore from the latest verified backup (§4), or rotate the corrupt tail if it is the last segment. |
| Lost state dir | Restore `audit/` from backup; chain hash validates. |
| Suspected tamper | `GET /audit/integrity` → `valid:false`; investigate and restore from a verified backup. |
| Operator keys lost | Regenerate keys (`scripts/gen-operator-key.sh`); audit data is unaffected. |

## See also

- ADR-005 (audit store design), ADR-007 §2.3 (readiness reflects WAL health)
- [Runbook](runbook.md) — day-to-day operations
