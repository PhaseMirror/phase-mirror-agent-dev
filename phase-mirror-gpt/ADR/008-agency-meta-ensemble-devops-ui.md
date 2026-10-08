# ADR-008: Agency DevOps Meta-Ensemble Overhaul

## Status
Proposed (2026-07-01)

---

## Context

The Phase Mirror repository now contains a mature but fragmented ecosystem:

| Layer | Location | Runtime | Status |
|:--|:--|:--|:--|
| Governance core | `packages/phase-mirror-gpt/` | Rust | Production-ready (ADR-007) |
| Orchestrator | `packages/phase-mirror-agency/agency-server/` | Node.js/Express | Functional scaffold |
| Compute engine | `packages/q-calculator/` | React/TS + WASM | Standalone demo app |
| MCP transport | `packages/phase-mirror-mcp/` | Rust | Implemented |
| UI derivative | `packages/phase-mirror-agency/admin/` | React/TS | Wireframe |
| Prime substrate | `Prime/` | Rust (`pirtm-rs`, `multiplicity-mcp`) | Active research |
| Dissonance viz | `packages/phase-mirror-dissonance/` | — | Present |
| UI logic core | `packages/ui-core/` | Rust → WASM | Prototype |

Every component can run standalone. Every component can be wired to others. However, **no unified DevOps surface exists** that treats them as a single meta-ensemble governed by the Phase Mirror methodology. The current sites (`admin`, `citizen-gardens`, `q-calculator`) are demos; the real authority belongs in `agency-server`, which currently lacks depth.

The PC at `/home/multiplicity/Multiplicity/Phase Mirror/` is the **dev headquarters**. What ships externally must first be validated here. Therefore, the local deployment surface must become the canonical orchestration environment.

### Advancements in `Prime/` to incorporate
- `Prime/crates/mcp` -> `multiplicity-mcp`: governed MCP server with ALP policy gate, stdio + HTTP/SSE transport, JSON-RPC 2.0 tool discovery (Phase 3 complete).
- `Prime/crates/core` -> `pirtm-rs`: PIRTM (Prime-Indexed Recursive Tensor Math) engine, prime-indexed canonical forms, contractivity operators.
- `Prime/crates/pirtm-ui`: WASM-ready PIRTM surface.
- `Prime/AGENTS.md` -> Phase 4 roadmap: offline-first, schema-driven, governance-first CLI (`pscmd`).

---

## Decision

Adopt the **Agency as Meta-Ensemble** architecture. Overhaul `packages/phase-mirror-agency/` into a unified DevOps UI that selects the best implementation from each component and composes them through the governed Phase Mirror spine.

### Architectural Tenets
1.  **Single Pane of Glass**: One React/TS DevOps UI replaces `admin/`, `citizen-gardens/`, and the `q-calculator` standalone shell.
2.  **Every component standalone**: Each domain package retains a production entry point independent of the meta-UI.
3.  **Every component combinable**: The meta-UI embeds and orchestrates components via MCP or REST.
4.  **Governance-first**: All operator-facing paths route through the Rust MCP server (`multiplicity-mcp`) and its ALP gate; the UI is a strict transformer of engine-computed facts (zero drift).
5.  **Local-first HQ**: Dev headquarters runs the canonical meta-ensemble; external deployments mirror this exact topology.

### Meta-Ensemble Topology

```
┌─────────────────────────────────────────────────────────────┐
│  Agency DevOps Meta-Ensemble (local headquarters)           │
│                                                             │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────────┐   │
│  │  React/TS    │  │  Zustand     │  │  Tailwind +     │   │
│  │  DevOps UI   │◄─►│  Stores      │  │  Headless UI    │   │
│  │  (port 5173) │  │              │  │                 │   │
│  └──────┬───────┘  └──────────────┘  └─────────────────┘   │
│         │                                                     │
│  ┌──────▼───────────────────────────────────────┐           │
│  │  agency-server (Node.js, port 8082)          │           │
│  │  - Auth (JWT + bcrypt)                       │           │
│  │  - SSE event bus                             │           │
│  │  - Archivum ledger bridge                    │           │
│  │  - MCP client → phase-mirror-gpt             │           │
│  │  - Binary proxy (Triple-Lock harnesses)      │           │
│  └──────┬───────────────────────────────────────┘           │
│         │                                                     │
│  ┌──────▼───────────────────────────────────────┐           │
│  │  phase-mirror-gpt (Rust, stdio/MCP)          │           │
│  │  - L0 Bitmask Validator                      │           │
│  │  - Triple-Lock (Guardian/Examiner/Publisher) │           │
│  │  - Λ-Archivum WAL                            │           │
│  │  - Semantic policy hot-reload                │           │
│  │  - MCP transport (JSON-RPC 2.0)              │           │
│  └──────┬───────────────────────────────────────┘           │
│         │                                                     │
│  ┌──────▼───────────────────────────────────────┐           │
│  │  multiplicity-mcp (Rust, governed)           │           │
│  │  - ALP policy gate                           │           │
│  │  - Tool descriptor registry                  │           │
│  │  - σ kernel state transitions                │           │
│  └──────┬───────────────────────────────────────┘           │
│         │                                                     │
│  ┌──────▼───────────────────────────────────────┐           │
│  │  Compute Substrates (standalone + embedded)  │           │
│  │  - pirtm-rs (WASM) → Q-Calculator engine     │           │
│  │  - q-calculator FastAPI (port 7070)          │           │
│  │  - ui-core-rs (WASM → React hooks)            │           │
│  └──────────────────────────────────────────────┘           │
└─────────────────────────────────────────────────────────────┘
```

