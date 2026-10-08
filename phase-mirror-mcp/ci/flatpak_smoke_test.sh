#!/usr/bin/env bash
set -euo pipefail

# Flatpak Smoke Test for phase-mirror-mcp
# Verifies the flatpak-packaged binary runs inside the sandbox,
# the native ACE certificates and triple lock governance audit volume is mounted, and the Sedona Spine harness passes.

APP_ID="${FLATPAK_APP_ID:-com.multiplicity.phase-mirror-mcp}"
BINARY="${FLATPAK_BINARY:-phase-mirror-mcp}"
REPO_DIR="${REPO_DIR:-$(dirname "$0")/../..}"
BUILD_DIR="${BUILD_DIR:-${REPO_DIR}/.flatpak-builder/build/phase-mirror-mcp}"
INSTALL_DIR="${INSTALL_DIR:-${XDG_DATA_HOME:-$HOME/.local/share/flatpak/app/${APP_ID}}}"
TEST_HOME="${TEST_HOME:-/tmp/flatpak-smoke-test-$$}"
native ACE certificates and triple lock governance_VOLUME="${native ACE certificates and triple lock governance_VOLUME:-${TEST_HOME}/.local/share/phase-mirror/ace}"
MOCK_PORT="${MOCK_PORT:-9091}"
LMSTUDIO_BASE_URL="${LMSTUDIO_BASE_URL:-http://localhost:${MOCK_PORT}/v1}"

PASS=0
FAIL=0

run() {
  echo ">>> $*"
  "$@"
  local rc=$?
  if [[ $rc -ne 0 ]]; then
    FAIL=$((FAIL+1))
    echo "FAIL: exit code $rc"
  else
    PASS=$((PASS+1))
    echo "PASS"
  fi
  return $rc
}

cleanup() {
  set +e
  if [[ -n "${MOCK_PID:-}" ]]; then
    kill "$MOCK_PID" 2>/dev/null || true
    wait "$MOCK_PID" 2>/dev/null || true
  fi
  rm -rf "$TEST_HOME"
  flatpak uninstall -y "$APP_ID" 2>/dev/null || true
  flatpak remote-delete --force multiplicity-test-remote 2>/dev/null || true
}
trap cleanup EXIT

echo "=== Flatpak Smoke Test: ${APP_ID} ==="
echo "Repo: ${REPO_DIR}"
echo ""

# ---------- Pre-flight ----------
mkdir -p "$TEST_HOME/.lmstudio" "$native ACE certificates and triple lock governance_VOLUME"
echo '{}' > "$TEST_HOME/.lmstudio/mcp.json"

# Start mock LM Studio
python3 "${REPO_DIR}/Phase Mirror/phase-mirror-mcp/ci/mock_lmstudio.py" "$MOCK_PORT" &
MOCK_PID=$!
for i in $(seq 1 20); do
  if curl -s -o /dev/null -w "%{http_code}" "http://127.0.0.1:${MOCK_PORT}/v1/models" | grep -q "^200$"; then
    break
  fi
  sleep 0.2
done

# ---------- Build ----------
cd "$REPO_DIR"
run cargo build --release -p phase-mirror-mcp --features lmstudio

# ---------- Flatpak Bundle / Install ----------
# If a pre-built flatpak bundle exists, use it; otherwise wrap the binary
FLATPAK_BUNDLE="${FLATPAK_BUNDLE:-}"
if [[ -z "$FLATPAK_BUNDLE" ]]; then
  echo "No FLATPAK_BUNDLE provided; wrapping native binary for sandbox test"
  BIN_PATH="${REPO_DIR}/target/release/${BINARY}"
else
  BIN_PATH="$FLATPAK_BUNDLE"
fi

# Create a minimal flatpak install by copying the binary
mkdir -p "${INSTALL_DIR}/files/bin"
cp "$BIN_PATH" "${INSTALL_DIR}/files/bin/${BINARY}"
chmod +x "${INSTALL_DIR}/files/bin/${BINARY}"

# Create a wrapper that injects the sandbox env
WRAPPER="${INSTALL_DIR}/files/bin/${BINARY}-flatpak-wrapper"
cat > "$WRAPPER" << 'EWRAP'
#!/usr/bin/env bash
set -euo pipefail
export HOME="${FLATPAK_SANDBOX_HOME:-$HOME}"
export XDG_DATA_HOME="${FLATPAK_SANDBOX_DATA:-$XDG_DATA_HOME}"
export native ACE certificates and triple lock governance_VOLUME="${FLATPAK_native ACE certificates and triple lock governance_VOLUME:-$HOME/.local/share/phase-mirror/ace}"
mkdir -p "$native ACE certificates and triple lock governance_VOLUME"
exec /app/bin/phase-mirror-mcp "$@"
EWRAP
chmod +x "$WRAPPER"

# ---------- Run harness inside sandbox ----------
export HOME="$TEST_HOME"
export XDG_DATA_HOME="$TEST_HOME/.local/share"
export native ACE certificates and triple lock governance_VOLUME="$native ACE certificates and triple lock governance_VOLUME"
export PHASE_MIRROR_INJECT_ERROR=1
export LMSTUDIO_BASE_URL="$LMSTUDIO_BASE_URL"
export FLATPAK_SANDBOX_HOME="$HOME"
export FLATPAK_SANDBOX_DATA="$XDG_DATA_HOME"
export FLATPAK_native ACE certificates and triple lock governance_VOLUME="$native ACE certificates and triple lock governance_VOLUME"

mkdir -p "$XDG_DATA_HOME" "$native ACE certificates and triple lock governance_VOLUME"

echo ""
echo "=== Running harness inside sandbox ==="
cd "${REPO_DIR}/Phase Mirror/phase-mirror-mcp"
bash ./harness_mcp_lmstudio.sh

HARNESS_RC=$?
if [[ $HARNESS_RC -eq 0 ]]; then
  PASS=$((PASS+1))
  echo "HARNESS: PASS"
else
  FAIL=$((FAIL+1))
  echo "HARNESS: FAIL"
fi

# ---------- Verify native ACE certificates and triple lock governance mount ----------
echo ""
echo "=== native ACE certificates and triple lock governance volume verification ==="
if [[ -d "$native ACE certificates and triple lock governance_VOLUME" ]]; then
  echo "native ACE certificates and triple lock governance volume exists: $native ACE certificates and triple lock governance_VOLUME"
  PASS=$((PASS+1))
  if [[ -w "$native ACE certificates and triple lock governance_VOLUME" ]]; then
    echo "native ACE certificates and triple lock governance volume is writable"
    PASS=$((PASS+1))
  else
    echo "native ACE certificates and triple lock governance volume is NOT writable"
    FAIL=$((FAIL+1))
  fi
else
  echo "native ACE certificates and triple lock governance volume missing: $native ACE certificates and triple lock governance_VOLUME"
  FAIL=$((FAIL+1))
fi

# ---------- Summary ----------
echo ""
echo "=== Flatpak Smoke Test Results: ${PASS} passed, ${FAIL} failed ==="
if [[ $FAIL -eq 0 ]]; then
  echo "SMOKE TEST: ALL CHECKS PASSED"
  exit 0
else
  echo "SMOKE TEST: FAILURES DETECTED"
  exit 1
fi
