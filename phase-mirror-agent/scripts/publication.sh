#!/usr/bin/env bash
set -euo pipefail

# ── Phase Mirror Agent — Defensive Publication Command Pack ──────────────
#
# Steps:
#   1. Append integration test report as Appendix E to the whitepaper source.
#   2. Generate PDF with Pandoc.
#   3. Pin PDF and repository root to IPFS.
#   4. Anchor the IPFS CID hash on Ethereum via AnchorContract.submitRoot.
#
# Prerequisites:
#   - pandoc, texlive-xetex (or equivalent LaTeX), ipfs, node, cast (foundry)
#   - WHITEPAPER_SRC path must point to the canonical markdown source.
#   - ETH_RPC_URL, DEPLOYER_KEY, ANCHOR_CONTRACT_ADDRESS env vars or pass via flags.
#
# Usage:
#   ./scripts/publication.sh \
#     --whitepaper path/to/whitepaper.md \
#     --eth-rpc https://sepolia.infura.io/v3/... \
#     --deployer-key 0x... \
#     --anchor-contract 0x...
#
# Or export env vars:
#   export WHITEPAPER_SRC=docs/whitepaper.md
#   export ETH_RPC_URL=https://...
#   export DEPLOYER_KEY=0x...
#   export ANCHOR_CONTRACT_ADDRESS=0x...
#   ./scripts/publication.sh

WHITEPAPER_SRC="${WHITEPAPER_SRC:-}"
ETH_RPC_URL="${ETH_RPC_URL:-}"
DEPLOYER_KEY="${DEPLOYER_KEY:-}"
ANCHOR_CONTRACT_ADDRESS="${ANCHOR_CONTRACT_ADDRESS:-}"
REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TEST_REPORT="$REPO_ROOT/dist/integration-test-report.txt"
OUT_DIR="$REPO_ROOT/dist"
TIMESTAMP="$(date -u +%Y%m%d-%H%M%S)"
APPENDIX_HEADER="\\n\\n---\\n\\n# Appendix E: Kubernetes Integration Test\\n\\n**Date:** $(date -u +%Y-%m-%dT%H:%M:%SZ)\\n**Cluster:** kind (phase-mirror-test)\\n\\n```"

usage() {
  sed -n '2,20p' "$0" | sed 's/^# \?//'
  exit 1
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --whitepaper) WHITEPAPER_SRC="$2"; shift 2 ;;
    --eth-rpc)    ETH_RPC_URL="$2"; shift 2 ;;
    --deployer-key) DEPLOYER_KEY="$2"; shift 2 ;;
    --anchor-contract) ANCHOR_CONTRACT_ADDRESS="$2"; shift 2 ;;
    -h|--help) usage ;;
    *) echo "Unknown flag: $1"; usage ;;
  esac
done

if [[ -z "$WHITEPAPER_SRC" ]]; then
  echo "❌ WHITEPAPER_SRC is required (flag --whitepaper or env var)."
  usage
fi
if [[ ! -f "$WHITEPAPER_SRC" ]]; then
  echo "❌ Whitepaper source not found: $WHITEPAPER_SRC"
  exit 1
fi
if [[ ! -f "$TEST_REPORT" ]]; then
  echo "❌ Integration test report not found: $TEST_REPORT"
  echo "   Run scripts/integration-test-kind.sh first."
  exit 1
fi

mkdir -p "$OUT_DIR"

# ── 1. Append Appendix E ──────────────────────────────────────────────────
echo "📝 Appending integration test report to whitepaper…"
cp "$WHITEPAPER_SRC" "$OUT_DIR/whitepaper-with-appendix.md"
{
  echo ""
  echo "---"
  echo ""
  echo "# Appendix E: Kubernetes Integration Test"
  echo ""
  echo "**Date:** $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "**Cluster:** kind (phase-mirror-test)"
  echo ""
  echo '```'
  cat "$TEST_REPORT"
  echo '```'
} >> "$OUT_DIR/whitepaper-with-appendix.md"

# ── 2. Generate PDF ──────────────────────────────────────────────────────
echo "📄 Generating PDF with Pandoc…"
PDF_OUT="$OUT_DIR/phase-mirror-whitepaper-$TIMESTAMP.pdf"
pandoc "$OUT_DIR/whitepaper-with-appendix.md" \
  -o "$PDF_OUT" \
  --pdf-engine=xelatex \
  -V geometry:margin=1in \
  -V fontsize=11pt \
  -V colorlinks=true \
  --toc \
  --toc-depth=2

echo "    PDF: $PDF_OUT"

# ── 3. Pin to IPFS ───────────────────────────────────────────────────────
echo "📌 Pinning to IPFS…"
if ! command -v ipfs >/dev/null 2>&1; then
  echo "❌ ipfs CLI not found. Install ipfs-desktop or kubo."
  exit 1
fi

REPO_CID=$(ipfs add --recursive --pin "$REPO_ROOT" | tail -n1 | awk '{print $2}')
PDF_CID=$(ipfs add --pin "$PDF_OUT" | awk '{print $2}')

echo "    Repository CID: $REPO_CID"
echo "    PDF CID:         $PDF_CID"

# Save CIDs for anchoring
cat > "$OUT_DIR/cids-$TIMESTAMP.json" <<EOF
{
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%Z)",
  "pdf_cid": "$PDF_CID",
  "repo_cid": "$REPO_CID",
  "pdf_file": "$PDF_OUT"
}
EOF

# ── 4. Anchor on Ethereum ────────────────────────────────────────────────
echo "⛓ Anchoring hash on Ethereum…"

if [[ -z "$ETH_RPC_URL" || -z "$DEPLOYER_KEY" || -z "$ANCHOR_CONTRACT_ADDRESS" ]]; then
  echo "⚠️  Skipping Ethereum anchor: missing ETH_RPC_URL, DEPLOYER_KEY, or ANCHOR_CONTRACT_ADDRESS."
  echo "   To anchor manually, run:"
  echo "   cast send $ANCHOR_CONTRACT_ADDRESS 'submitRoot(bytes32)' 0x$(sha256sum -b <<< "$PDF_CID" | awk '{print $1}') --rpc-url \$ETH_RPC_URL --private-key \$DEPLOYER_KEY"
  exit 0
fi

# Compute SHA-256 of the PDF CID (the "root" to anchor)
ROOT_HASH=$(sha256sum -b <<< "$PDF_CID" | awk '{print $1}')
echo "    Root hash (SHA-256 of PDF CID): $ROOT_HASH"

# Submit via cast (Foundry)
# Assumes AnchorContract ABI includes: function submitRoot(bytes32 root) external
cast send "$ANCHOR_CONTRACT_ADDRESS" \
  "submitRoot(bytes32)" \
  "0x$ROOT_HASH" \
  --rpc-url "$ETH_RPC_URL" \
  --private-key "$DEPLOYER_KEY" \
  --ether-gas-limit 100000 \
  --wait

echo "✅ Publication complete."
echo "   PDF:      $PDF_OUT"
echo "   PDF CID:  $PDF_CID"
echo "   Repo CID: $REPO_CID"
echo "   CID file: $OUT_DIR/cids-$TIMESTAMP.json"
