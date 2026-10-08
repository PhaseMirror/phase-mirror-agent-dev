#!/usr/bin/env bash
# Generate an operator API key for phase-mirror-agent (ADR-006 §2.1).
#
# Usage:
#   scripts/gen-operator-key.sh [id] [scope]
#   scripts/gen-operator-key.sh alice operator:write
#
# Prints the raw key (shown once) and the hashed line to store in
# config/keys/operator.keys (chmod 600). The server only ever sees hashes.
set -euo pipefail

id="${1:-operator}"
scope="${2:-operator:write}"

case "$scope" in
  operator:read|operator:write) ;;
  *) echo "invalid scope '$scope' (expected operator:read or operator:write)" >&2; exit 2 ;;
esac

if ! command -v openssl >/dev/null 2>&1; then
  echo "openssl is required to generate CSPRNG keys" >&2
  exit 1
fi

# Key format: `pmr_op_` + 32 lowercase hex chars (16 random bytes). This must
# match matches_key_format() in src/security/auth.rs.
hex="$(openssl rand -hex 16)"
key="pmr_op_${hex}"
hash="$(printf '%s' "$key" | sha256sum | awk '{print $1}')"

echo "raw key (show once, keep in your secret manager):"
echo "  $key"
echo
echo "append to config/keys/operator.keys (chmod 600):"
echo "  ${id}:${scope}:${hash}"
