# ADR-010: Phase Mirror Kubernetes Operator

## Status
Proposed (2026-07-02)

## Context

The CNL compiler and execution bridge (ADR-009) currently run as CLI/REST processes. For production DevOps adoption, the Phase Mirror must become a native Kubernetes citizen: declarative, auditable, and reconcilable. Operators provide exactly this abstraction layer—watching custom resources, enforcing governance, and driving real infrastructure state.

## Decision

We will implement a Kubernetes Operator in Rust using `kube-rs` that watches `PhaseMirrorCommand` custom resources and reconciles them through the full Phase Mirror pipeline:

```
PhaseMirrorCommand CR → Controller (reconcile loop)
    ↓
CNL Compiler (pirtm-apps) → MOCWord + invariants (c, R_sc)
    ↓
ConsensusVerifier (Pell VDF + STARK)
    ↓
VerifiedAction → Kubernetes Deployment/Service/Job
    ↓
Status update on CR
```

## Custom Resource Definition

```yaml
apiVersion: apiextensions.k8s.io/v1
kind: CustomResourceDefinition
metadata:
  name: phasemirrorcommands.phase-mirror.io
spec:
  group: phase-mirror.io
  names:
    kind: PhaseMirrorCommand
    plural: phasemirrorcommands
    scope: Namespaced
    shortNames: [pmc]
  versions:
    - name: v1
      served: true
      storage: true
      schema:
        openAPIV3Schema:
          type: object
          properties:
            spec:
              type: object
              properties:
                command:
                  type: string
                  description: "Controlled Natural Language instruction"
                consensusWitness:
                  type: object
                  properties:
                    unifiedWitness:
                      type: string
                    consensusProof:
                      type: string
                executionTarget:
                  type: string
                lexicon:
                  type: string
                  description: "Domain lexicon identifier (e.g., devops, legal)"
              required: ["command"]
            status:
              type: object
              properties:
                phase:
                  type: string
                  enum: [Pending, Validated, Executing, Succeeded, Failed, Vetoed]
                message:
                  type: string
                cBound:
                  type: number
                rsc:
                  type: number
                executionReceipt:
                  type: string
                invariantChecks:
                  type: array
                  items:
                    type: object
                    properties:
                      name:
                        type: string
                      passed:
                        type: boolean
                      detail:
                        type: string
```

## Operator Architecture

### Crate
```
crates/phase-mirror-operator/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── reconciler.rs
│   ├── context.rs
│   └── executor/
│       ├── mod.rs
│       ├── deployment.rs
│       └── revoke.rs
└── crd.yaml
```

### Dependencies
- `kube` (0.93+) with `runtime`, `derive` features
- `k8s-openapi` (0.22+)
- `tokio` (full)
- `pirtm-apps` — CNL compiler, invariant enforcer
- `commander-core` — ConsensusVerifier, UnifiedWitness
- `tracing` / `tracing-subscriber` — structured observability

### Reconciliation Flow

1. Fetch `PhaseMirrorCommand` CR
2. Compile CNL command → `CompilationResult` (MOCWord, c, R_sc, VerifiedAction)
3. If invariants fail: update CR status to `Failed` with diagnostic; emit Kubernetes Event
4. If invariants pass and consensus required: verify `UnifiedWitness` + Pell VDF + STARK proof
5. If consensus fails: update status to `Vetoed`
6. If consensus passes: execute `VerifiedAction` via typed Kubernetes API
7. Update CR status to `Succeeded` with execution receipt

## Key Design Decisions

- **Typed K8s API**: Use `k8s_openapi::api::apps::v1::Deployment` instead of `DynamicObject` for type safety.
- **Async CNL compilation**: Wrap synchronous CNL compiler in `tokio::task::spawn_blocking` to avoid blocking the reconciler.
- **Idempotency**: Revoke operations on already-revoked targets return safe no-op (`neutral Stratum`).
- **Finalizers**: Ensure cleanup on CR deletion.
- **Metrics**: Expose reconciliation latency, invariant violation counts, execution success/failure via Prometheus.

## Execution Mapping

| VerifiedAction | Kubernetes Resource |
|----------------|---------------------|
| `Deploy { service, target, replicas }` | `Deployment` + `Service` |
| `Scale { service, replicas }` | Patch `Deployment` replicas |
| `Destroy { service }` | Delete `Deployment` + `Service` |
| `Revoke { target_action_id }` | Delete resources matching `target_action_id` labels |

## Alternatives Considered

1. **Argo Workflows / GitOps**: Rejected because they lack native topological invariant enforcement and ALP gating.
2. **External REST bridge only**: Rejected because Kubernetes-native reconciliation provides better observability, RBAC, and declarative state.
3. **Operator in Go**: Rejected because the Phase Mirror core is Rust; kube-rs allows direct integration without FFI.

## Consequences

- **Positive**: Kubernetes becomes a first-class execution target for governed CNL commands. Full audit trail via CR status and Kubernetes Events.
- **Trade-off**: Adds Kubernetes cluster dependency. Operator must be deployed with appropriate RBAC.
- **Risk**: Misconfigured RBAC could allow privilege escalation. Mitigation: least-privilege ClusterRole with explicit `create`, `patch`, `delete` on `deployments` and `services` only.

## Implementation Evidence

Skeleton compiled with `cargo check`:
- `PhaseMirrorCommand` CRD with typed status subresource
- `Controller` with async reconciler
- `execute_action` using `kube::api::PostParams` and typed `Deployment` API

## Future Extensions

- Helm chart for operator installation
- Multi-namespace governance with per-namespace RBAC
- Webhook validation for `PhaseMirrorCommand` (admission control)
- Integration with Phase Mirror UI (`pirtm-ui`) for real-time status streaming
- Support for `Job`, `StatefulSet`, and custom resources

## References

- `Prime/crates/pirtm-apps/src/bin/cnl.rs` — CNL compiler prototype
- `packages/phase-mirror-gpt/ADR/009-cnl-compiler-over-pirtm.md` — CNL compiler ADR
- `Governance/adr/accepted/ALP CNL Compiler Discussion 2.md` — operator design discussion
- `Prime/substrates/the-commander/crates/commander-core/src/` — ConsensusVerifier, UnifiedWitness

---

*Prepared by the PhaseSpace Commander Coding Agent on 2026-07-02.*
<!-- LawfulRecursionVersion:1.0 -->