---

## Consequences

### Positive
- **Operational consistency**: Every UI action flows through the same policy-verified Rust path, satisfying the Sedona Spine Mandate and Ξ-Constitution.
- **Unified observability**: Single SSE event stream, single Archivum, single witness chain replaces four separate log files.
- **Component karma preservation**: `q-calculator`, `admin`, `citizen-gardens`, and `discord-bot` remain runnable standalone via their own `start.sh`/`compose.yaml`. The DevOps UI embeds them as iframes or proxy routes.
- **Prime readiness**: `pirtm-rs` WASM module plugs directly into the React surface; `multiplicity-mcp` becomes the governed CLI backend.
- **Offline-first**: The meta-ensemble runs fully without network; each component uses local storage or file-backed SQLite.

### Risk
- **Centralization surface**: If `agency-server` fails, the UI breathes. Mitigation: each component retains its standalone port; the UI gracefully degrades to individual app URLs.
- **Migration debt**: Existing `admin/` and `q-calculator/` codebases must be audited before embedding. Use phased extraction rather than bulk copy-paste.
- **Governance boundary**: The React UI runs in the browser (untrusted). All enforcement stays server-side in Rust. The UI is strictly a transformer of engine facts.

---

## Implementation Plan

### Phase 0 — Audit & Best-Component Selection

Goal: inventory every working component, choose the best implementation per domain, document migration paths.

| Domain | Current Candidates | Selected | Rationale |
|:--|:--|:--|:--|
| React shell | `q-calculator/App.tsx`, `admin/src/App.tsx` | `q-calculator/` layout | 3-panel app shell, sidebar, state-driven views already implemented. |
| State management | `q-calculator` (inline), `admin/` (none) | Zustand + devtools | Matches UI_Plumbing blueprint; explicit stores per domain. |
| API client | `q-calculator/server.ts`, `agency-server/src/index.js` | `agency-server` REST + SSE | Already proxies to Rust binaries; extend rather than replace. |
| Binary orchestration | `agency-server/src/index.js` (child_process.exec) | Keep `exec`, sandbox via `isolated-vm` or explicit allow-list | Functional for local dev; Phase 5 adds container isolation. |
| Graph / Dissonance | `admin/` (mock), `phase-mirror-dissonance/` | `phase-mirror-dissonance` crate + Cytoscape.js | Real data source; mock data removed. |
| Q-Calculator engine | `Legacy/q-calculator/` (Python), `q-calculator/wasm/` (WASM) | `q-calculator/wasm/` (WASM) | WASM runs in the React UI without Python dependency for demo paths. Python service retained for heavy PIRTM jobs via REST. |
| Governed MCP | `Prime/crates/mcp/` | `multiplicity-mcp` | Already implements ALP gate, stdio + HTTP/SSE transports. |
| Policy engine | `packages/phase-mirror-gpt/src/domain_invariants.rs` | Rust + hot-reload TOML | current production code; proven sub-250ns L0 validation. |
| Ledger | `agency-server/var/archivum/ledger.jsonl`, `q-calculator` SQLite | Λ-Archivum WAL in `phase-mirror-gpt` | Single source of truth; UI reads via agency-server REST. |
| Telemetry | `phase-mirror-gpt/src/telemetry.rs`, `q-calculator/src/metrics.py` | `phase-mirror-gpt` Prometheus metrics | Rust-native; Python metrics adapter for Q-Calculator if needed. |

