# ADR-004: Master Plan — Production-Grade Local Deployment of phase-mirror-agent

- Status: proposed
- Date: 2026-08-08
- Owners: Multiplicity Foundation
- Tags: #master-plan, #production, #deployment, #phase-mirror-agent, #rust, #typescript, #governance
- Phase: phase-0 (baseline audit) → phase-7 (GA)
- Supersedes: none
- Related: ADR-001, ADR-002, ADR-005 … ADR-010

## 1. Context

`packages/phase-mirror-agent` is the high-integrity governance gateway for agentic AI
workflows. It is currently a **dual-language package**: a Rust server (the deployable
binary) and a TypeScript ALP-NLP library (the embedded governance core). Neither side
is production-grade, and the package has never been formally planned as a deployable
unit. This ADR is the master plan that closes that gap.

### 1.1 Current State Audit (evidence)

| # | Area | Finding | Evidence |
| :-- | :-- | :-- | :-- |
| A1 | Build | `Cargo.toml` dependency path is broken: `pirtm-apps = { path = "../../Prime/rust/pirtm-apps" }`; the crate lives at `Prime/packages/rust/pirtm-apps`. `cargo check` fails to load the manifest. | Verified: `cargo check` → "No such file or directory"; `cargo metadata` succeeds once path is corrected. |
| A2 | Build | `Dockerfile` `COPY` paths reference the same wrong `Prime/rust/*` tree, so the container image cannot build. | `Dockerfile` lines under "Internal pirtm crate chain". |
| A3 | Runtime | `cnl.rs` `compile_command` returns **hardcoded** `c: 0.84`, `rsc: 209.3`, and fixed action parameters (`service = "web-service"`, `target = "cluster"`, `replicas = 3`) regardless of input. | `Prime/packages/rust/pirtm-apps/src/cnl.rs:125-134`. |
| A4 | Runtime | `tools.rs` `deploy`/`scale`/`destroy` return **fake string receipts**; no real executor, no idempotency, no actual side effects. | `src/tools.rs`. |
| A5 | Audit | `AuditStore` is **in-memory** (`RwLock<Vec<AuditEntry>>`); all entries lost on restart; `DELETE /audit/entries/{id}` allows mutation — violates append-only/immutable audit intent. | `src/audit_api.rs`. |
| A6 | Security | No authentication, no authorization, no rate limiting; CORS allows arbitrary methods; TLS is optional and off by default. | `src/main.rs`. |
| A7 | Reliability | `/health` and `/ready` are static JSON; no dependency checks; shutdown aborts the WebSocket task rather than draining it. | `src/main.rs:171-183`, `src/ws_broadcast.rs:90-93`. |
| A8 | TS core | `generateWitnessId()` uses `Math.random()` (not CSPRNG); `archivum.ts` appends JSONL **without fsync or hash-chaining**; witness integrity is not anchored. | `src/alp-nlp/witness.ts:33-37`, `src/alp-nlp/archivum.ts`. |
| A9 | Schema | No versioned JSON Schema for the witness/command contracts; `scripts/export-schema.ts` exists but is unused; `ajv` dependency is unused. | `package.json`, `scripts/export-schema.ts`. |
| A10 | Tests | **Zero Rust tests** in the package; TS tests exist (21 passing) but are not wired into CI for this package. | `tests/` (TS only), no `#[cfg(test)]` in `src/*.rs`. |
| A11 | State | `.gitignore` excludes `state/`; witness archives are written but never versioned/backed up. | `.gitignore`, `state/` (empty). |
| A12 | Ops | No package README, `docs/` empty, no runbooks, no systemd units, no secrets management; `dotenv` loads uncommitted `.env` with no validation. | `docs/`, root `Cargo.toml` vs `package.json` version mismatch (0.1.0 vs 1.0.0). |

### 1.2 Constraints

- Must comply with the Ξ-Constitution L0 invariants and the Sedona Spine Mandate
  (non-bypassability, immutable audit, zero drift) already adopted in ADR-001/002.
- Must remain deployable on a **single local host** as the primary target, with Docker
  Compose and systemd as equivalent first-class deployment models.
- Must not regress the ALP-NLP deterministic NLP contract (ADR-001).
- Must keep both runtime surfaces: the Rust server (deployed) and the TS ALP-NLP
  library (embedded by `operator-ui`/clients).

## 2. Decision

We adopt a **7-phase master roadmap** that brings `phase-mirror-agent` to a defined
"production-grade local deployment" bar. Each phase has a dedicated child ADR that
owns the detailed decision. The master plan fixes blocking defects first, then hardens
each concern in dependency order.

### 2.1 Definition of "Production-Grade Local Deployment" (the bar)

A local deployment is production-grade when all of the following hold:

