#!/usr/bin/env bash
# audit-backup.sh — copy + hash-verify the audit WAL to a timestamped archive.
#
# Usage: scripts/audit-backup.sh [STATE_DIR] [ARCHIVE_DIR]
#   STATE_DIR   directory holding audit/wal.jsonl (default ./state)
#   ARCHIVE_DIR destination for the timestamped copy (default ./state/audit-backups)
#
# The copy is byte-verified against the source via sha256sum. If the agent is
# running and an integrity endpoint is reachable via PHASE_MIRROR_INTEGRITY_URL,
# the chain is also verified through the API.
set -euo pipefail

STATE_DIR="${1:-./state}"
ARCHIVE_DIR="${2:-./state/audit-backups}"
WAL="$STATE_DIR/audit/wal.jsonl"

if [[ ! -f "$WAL" ]]; then
  echo "audit-backup: no WAL at $WAL (nothing to back up)" >&2
  exit 0
fi

mkdir -p "$ARCHIVE_DIR"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
DEST="$ARCHIVE_DIR/wal-$STAMP.jsonl"

cp "$WAL" "$DEST"
SRC_HASH="$(sha256sum "$WAL" | awk '{print $1}')"
DST_HASH="$(sha256sum "$DEST" | awk '{print $1}')"

if [[ "$SRC_HASH" != "$DST_HASH" ]]; then
  rm -f "$DEST"
  echo "audit-backup: FAILED — copy hash mismatch" >&2
  exit 1
fi

echo "audit-backup: OK $DEST (sha256 $SRC_HASH)"

if [[ -n "${PHASE_MIRROR_INTEGRITY_URL:-}" ]]; then
  if command -v curl >/dev/null 2>&1; then
    RESP="$(curl -fsS "$PHASE_MIRROR_INTEGRITY_URL" 2>/dev/null || true)"
    if [[ -n "$RESP" ]]; then
      echo "audit-backup: integrity endpoint reports: $RESP"
    fi
  fi
fi
