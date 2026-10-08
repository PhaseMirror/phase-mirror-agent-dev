#!/usr/bin/env bash
# audit-rotate.sh — archive the current audit WAL and start a new one that keeps
# chain continuity. The agent itself performs the atomic rotation via the
# POST /audit/rotate endpoint; this script is a thin, operator-friendly wrapper.
#
# Usage: scripts/audit-rotate.sh [BASE_URL] [AUTH_HEADER]
#   BASE_URL    agent base URL (default http://localhost:8080)
#   AUTH_HEADER optional, e.g. "Authorization: Bearer <token>"
#
# A fresh WAL begins with first_prev_hash == the archived tail, so the hash
# chain is continuous across rotation (see ADR-005 section 2.4).
set -euo pipefail

BASE_URL="${1:-http://localhost:8080}"
AUTH_HEADER="${2:-}"

ROTATE_URL="$BASE_URL/audit/rotate"
ARGS=()
if [[ -n "$AUTH_HEADER" ]]; then
  ARGS+=(-H "$AUTH_HEADER")
fi

if ! command -v curl >/dev/null 2>&1; then
  echo "audit-rotate: curl is required" >&2
  exit 1
fi

echo "audit-rotate: requesting rotation at $ROTATE_URL"
RESP="$(curl -fsS -X POST "${ARGS[@]}" "$ROTATE_URL")"
echo "audit-rotate: OK $RESP"
