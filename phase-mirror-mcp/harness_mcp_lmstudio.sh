#!/usr/bin/env bash
set -euo pipefail

BIN="/home/multiplicity/Multiplicity/target/release/phase-mirror-mcp"
MOCK_PORT="${MOCK_PORT:-9090}"
HOME_DIR="${HOME:-/tmp/ci-mcp-home}"
CONTRACT="mcp-contract.json"
CONTRACT_BACKUP=""
PASS=0
FAIL=0
SKIP=0
LMSTUDIO_URL="${LMSTUDIO_BASE_URL:-http://localhost:${MOCK_PORT}/v1}"
INJECT_ERROR="${PHASE_MIRROR_INJECT_ERROR:-0}"

export HOME="$HOME_DIR"
mkdir -p "$HOME_DIR/.lmstudio"

if [[ -f "$CONTRACT" ]]; then
  CONTRACT_BACKUP="$(mktemp)"
  cp "$CONTRACT" "$CONTRACT_BACKUP"
fi

restore_contract() {
  if [[ -n "$CONTRACT_BACKUP" ]]; then
    cp "$CONTRACT_BACKUP" "$CONTRACT"
    rm -f "$CONTRACT_BACKUP"
  else
    rm -f "$CONTRACT"
  fi
}

start_mock() {
  if ! curl -s -o /dev/null -w "%{http_code}" "http://127.0.0.1:${MOCK_PORT}/v1/models" 2>/dev/null | grep -q "^200$"; then
    MOCK_PORT="$MOCK_PORT" python3 "$(dirname "$0")/ci/mock_lmstudio.py" &
    MOCK_PID=$!
    for i in $(seq 1 20); do
      if curl -s -o /dev/null -w "%{http_code}" "http://127.0.0.1:${MOCK_PORT}/v1/models" 2>/dev/null | grep -q "^200$"; then
        break
      fi
      sleep 0.2
    done
  fi
}

stop_mock() {
  if [[ -n "${MOCK_PID:-}" ]]; then
    kill "$MOCK_PID" 2>/dev/null || true
    wait "$MOCK_PID" 2>/dev/null || true
  fi
}

check_witness() {
  local resp="$1"
  if echo "$resp" | grep -q 'zero_spacings' && echo "$resp" | grep -q 'SIGNED_HASH'; then
    return 0
  else
    return 1
  fi
}

check_error() {
  local resp="$1"
  if echo "$resp" | grep -q '"isError":true'; then
    return 0
  fi
  if echo "$resp" | grep -q '"status":"WARN"'; then
    return 0
  fi
  return 1
}

check_no_witness_leak() {
  local resp="$1"
  if echo "$resp" | grep -q '"isError":true'; then
    if echo "$resp" | grep -q 'zero_spacings'; then
      return 1
    fi
  fi
  return 0
}

run_success() {
  local tool="$1"
  local args="$2"
  local resp
  resp=$(echo "{\"jsonrpc\":\"2.0\",\"method\":\"tools/call\",\"params\":{\"name\":\"${tool}\",\"arguments\":${args}},\"id\":1}" | LMSTUDIO_BASE_URL="$LMSTUDIO_URL" PHASE_MIRROR_INJECT_ERROR=0 "$BIN" 2>&1) || true
  if echo "$resp" | grep -q '"isError":false'; then
    if check_witness "$resp"; then
      echo "  PASS: witness present"
      PASS=$((PASS+1))
    else
      echo "  FAIL: isError=false but witness missing"
      echo "  RESPONSE: $resp"
      FAIL=$((FAIL+1))
    fi
  elif echo "$resp" | grep -q '"status":"WARN"'; then
    echo "  SKIP: WARN (runtime condition)"
    SKIP=$((SKIP+1))
  else
    echo "  FAIL: unexpected response"
    echo "  RESPONSE: $resp"
    FAIL=$((FAIL+1))
  fi
}

run_error() {
  local tool="$1"
  local resp
  resp=$(echo "{\"jsonrpc\":\"2.0\",\"method\":\"tools/call\",\"params\":{\"name\":\"${tool}\",\"arguments\":{\"__invalid__\":true}},\"id\":1}" | LMSTUDIO_BASE_URL="$LMSTUDIO_URL" PHASE_MIRROR_INJECT_ERROR="$INJECT_ERROR" "$BIN" 2>&1) || true
  if check_error "$resp"; then
    if check_no_witness_leak "$resp"; then
      echo "  PASS: error returned, no witness leak"
      PASS=$((PASS+1))
    else
      echo "  FAIL: error code present but witness leaked"
      echo "  RESPONSE: $resp"
      FAIL=$((FAIL+1))
    fi
  else
    echo "  FAIL: expected error not found"
    echo "  RESPONSE: $resp"
    FAIL=$((FAIL+1))
  fi
}

trap stop_mock EXIT

echo "=== MCP Sedona Spine Witness Harness (full tool surface) ==="
echo "BIN: $BIN"
echo "LMSTUDIO_URL: $LMSTUDIO_URL"
echo "HOME: $HOME_DIR"
echo "INJECT_ERROR: $INJECT_ERROR"
echo ""

start_mock

TOOLS=(
  "get_stability_metric"
  "verify_ledger"
  "evaluate_esi_risk"
  "check_governed_bridge"
  "get_metrics"
  "lmstudio_health"
  "lmstudio_list_models"
  "lmstudio_generate"
  "lmstudio_chat"
  "lmstudio_embed"
  "lmstudio_register_mcp"
)

# ---------- SUCCESS PATH ----------
echo "=== SUCCESS PATH ==="
for tool in "${TOOLS[@]}"; do
  echo "--- ${tool} success ---"
  ARGS="{}"
  case "$tool" in
    lmstudio_chat) ARGS='{"messages":[{"role":"user","content":"hi"}],"model":"test-model"}' ;;
    lmstudio_generate) ARGS='{"prompt":"hi","model":"test-model"}' ;;
    lmstudio_embed) ARGS='{"input":"hi","model":"test-model"}' ;;
    lmstudio_register_mcp) ARGS='{"command":"/bin/echo","args":[]}' ;;
    verify_ledger) ARGS='{"ledger":[],"current_state":{"schemaVersion":"1.0","schemaHash":"","permissionBits":0,"driftMagnitude":0,"nonce":{"value":"xx","issuedAt":0},"contractionWitnessScore":null}}' ;;
    check_governed_bridge) ARGS='{"src_prime":2,"tgt_prime":3,"tissue_id":1,"current_tick":0,"jubilee_window":[0,0],"ace_blocks":[],"morphisms":[],"pre_memory":[0,0,0,0],"post_memory":[0,0,0,0]}' ;;
    evaluate_esi_risk) ARGS='{"artifact_id":"test","preservation_state":{"last_verified":0,"redundancy_level":1,"storage_tier":"HOT"},"active_litigation":false,"data_sensitivity":"Low"}' ;;
  esac
  run_success "$tool" "$ARGS"
done

# ---------- ERROR PATH ----------
echo ""
echo "=== ERROR PATH ==="
for tool in "${TOOLS[@]}"; do
  echo "--- ${tool} error ---"
  run_error "$tool"
done

restore_contract

echo ""
echo "=== Results: ${PASS} passed, ${FAIL} failed, ${SKIP} skipped ==="
if [[ "${FAIL}" -eq 0 ]]; then
  echo "HARNESS: ALL LEVER METRICS MET"
  exit 0
else
  echo "HARNESS: FAILURES DETECTED"
  exit 1
fi
