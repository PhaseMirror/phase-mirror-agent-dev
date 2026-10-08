# Phase 0 — Audit & Best-Component Selection

**Status**: Complete (2026-07-01)
**Gate for**: Phase 1 (Unified Meta-Orchestrator)

---

## 1. Inventory

| Domain | Current Candidates | Standalone Entry Point | Notes |
|:--|:--|:--|:--|
| React shell | `q-calculator/App.tsx`, `admin/src/App.tsx` | `q-calculator: npm run dev` (port 7070+5173); `admin: npm run dev` (port 3000) | `q-calculator/` has a complete 3-panel layout with sidebar, center router, right telemetry panel. `admin/` is a thin shell. |
| State management | `q-calculator` (inline useState), `admin/` (none) | — | Neither uses a global store. Zustand selected as the canonical layer per `docs/UI_Plumbing.md`. |
| API client | `q-calculator/server.ts` (Express 3001), `agency-server/src/index.js` (Express 8082) | `q-calculator: npm run dev`; `agency-server: node src/index.js` | `agency-server` is the production proxy with JWT + SSE + binary dispatch. |
| Binary orchestration | `agency-server/src/index.js` (child_process.exec) | `agency-server/bin/coding-commander` | Functional but unsandboxed. Replace `exec` with allow-listed `spawn` in Phase 1. |
| Graph / Dissonance | `admin/` (mock hardcoded nodes/edges), `packages/phase-mirror-dissonance/` (crate) | — | `phase-mirror-dissonance` is the real data source. Mock-to-real migration is a one-time route change. |
| Q-Calculator engine | `Legacy/q-calculator/` (Python FastAPI, port 7070), `q-calculator/wasm/` (Rust→WASM) | `Legacy/q-calculator: uvicorn` or `compose.yaml up` | WASM `q_calculator_rs.js` runs client-side for <1s feedback. Python service retained for batch PIRTM jobs via REST. |
| Governed MCP | `Prime/crates/mcp/` (`multiplicity-mcp`), `packages/phase-mirror-mcp/` | `Prime: cargo run --bin multiplicity-mcp` | `multiplicity-mcp` implements ALP gate, stdio + HTTP/SSE, JSON-RPC 2.0. Phase 3 complete. |
| Policy engine | `packages/phase-mirror-gpt/src/domain_invariants.rs` (Rust), `q-calculator/core/qari/csl_policy.py` (Python) | `phase-mirror-gpt: cargo run` | Rust engine is production-ready (ADR-007). Python CSL is superseded. |
| Ledger | `agency-server/var/archivum/ledger.jsonl`, `q-calculator` SQLite, `phase-mirror-gpt` Λ-Archivum WAL | — | Λ-Archivum WAL in `phase-mirror-gpt` is the single source of truth. UI reads via agency-server REST proxy. |
| Telemetry | `phase-mirror-gpt/src/telemetry.rs` (Prometheus), `q-calculator/src/metrics.py` (Prometheus) | — | Rust-native metrics; Python adapter only if q-calculator REST runs standalone. |

---

## 2. Best-Component Selections

| Domain | Selected Implementation | Source Path | Rationale |
|:--|:--|:--|:--|
| **React shell** | `q-calculator` 3-panel layout | `packages/q-calculator/App.tsx` | Complete app shell with sidebar (nav + system status), center router, right panel (safety meter, jurisdiction, prime ledger, provenance). `admin/` is only a layout stub. |
| **State management** | Zustand + devtools | — | Explicit per-domain stores, zero boilerplate, matches `docs/UI_Plumbing.md`. |
| **API client** | `agency-server` REST + SSE | `packages/phase-mirror-agency/agency-server/` | Already proxies Rust binaries, JWT auth, SSE events. Extend with modular TS routes. |
| **Binary orchestration** | `agency-server` spawn with allow-list | `packages/phase-mirror-agency/agency-server/src/` | Functional; Phase 1 replaces `exec` with allow-listed `spawn`. |
| **Dissonance graph** | `packages/phase-mirror-dissonance` crate + Cytoscape.js | Crate + `react-cytoscapejs` | Real engine output replaces mock data in `admin/`. |
| **Q-Calculator engine** | WASM (client) + FastAPI REST (server fallback) | `packages/q-calculator/wasm/` + `Legacy/q-calculator/src/api_v2.py` | WASM for interactive <1s feedback; Python REST for heavy batch jobs. |
| **Governed MCP** | `Prime/crates/mcp/` (`multiplicity-mcp`) | `Prime/crates/mcp/` | Production ALP gate, stdio + HTTP/SSE transports, JSON-RPC 2.0 interface. |
| **Policy engine** | `packages/phase-mirror-gpt/src/domain_invariants.rs` | Rust crate | Sub-250ns L0 validation, hot-reload semantic policy, ADR-007 proven. |
| **Ledger** | Λ-Archivum WAL (`phase-mirror-gpt`) | `packages/phase-mirror-gpt/src/archivum.rs` | Append-only, hash-chained, async WAL. Backed by `agency-server` REST. |
| **Observability** | `phase-mirror-gpt` Prometheus + structured log | `packages/phase-mirror-gpt/src/telemetry.rs` + `structured_log.rs` | Rust-native, low overhead. Python metrics adapter only when q-calculator REST is independent. |

---

## 3. Standalone Verification Protocol

For each component selected above, confirm it runs independently before embedding.

```bash
# Governance core
cd packages/phase-mirror-gpt && cargo run --bin phase-mirror-gpt

# MCP server
cd Prime && cargo run --bin multiplicity-mcp

# Agency server (JS scaffold)
cd packages/phase-mirror-agency/agency-server && node src/index.js

# Q-Calculator (WASM frontend + Python REST)
cd packages/q-calculator && npm run dev          # Vite (port 5173)
cd Legacy/q-calculator && uvicorn packages.q_calculator.src.api_v2:app --port 7070  # Python REST

# PIRTM core
cd Prime && cargo run --bin pirtm-core
```

---

## 4. Migration Boundaries

- **Not moving**: `Legacy/q-calculator/`, `citizen-gardens/`, `discord-bot/`. They keep their own `start.sh` / `compose.yaml`. The meta-UI links to them.
- **Not replacing**: `phase-mirror-gpt` Rust binaries. `agency-server` acts as a proxy.
- **Converting**: `agency-server` from `.js` to `.ts` with modular routes. Existing `.js` archived as `src/legacy/`.
- **Extracting**: UI components from `q-calculator/App.tsx` into `agency-server/ui/src/components/`. The original file is refactored to import from the extracted components.
- **New**: `agency-server/ui/` (React/TS + Vite), `agency-server/src/routes/` (TS route modules), `agency-server/config/` (YAML).

---

## 5. Gate Criteria for Phase 1 Entry

- [x] Every selected component verified `cargo run` / `npm run dev` independently.
- [x] Best-component table documented above.
- [x] Migration boundaries defined to prevent scope creep.
- [ ] `MIGRATION.md` committed to `packages/phase-mirror-gpt/ADR/` (in progress).

*Next phase: Phase 1 — Unified Meta-Orchestrator.*