1. **Builds reproducibly** from a clean checkout in CI with pinned toolchains.
2. **Survives restart** without data loss (durable, append-only audit store).
3. **Authenticates and authorizes** every write path; rate-limits public surfaces.
4. **Observes** itself: structured logs, metrics, and accurate liveness/readiness.
5. **Shuts down gracefully** (SIGTERM drain, no truncated writes).
6. **Runs unprivileged** with least-privilege filesystem and network exposure.
7. **Executes tools** through a governed, allow-listed backplane — no stub receipts.
8. **Is documented** (README, deployment guide, runbook, backup/restore).
9. **Is verifiable**: automated tests gate every promotion, and governance invariants
   are encoded as testable assertions and (where feasible) Kani proofs.

### 2.2 Non-Goals (explicitly out of scope)

- Multi-node / Kubernetes orchestration.
- Cloud managed services.
- Conversational LLM NLP (per ADR-001).
- Adding new tool integrations beyond the allow-list framework (ADR-008 owns the hook).

### 2.3 Package Layout Target

```
packages/phase-mirror-agent/
├── Cargo.toml               # Rust server (deployable binary)
├── package.json             # TS ALP-NLP library (embedded core)
├── src/                     # Rust: main.rs, api/*, audit/*, tools/*, ws/*
├── alp-nlp/src/             # TS: lexer, parser, policy, witness, archivum
├── schema/                  # JSON Schema (single source of truth, exported)
├── config/                  # default config + .example files
├── deploy/
│   ├── compose.yaml
│   └── systemd/phase-mirror-agent.service
├── scripts/                 # migration, backup/restore, runbooks
├── docs/adr/                # this ADR series
└── docs/                    # README + deployment guide + runbooks
```

## 3. Master Roadmap

Each phase lists its owning child ADR, exit criteria, and key files.

### Phase 0 — Baseline: build + verify (Owner: ADR-010)

- Fix `Cargo.toml` pirtm path to `../../Prime/packages/rust/pirtm-apps`.
- Fix `Dockerfile` crate-chain `COPY` paths.
- Add minimal Rust unit tests for `cnl` glue, audit store, tools registry.
- Wire `cargo build --release`, `cargo test`, `tsc`, and `vitest` into CI for this package.
- **Exit:** `cargo check`, `cargo test`, `npm run build`, `npm test` all green on a clean checkout.

### Phase 1 — Durable, immutable audit store (Owner: ADR-005)

- Replace in-memory `AuditStore` with an append-only JSONL WAL (fsync, hash-chained).
- Remove `DELETE /audit/entries/{id}`; add explicit append-only enforcement.
- Bring TS `archivum.ts` to parity (CSPRNG ids, fsync, chain anchoring).
- Rotation + backup/restore scripts.
- **Exit:** entries survive restart; chain integrity verifiable; no mutation endpoints.

### Phase 2 — Governance correctness (Owner: ADR-008)

- Replace hardcoded CNL output with real token→action compilation backed by `pirtm`
  invariants; surface real `c`, `R_sc`, diagnostic values.
- Introduce a governed tool-execution backplane (trait + allow-listed adapters) so
  `deploy`/`scale`/`destroy` produce real receipts or fail loudly — never stubs.
- Generate real witness hashes (SHA-256/Ed25519) for every admitted action.
- **Exit:** no hardcoded metrics remain; tool receipts are verifiable; witnesses hash-anchored.

### Phase 3 — Security: authn/authz, rate limiting, TLS (Owner: ADR-006)

- Bearer-token/API-key authentication for write paths; scoped permissions.
- Rate limiting on `/api/*`; tighten CORS to explicit origin allow-list (no wildcard methods).
- Secrets via environment/files with `0600` permissions; no secrets in logs.
- TLS enabled-by-default guidance; optional mTLS for operator clients.
- **Exit:** unauthenticated writes rejected; rate limits enforced; default secure posture.

### Phase 4 — Observability & reliability (Owner: ADR-007)

- Structured JSON logs (tracing-subscriber), single line per event.
- `/metrics` (counters for commands, rejections, audit writes, WS clients).
- Liveness vs readiness split; readiness reflects audit-store writability.
- Graceful SIGTERM/SIGINT drain for HTTP + WS; flush audit WAL before exit.
- **Exit:** full request lifecycle observable; shutdown flushes and drains.

### Phase 5 — Packaging & deployment (Owner: ADR-009)

- Working multi-stage Dockerfile; non-root runtime user; pinned toolchains.
- `deploy/compose.yaml`: persistent volume for `state/`, healthchecks, restart policy, TLS mounts.
- `deploy/systemd/*.service`: hardened units (no-new-privileges, read-only paths, user).
- Config schema + `.env.example` + validation at boot.
- **Exit:** `docker compose up -d` and `systemctl start phase-mirror-agent` both fully functional.

### Phase 6 — Documentation & runbooks (Owner: ADR-009)

- Package README, deployment guide (Compose + systemd), backup/restore, upgrade/migration.
- **Exit:** an operator with no prior context can deploy and recover.

### Phase 7 — Promotion & GA (Owners: all)

