# Production Rollout: Gap Analysis & Prioritized Steps

## ADR-009 vs ADR-010 Comparison

### ADR-009: CNL Compiler over PIRTM

| ADR Requirement | Current State | Gap | Maturity |
|:--|:--|:--|:--|
| Lexical Anchor Table | ✅ Hardcoded in `cnl.rs` (9 tokens) | Expand to full domain vocabularies | Prototype |
| Semantic Compiler → MOCWord | ✅ `cnl.rs` compiles to `StratumBoundary` | None | Prototype |
| Phase Mirror Invariant Enforcer | ✅ `enforce_all` blocks `Ap(1)`, enforces `c < 1.0` | Add all L0-L9 gates | Prototype |
| DialogueFrame with Stratum Nesting | ✅ Assert/Retract modes, anaphora "it" | Add Query mode, multi-object resolution | Prototype |
| Revoke Operator (`Ap(19)`) | ✅ Implemented, neutralizes prior stratum | Prove `Ap(p) ◦ Ap(p_inv)` = neutral algebraically | Prototype |
| Topological Suggestion Engine | ✅ Prime-distance adjacency, cost function | Pre-compute adjacency graph offline; cache suggestions | Prototype |
| Execution Bridge (Docker) | ✅ `tokio::process::Command` bridge | Swap bollard for typed Docker API; add K8s | Prototype |
| ALP Policy Gate | ✅ `alp-cli` binary + `/v1/alp/evaluate` route | Wire ALP into CNL `compile_and_verify` | Scaffold |
| Audit Trail | ✅ Λ-Archivum WAL + agency-server REST | Bind CNL steps to `UnifiedWitness` entries | Partial |
| Lean 4 Formalization | ❌ Not started | Formalize CNL compiler + invariant proofs | Not Started |
| WASM Compilation | ❌ Not started | Compile `cnl.rs` to WASM for browser-native execution | Not Started |
| Domain Expansion | ❌ DevOps only | Legal, medical, financial lexicons | Not Started |
| Multi-Party Consensus | ❌ Mock only | Wire `ConsensusVerifier` + Pell VDF | Mock |

### ADR-010: Phase Mirror Kubernetes Operator

| ADR Requirement | Current State | Gap | Maturity |
|:--|:--|:--|:--|
| CRD (`PhaseMirrorCommand`) | ✅ `crd.yaml` + kube-rs derive | Add `invariantChecks` array to status | Scaffold |
| Controller / Reconciler | ✅ `Controller::new` + async reconcile | None | Scaffold |
| CNL Compiler Integration | ❌ Mock `compile_and_verify` | Wire real `pirtm-apps` CNL compiler | Mock |
| ConsensusVerifier Integration | ❌ Mock `verify_consensus` (always returns `true`) | Wire real `commander-core` + Pell VDF | Mock |
| Execution Bridge (K8s) | ❌ Commented-out typed Deployment API | Uncomment, complete Revoke delete logic, add Scale/Destroy | Mock |
| WebSocket Event Stream | ✅ `ws_broadcast.rs` exists | Wire all event types to operator | Partial |
| Typed K8s API | ✅ `k8s_openapi::apps::v1::Deployment` imported | Uncomment and test `create`/`delete` | Partial |
| Finalizers & RBAC | ❌ Not started | Add cleanup finalizers, least-privilege ClusterRole | Not Started |
| Helm Chart | ❌ Not started | Package operator for deploy | Not Started |
| Integration Tests | ❌ Not started | End-to-end test with kind cluster | Not Started |
| Web UI (`pirtm-ui`) | ✅ Crate exists, `main.rs` minimal | Build Axum WebSocket server + HTML terminal | Scaffold |

---

## Prioritized Production Implementation Steps

### P0 — Must Have Before Any Production Exposure

1. **Wire real CNL compiler into operator reconciler**
   - Replace `compile_and_verify` mock with `pirtm-apps::compile_command` (needs clean public API extraction from `cnl.rs`)
   - Ensure `Result` types propagate invariant violations correctly
   - Files: `Prime/crates/pirtm-apps/src/lib.rs`, `Prime/crates/phase-mirror-operator/src/reconciler.rs`

2. **Wire real ConsensusVerifier + Pell VDF into operator**
   - Replace `ConsensusVerifier` mock with `commander-core::ConsensusVerifier::verify_consensus`
   - Add `PellVDF` proof verification path
   - Files: `Prime/substrates/the-commander/crates/commander-core/src/`, `Prime/crates/phase-mirror-operator/src/reconciler.rs`

3. **Complete K8s Execution Bridge**
   - Uncomment `deployments.create` and `deployments.delete` in `execute_action`
   - Add `Scale` and `Destroy` arms to `VerifiedAction`
   - Add proper error mapping (`kube::Error` → `PhaseMirrorStatus::Failed`)
   - Files: `Prime/crates/phase-mirror-operator/src/reconciler.rs`

4. **Secure Modulus Generation**
   - Generate fresh 2048-bit RSA modulus in disposable environment
   - Store in HSM/secure enclave; never in source
   - Files: `Prime/substrates/goldilocks-pro/` or dedicated `crates/vdf-config/`

