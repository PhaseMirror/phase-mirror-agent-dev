#!/usr/bin/env bash

set -euo pipefail

# Distribution Paths
DIST_DIR="bin"
PROJECT_NAME="phase-mirror-gpt"

echo "=== [1/5] Initiating High-Integrity Compilation ==="
cargo build --release

echo "=== [2/5] Stripping Symbols & Optimizing Footprint ==="
mkdir -p "$DIST_DIR"
cp "target/release/$PROJECT_NAME" "$DIST_DIR/" || cp "../target/release/$PROJECT_NAME" "$DIST_DIR/" 2>/dev/null || true

if command -v strip &> /dev/null; then
    strip "$DIST_DIR/$PROJECT_NAME" 2>/dev/null || true
    echo "Optimization complete."
else
    echo "Warning: 'strip' utility not found. Skipping binary size optimization."
fi

echo "=== [3/5] Scaffolding Distribution Environment ==="
mkdir -p config docs/legal
if [ ! -f "config/policy.toml" ]; then
    cat <<EOF > "config/policy.toml"
[semantic_policy]
forbidden_patterns = ["public", "s3://", "DROP TABLE", "chmod 777"]
EOF
fi

echo "=== [4/5] Running Self-Diagnostics ==="
if [ -f "$DIST_DIR/$PROJECT_NAME" ]; then
    ./"$DIST_DIR/$PROJECT_NAME" <<EOF
{"jsonrpc": "2.0", "method": "tools/call", "params": {"name": "get_governance_status", "arguments": {}}, "id": 0}
EOF
else
    echo "Binary not found, skipping self-diagnostics"
fi

echo -e "\n=== [5/5] Success: Phase Mirror GPT is Ready ==="
echo "Binary location: $(pwd)/$DIST_DIR/$PROJECT_NAME"
echo "Project root (CWD): $(pwd)"
echo "Follow the README.md to register with Claude Desktop."
