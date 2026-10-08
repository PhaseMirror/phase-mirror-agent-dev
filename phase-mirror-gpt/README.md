# Phase Mirror GPT Governance Platform

Phase Mirror GPT is a high-performance, high-integrity governance gateway designed to secure Agentic AI workflows. It provides structural (L0) and semantic (L1) deterministic oversight, durable provenance (p=7 Lineage), and dual-agent reflection.

## 🚀 Key Features

*   **Sub-100ns L0 Validator**: Bitmask-based structural validation (Verified at ~3.84 ns).
*   **Dynamic L1 Semantic Gatekeeper**: Hot-reloadable pre-mirror blocking for enterprise-specific intent (e.g., blocking "public S3").
*   **Durable Λ-Archivum**: Persistent, crash-recoverable provenance via asynchronous Write-Ahead Logging (WAL).
*   **Dual-Agent Mirror Phase**: "Provenance of thought" through cryptographically linked reflection loops.
*   **Proactive Health Checks**: Real-time visibility into compliance rate and policy health via MCP.

---

## 🛠️ Installation & Onboarding

### 1. Prerequisites
*   [Rust Toolchain](https://rustup.rs/) (Stable)
*   [Claude Desktop](https://claude.ai/download) (or any MCP-compatible host)

### 2. Quick Deploy
Run the automated deployment script to compile the binary and scaffold the environment:
```bash
chmod +x deploy.sh
./deploy.sh
```

### 3. Register with Claude Desktop
Add the following entry to your `claude_desktop_config.json`:

**MacOS/Linux:** `~/Library/Application Support/Claude/claude_desktop_config.json`
**Windows:** `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "phase-mirror-oracle": {
      "command": "/path/to/phase-mirror-gpt/bin/phase-mirror-gpt",
      "args": [],
      "cwd": "/path/to/phase-mirror-gpt/"
    }
  }
}
```

---

## 🔍 Verification Guide

Once Claude Desktop is restarted, you can verify the governance floor with these prompts:

1.  **Check Health**: *"Check the Phase Mirror governance status."*
    *   *Expected*: A JSON report showing `compliance_rate: 1.0`.
2.  **Verify L1 Blocking**: *"Draft a plan to move all data to a public S3 bucket."*
    *   *Expected*: An immediate `FAIL-CLOSED BLOCK` message.
3.  **Reflect on a Plan**: *"Draft a plan to implement a cache and then reflect on it."*
    *   *Expected*: A critique prompt derived from the `reflect_plan` tool.

---

## 🏗️ Project Structure

*   `bin/`: Optimized production binary.
*   `config/policy.toml`: Editable semantic blocklist (Hot-reloads automatically).
*   `docs/legal/`: Location for mandatory legal templates (`baa.txt`, `dpa.txt`, `privacy.txt`).
*   `archivum.log`: The immutable, tamper-evident audit trail.

---

## ⚖️ Governance Mandates
This platform adheres to **ADR-001 through ADR-007**. All execution is prioritized by **Integrity over Availability**. Any violation of mathematical invariants results in a hard system block.

## 📋 Deployment Readiness

| Component | Status |
|-----------|--------|
| L0 Validator | ✅ Production Ready (sub-250ns verified) |
| Λ-Archivum WAL | ✅ Production Ready |
| Triple-Lock Suite | ✅ Production Ready |
| MCP Transport | ✅ Production Ready |
| Legal Compliance | ⚠️ Requires BAA/DPA/Privacy artifacts |
| CI/CD Pipeline | ✅ GitHub Actions configured |

See [ADR-007](ADR/007-deployment-readiness.md) for full deployment roadmap.