### P1 — Required for Production Hardening

5. **Structured Observability**
   - Add `tracing` spans to entire CNL → MOCWord → VDF → Execution pipeline
   - Expose Prometheus metrics: VDF latency, STARK proof time, consensus throughput, invariant violations
   - Files: operator, pirtm-apps, commander-core

6. **Deterministic Build**
   - Pin `rust-toolchain.toml`
   - Add Dockerfile producing single static binary for operator
   - Sign binary with Ed25519 key; commit signature to Λ-Archivum

7. **Key Management**
   - Ed25519 governance keypair generation (offline)
   - `zeroize` crate for sensitive memory (VDF seeds, private keys)
   - Key rotation strategy

8. **Error Handling Typing**
   - Replace `anyhow!` with typed error enums (`OperatorError::InvariantViolation`, `OperatorError::ConsensusFailed`, `OperatorError::ExecutionFailed`)
   - Propagation to CR status and Kubernetes Events

### P2 — Operator Ecosystem

9. **Kubernetes Operator Polish**
   - Add finalizers for CR deletion cleanup
   - RBAC: least-privilege `ClusterRole` (create/patch/delete only on `deployments` and `services`)
   - Helm chart for installation
   - Integration tests using `kind` or `k3d`

10. **Real-Web UI (`pirtm-ui`)**
    - Axum WebSocket server streaming `OperatorEvent` types
    - Matrix-styled HTML terminal (single page, no frameworks)
    - Live `c` and `R_sc` gauges
    - Serve static HTML from Axum via `tower-http`

11. **CNL Compiler Public API**
    - Extract `compile_command` and `InvariantResult` into clean `pirtm-apps/src/lib.rs` API
    - Remove hardcoded demo logic; parameterize lexicon loading
    - Add `CompilationResult::new(command: &str, lexicon: &Lexicon) -> Result<Self>`

### P3 — Scaling & Formal Verification

12. **Lexicon Expansion Tooling**
    - Automate `lexicon_verify.rs` to run in CI on lexicon changes
    - Generate "lexicon profile" artifacts for each domain
    - Pre-compute suggestion caches for top-N error patterns

13. **Lean 4 CNL Formalization**
    - Formalize Lexical Anchor Table + grammar rules in Lean 4
    - Prove `∀ sentence ∈ CNL grammar. c < 1.0 ∧ R_sc ≥ 1.0`
    - Link proofs to existing `MOC/PellVDF.lean`

14. **Multi-Domain Lexicons**
    - Legal CNL (contracts, obligations, termination)
    - Medical CNL (prescriptions, contraindications)
    - Financial CNL (transactions, settlements)

15. **WASM Compilation**
    - Compile CNL compiler to WASM for zero-latency browser execution
    - Only execution events traverse to server; verification is client-side

### P4 — Community & Deployment

16. **Whitepaper & Documentation**
    - Full architecture paper: `F1 → CNL → Phase Mirror → Goldilocks STARKs → Pell VDF → UnifiedWitness → Execution`
    - Operator's Manual: adding lexicons, onboarding parties, interpreting diagnostics
    - API reference for `PhaseMirrorCommand` CRD

17. **Open Source Release**
    - License: Apache 2.0 for Rust/Lean components
    - Publish crates: `pirtm-apps`, `phase-mirror-operator`, `goldilocks-pro`
    - `CONTRIBUTING.md` + lexicon contribution guide

18. **Mic-Drop Demo**
    - Split-screen video: Matrix UI → English command → live `c/R_sc` → Pell VDF countdown → K8s rollout → revoke rollback
    - Narrate: "This is not AI. This is mathematics."

---

## Critical Path to MVP

```
Week 1:    P0-1 (CNL API) → P0-3 (K8s execution) → P0-4 (secure modulus)
Week 2:    P1-1 (observability) → P1-2 (deterministic build) → P1-3 (key mgmt)
Week 3:    P1-4 (typed errors) → P2-1 (operator polish) → P2-2 (WebSocket UI)
Week 4:    P2-3 (CNL public API) → P2 integration test → kind cluster demo
Week 5-8:  P3 (formal verification + lexicon CI) → P4 (paper + release)
```

## Risk Register

| Risk | Mitigation |
|:--|:--|
| CNL compiler API extraction stalls P0 | Extract minimal `compile_command` wrapper first; defer refactoring |
| K8s execution bugs cause data loss | Use mock executor in CI; test against `kind` before production cluster |
| Modulus generation compromised | Generate in air-gapped disposable container; destroy primes immediately |
| Operator RBAC too permissive | Audit with `kubectl auth can-as`; use `Namespaced` scope with explicit resource lists |
| Lexicon expansion breaks invariants | CI gate: `lexicon_verify` must pass before merge |

---

## Summary

The CNL compiler prototype (ADR-009) and Kubernetes operator skeleton (ADR-010) are **functional scaffolds** but contain significant mocks. The highest-leverage next step is **P0-1 + P0-3**: extract the real CNL compiler into a public API and wire it into the operator reconciler with real K8s execution. This transforms both ADRs from prototypes into a production-capable, mathematically governed infrastructure control plane.