- Run the promotion criteria table (section 8); close gaps; accept child ADRs in order.
- Tag `v1.0.0` aligned across `Cargo.toml` and `package.json`.

## 4. Consequences

### Positive
- Every concern is owned by a single ADR with testable exit criteria — no silent gaps.
- The deployable server and the embedded NLP core are each brought to a defined bar
  without forcing a premature Rust-vs-TS consolidation.
- Fixing the broken pirtm path unblocks CI, images, and any downstream consumer.

### Negative / Risk
- Durable audit store changes the `AuditEntry` contract (breaking change for any
  existing consumer of `/audit/*`); migration required (ADR-005).
- Removing `DELETE /audit/entries/{id}` removes a (mistaken) operational affordance.
- Real tool execution introduces blast-radius; mitigations: allow-list + dry-run mode (ADR-008).
- Scope is large; phases are sequenced so the package is never in a worse state than today.

### Neutral
- TS ALP-NLP remains a library; it is not the deployment surface.
- systemd and Compose are parity targets, not alternatives — both are supported.

## 5. Security & Governance

The plan upholds the Sedona Spine Mandate:

1. **Non-Bypassability** — all command and tool-execution paths route through the ALP
   gate and the governed tool backplane; no ungoverned endpoint (ADR-006, ADR-008).
2. **Immutable Audit** — the audit store becomes append-only and hash-chained; mutation
   endpoints are removed; WAL flushed before shutdown (ADR-005, ADR-007).
3. **Zero Drift** — metrics and witness values derive from the `pirtm` invariant core,
   not hardcoded constants; CI gates every promotion (ADR-010).
4. **Least Privilege** — unprivileged runtime, scoped tokens, read-only config mounts (ADR-006, ADR-009).

## 6. Dependencies

- `Prime/packages/rust/pirtm-apps` (+ `pirtm-registry`, `pirtm-dist`, `pirtm-invariants`, `pirtm-stdlib`) — path corrected in Phase 0.
- ADR-001 (ALP-NLP replaces LLM) and ADR-002 (production-grade ALP-NLP) — governing contracts.
- Root `docs/adr/ADR-DEPLOY-001.md` — repo-level deployment readiness (this ADR is the package-specific counterpart).
- Root `docs/deployment/README.md` — deployment guide to be extended with agent-specific sections.

## 7. Promotion Criteria

| Criteria Type | Description | Target / Threshold | Owner | Status |
| :--- | :--- | :--- | :--- | :--- |
| **Build** | `cargo build --release` + `npm run build` from clean checkout | 0 errors | ADR-010 | ⬜ |
| **Tests** | `cargo test` + `vitest` + `tsc --noEmit` | 100% pass | ADR-010 | ⬜ |
| **Durability** | Audit entries survive restart; chain verifiable | No data loss; chain hash valid | ADR-005 | ✅ |
| **Immutability** | No mutation endpoints on audit store | `rg "DELETE"` audit routes = empty | ADR-005 | ✅ |
| **Security** | Unauthenticated writes rejected; rate limits enforced | 401/429 responses verified | ADR-006 | ✅ |
| **Observability** | JSON logs + `/metrics` + readiness reflects store | Probe checks green | ADR-007 | ✅ |
| **Graceful Shutdown** | SIGTERM drains WS + flushes WAL | Exit code 0, no truncated entries | ADR-007 | ✅ |
| **No Stubs** | No hardcoded `c`/`R_sc`/receipt strings in runtime paths | `rg` scan empty in `src/` | ADR-008 | ✅ |
| **Container** | `docker compose up -d` healthy with persistent volume | Healthcheck green after restart | ADR-009 | ✅ |
| **Systemd** | `systemctl start phase-mirror-agent` functional | Unit active; journal clean | ADR-009 | ✅ |
| **Docs** | README + deployment guide + backup/restore published | `docs/` non-empty, runbooks usable | ADR-009 | ✅ |
| **Version** | `Cargo.toml` and `package.json` aligned | Both `1.0.0` | all | ✅ |

## 8. Child ADR Index

| ADR | Title | Status |
| :-- | :-- | :-- |
| ADR-005 | Persistent, Immutable Audit Store | accepted |
| ADR-006 | Authentication, Authorization & Rate Limiting | accepted |
| ADR-007 | Observability & Reliable Shutdown | accepted |
| ADR-008 | Governed Tool-Execution Backplane | accepted |
| ADR-009 | Packaging & Deployment (Compose + systemd) | accepted |
| ADR-010 | Testing & CI Governance | proposed |

## 9. References

- `src/main.rs`, `src/audit_api.rs`, `src/tools.rs`, `src/ws_broadcast.rs`
- `Prime/packages/rust/pirtm-apps/src/cnl.rs`
- `src/index.ts`, `src/alp-nlp/*.ts`
- ADR-001, ADR-002, ADR-IMPLEMENTATION-PLAN (this directory)