**Gate**: Document written to `packages/phase-mirror-agency/MIGRATION.md`. Each standalone component verified `npm run dev` or `cargo run` after selection.

---

### Phase 1 — Unified Meta-Orchestrator

Refactor `packages/phase-mirror-agency/agency-server/` from a thin Express proxy into the canonical orchestration layer.

#### 1a. Structural rewrite
- Convert `src/index.js` to **TypeScript** (`src/index.ts`) with explicit route modules.
- Replace `child_process.exec` for binary dispatch with a **sandboxed spawn** utility:
  - Allow-list binary paths from `agency-server/bin/` and `phase-mirror-gpt/bin/`.
  - Return `UnifiedWitness` JSON parsed from stdout.
- Add route modules:
  - `routes/health.ts` — `/v1/health`
  - `routes/auth.ts` — JWT login (bcrypt, 24h token)
  - `routes/archivum.ts` — `GET /v1/agency/archivum/ledger`, `POST /v1/agency/archivum/register`
  - `routes/triple-lock.ts` — `POST /v1/agency/coding-commander/completions`, examiner/publisher proxies
  - `routes/dissonance.ts` — `GET /v1/agency/dissonance/graph` (from `phase-mirror-dissonance`)
  - `routes/governance.ts` — `GET /v1/agency/daemon/metrics`, `GET /v1/agency/adrs`
  - `routes/wasm.ts` — bridge to `pirtm-ui` WASM and `ui-core-rs` WASM for client-side verification
  - `routes/mcp.ts` — WebSocket SSE bridge to `multiplicity-mcp` (HTTP/SSE transport, Phase 3 Step 6)

#### 1b. Binary compilation pipeline
- `compile_binaries.sh` builds:
  1. `phasemirror-agency` -> `bin/coding-commander`
  2. `phase-mirror-gpt` -> `bin/phase-mirror-gpt`
  3. `pirtm-rs` (release) -> `bin/pirtm-core`
  4. `ui-core-rs` (wasm32-unknown-unknown) -> `wasm/ui_core_bg.wasm`
- Add a `Makefile` target `make binaries` that chains all four.

#### 1c. Config management
- Replace `.env` scattering with a single `agency-server/config/default.yaml`:
  - `paths.binaries[]`
  - `mcp.endpoint`
  - `archivum.wal_path`
  - `prime.enabled`
  - `qcalculator.wasm_path`
- Load via `dotenvy` + `yaml-rust2` (or Node `yaml`).

**Gate**: `npm run build` succeeds; all routes return 200 with stub/mock data if Rust binaries are missing.

---

### Phase 2 — DevOps UI (Single Pane of Glass)

Build the React UI at `packages/phase-mirror-agency/ui/` (or rebrand the existing `admin/` frontend).

#### 2a. Layout
Extract the 3-panel shell from `q-calculator/App.tsx`:
- **Left sidebar**: agent registry (Coding-Commander, Ataraxia, Finton, The Guardian, The Examiner, The Publisher), ensemble status, navigation groups (Governance, Compute, Projects, Fleet).
- **Center**: router outlet — Dashboard, Dissonance Graph, Triple-Lock Inspector, Q-Calculator, PIRTM Workspace, Archivum, Settings.
- **Right sidebar**: live telemetry (L0 validator latency, witness count, spectral radius, contraction budget, prime-ledger PETC stats).

#### 2b. Zustand stores (mirroring UI_Plumbing blueprint)
| Store | Domain | Backend Route |
|:--|:--|:--|
| `authStore` | JWT, RBAC | `/v1/auth/login` |
| `governanceStore` | ADRs, Triple-Lock missions | `/v1/agency/coding-commander/completions`, `/v1/agency/adrs` |
| `dissonanceStore` | Dissonance graph nodes/edges | `/v1/agency/dissonance/graph` |
| `archivumStore` | Ledger entries, search | `/v1/agency/archivum/ledger` |
| `metricsStore` | Daemon metrics, Prometheus export | `/v1/agency/daemon/metrics` |
| `qcalcStore` | PIRTM jobs, WASM state, Python fallback | `/api/*` (if embedded) or direct WASM invocation |
| `fleetStore` | Agents, projects, ensemble harnesses | `/v1/agency/agents`, `/v1/agency/projects` |

#### 2c. Component extraction (the "best components" copy)
From `q-calculator/`:
- `AtomicLogo`, `Sidebar`, `RightPanel` (safety meter / prime ledger / provenance widget).
- `ProjectsView` (card grid with run/pass/residual metrics).
- `SimulatorView` pattern (strata toggles + WASM execution).

