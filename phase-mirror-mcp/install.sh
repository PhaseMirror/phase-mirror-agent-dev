#!/bin/bash
set -euo pipefail

echo "[LM-Studio] Checking for local LM Studio server..."
LMSTUDIO_URL="${LMSTUDIO_BASE_URL:-http://localhost:1234/v1}"
if curl -s -o /dev/null -w "%{http_code}" "$LMSTUDIO_URL/models" 2>/dev/null | grep -q "^200$"; then
    echo "[LM-Studio] Server detected at $LMSTUDIO_URL"
else
    echo "[LM-Studio] WARN: LM Studio server not reachable at $LMSTUDIO_URL"
    echo "[LM-Studio]       Local LLM inference tools will be unavailable until started."
fi

echo "Building Phase Mirror MCP Server..."
cargo build --release

if [[ "${1:-}" == "--register-mcp" ]]; then
    echo "Registering with LM Studio MCP..."
    BIN_PATH="$(pwd)/target/release/phase-mirror-mcp"
    "$BIN_PATH" --register-mcp || true
fi

echo "Installation complete."
echo "  Binary: target/release/phase-mirror-mcp"
echo "  MCP config: ~/.lmstudio/mcp.json"
echo ""
echo "Usage:"
echo "  ./install.sh                # build only"
echo "  ./install.sh --register-mcp # build + register with LM Studio"