From `phase-mirror-agency/admin/src/components/`:
- `MainLayout`, navigation shell, ADR registry table.

From existing `agency-server/src/index.js`:
- Dissonance graph endpoint logic (mock → real via `phase-mirror-dissonance` crate).

New components:
- `TripleLockInspector` — shows Guardian/Examiner/Publisher verdict per mission.
- `ArchivumTimeline` — live-updating ledger with witness hash details.
- `PrimeLedger` — PETC conservation signature, prime-indexed strand counts.
- `MCPTerminal` — stdio/WebSocket MCP tool inspector (governed call viewer).
- `GovernanceGateStepper` — ADR verification flow with human-on-the-loop sign-off.

#### 2d. API layer
- `ui/src/api/client.ts` — fetch wrapper with auth headers, abort controller, audit hash extraction.
- Typed modules per domain: `governance.ts`, `archivum.ts`, `dissonance.ts`, `qcalc.ts`, `fleet.ts`.
- SSE hook: `useEventStream.ts` connecting to `/v1/agency/events`.

**Gate**: Dev server (`vite --port 5173`) proxies `/api` and `/v1` to `localhost:8082`; all five stores hydrate from live data.

---

### Phase 3 — Compute Substrate Embedding

Goal: make q-calculator and Prime substrates first-class citizens in the meta-UI, without losing standalone capability.

#### 3a. WASM embedding
- Compile `q-calculator/wasm/q_calculator_rs.js` + `q_calculator_rs_bg.wasm` into `agency-server/wasm-pkg/` (or serve via Vite).
- Compile `ui-core-rs` (`PrimeGate`, prime validation) to WASM and serve from `agency-server/wasm-pkg/ui_core.js`.
- UI routes `/v1/wasm/qcalc/run` and `/v1/wasm/uicore/validate` through Node, or call WASM directly from React.

#### 3b. Q-Calculator REST fallback
- When WASM execution exceeds a configurable threshold or the user requests a full PIRTMC step, the UI calls `agency-server` which proxies to:
  - `localhost:7070` (`q-calculator` FastAPI v2) if running, OR
  - A spawned `pirtm-core` binary for offline computation.
- The server records the execution provenance entry in Λ-Archivum.

#### 3c. PIRTM Workspace
- New view in the UI: "PIRTM Workspace".
- User selects active strata (0, 1, 2, 3, 4, 7, 13, 99) — same registry as `q-calculator/App.tsx`.
- WASM engine computes `run_simulation(x0, xi, lam, g)` client-side for <1s feedback; server-side for heavy runs.
- Results render with contraction budget meter, operator norm, GAP_LB, and conservation signature (Π = 2,520 pattern).

**Gate**: `q-calculator` still runs standalone via `npm run dev` on port 7070 (or whatever custom port); the DevOps UI does not break its independent deploy.

---

### Phase 4 — Prime Advancements Integration

Wire `Prime/` substrates into the meta-ensemble.

#### 4a. ALP + σ kernel via MCP
- `multiplicity-mcp` exposes the tool registry defined in `packages/phase-mirror-gpt/mcp-contract.json`.
- `agency-server` acts as an MCP client (stdio transport to `multiplicity-mcp` binary) and as an SSE-facing MCP gateway for the React UI.
- The UI never calls `multiplicity-mcp` directly; all calls go through `/v1/mcp-tools/call`.

#### 4b. ψcmd CLI integration
- `agency-server` exposes a `POST /v1/agency/cli/execute` route backed by `multiplicity-mcp` rather than raw `exec`.
- The React "Terminal" panel sends commands through the ALP gate; policy violations return `403` with witness hash.
- Local devs can also use the `pscmd` binary from `Prime/` directly; the UI is optional.

#### 4c. Prime-ledger PETC display
- Extract the "Prime Ledger (PETC)" widget from `q-calculator/App.tsx` into a shared component.
- Render live prime counts `M(e) = p^count` and conservation signature Π.
- Backed by `phase-mirror-gpt` telemetry or `q-calculator` WASM.

#### 4d. Sovereignty & Witness display
- Every UI state mutation that triggers a backend action shows its `witness_hash`, `p_lineage`, and `governance_status` (VERIFIED / BLOCKED / WARNING).
- Matches the transparency contract of ADR-001 and Ξ-Constitution Article VIII.

---

### Phase 5 — Deployment, CI/CD, and Fleet Management

#### 5a. Local dev HQ script
- New `meta-start.sh` replaces `start-all.sh`:
  1. Compiles all Rust binaries (`make binaries`).
  2. Starts `agency-server` (port 8082).
  3. Starts `q-calculator` FastAPI (port 7070) in background for REST fallback.
  4. Starts DevOps UI Vite dev server (port 5173) with API proxy.
  5. Optionally starts `discord-bot` and `citizen-gardens` if running full external fleet.
- Each process logs to `logs/<service>.log`; `trap 'kill 0' EXIT` for teardown.

#### 5b. Docker Compose (local mirror)
```yaml
services:
  agency-server:
    build: ./packages/phase-mirror-agency/agency-server
    ports: ["8082:8082"]
    volumes: ["./bin:/app/bin", "./wasm-pkg:/app/wasm-pkg"]
  phase-mirror-gpt:
    build: ./packages/phase-mirror-gpt
    ports: [] # stdio only; embedded by agency-server
  q-calculator-rest:
    build: ./packages/q-calculator
    ports: ["7070:7070"]
  ui:
    build: ./packages/phase-mirror-agency/ui
    ports: ["5173:5173"]
    depends_on: [agency-server]
```

#### 5c. GitHub Actions (extend existing)
- `ci.yml` runs:
  1. `cargo test --test governance` in `packages/phase-mirror-gpt/` and `Prime/`.
  2. `cargo test` in `packages/ui-core/`.
  3. `npm run lint && npm run build` in `packages/phase-mirror-agency/agency-server/` and `packages/phase-mirror-agency/ui/`.
  4. `npm run build` in `packages/q-calculator/`.
  5. End-to-end test harness (`packages/phase-mirror-agency/test-harness.sh`) against the spun-up Docker Compose stack.

#### 5d. Observability
- `phase-mirror-gpt` Prometheus `/metrics` exposed via `agency-server` at `/v1/agency/daemon/metrics`.
- UI `MetricsPage` renders Grafana-embeddable panels: witness creation rate, policy violation count, L0 latency histogram, agent P&L.
- Λ-Archivum append-only ledger mirrored to SQLite for full-text search in the UI.

#### 5e. Release artifact
- `agency-server/deploy_package.sh` extended to include:
  - `bin/*` binaries
  - `ui/dist/` static assets
  - `wasm-pkg/*.wasm` + JS glue
  - `config/default.yaml`
- Output: `deploy_artifacts/agency-meta-ensemble.zip`.

---

## Rollout Sequence

1. **Week 1**: Phase 0 audit + `MIGRATION.md`. Confirm each component `cargo run` / `npm run dev` to baseline.
2. **Week 2-3**: Phase 1 orchestrator rewrite. TS routes, binary pipeline, config YAML.
3. **Week 4-5**: Phase 2 UI shell + stores. Stub data first, then wire live.
4. **Week 6**: Phase 3 WASM embedding + Q-Calculator integration.
5. **Week 7**: Phase 4 Prime/MCP integration + PETC display.
6. **Week 8**: Phase 5 Docker Compose, CI gates, `meta-start.sh`, deploy package.

### Success Criteria
- [`cargo test --test governance`] passes in `phase-mirror-gpt` and `Prime/`.
- `npm run build` succeeds for `agency-server` and `ui`.
- `meta-start.sh` brings up the full meta-ensemble on the dev HQ PC.
- All standalone component entry points (`q-calculator npm run dev`, `phase-mirror-gpt cargo run`, `multiplicity-mcp cargo run`) remain functional independently.
- Every UI-triggered backend action produces a `UnifiedWitness` in Λ-Archivum.

---

## References
- `packages/phase-mirror-gpt/ADR/007-deployment-readiness.md` — Production readiness baseline.
- `packages/phase-mirror-gpt/ADR/001-invariant-consistency-oracle.md` — Oracle architecture.
- `packages/phase-mirror-gpt/ADR/005-fail-closed-governance.md` — Fail-closed enforcement.
- `packages/phase-mirror-agency/GOVERNANCE.md` — Agent operational contract.
- `Ξ-Constitution.md` — Constitutional frame.
- `Prime/AGENTS.md` — Phase 4 roadmap, ALP/σ invariants.
- `docs/UI_Dev_Blueprint.md`, `docs/UI_Plumbing.md` — React/Zustand architecture reference.
- `packages/phase-mirror-agency/admin/ADR-IMPLEMENTATION-PLAN.md` — Predecessor dashboard plan.

---

*Prepared by the PhaseSpace Commander Coding Agent on 2026-07-01.*
<!-- LawfulRecursionVersion:1.0 -->
