# ADR-009: Controlled Natural Language Compiler over Phase Mirror / PIRTM

## Status
Proposed (2026-07-02)

## Context

Large Language Models (LLMs) are probabilistic, opaque, resource-intensive, and prone to hallucination. For high-stakes domains—DevOps, legal contracts, systems architecture—these properties are unacceptable. The Phase Mirror’s Atomic Language Policy (ALP) and PIRTM formalisms offer a deterministic, mathematically auditable alternative: tokenizing human language into Prime-Indexed Operators (`Ap(p)`) and validating sentences through geometric invariants (Contraction Bound `c < 1.0`, Resonance Tension `R_sc ≥ 1.0`).

The challenge is that natural language is inherently ambiguous. A full LLM replacement is infeasible. However, a **Controlled Natural Language (CNL)**—a restricted grammar over a domain-specific vocabulary—can be compiled into `MOCWord` ensembles and verified with absolute certainty.

## Decision

We will implement a domain-specific CNL compiler on top of `pirtm-apps`. Natural language utterances are parsed into `MOCWord` / `StratumBoundary` structures, validated by `PhaseMirrorInvariants`, and either executed or rejected with topological diagnostics.

We **explicitly reject** parsing arbitrary natural language or metaphors. Those are outside the grammar and will be rejected with `L0_08_PrimeOneForbidden` or equivalent invariant violations.

## Architecture

```
User Input → Tokenizer → Lexical Anchor Table → MOCWord Compiler → 
DialogueFrame (stack) → Invariant Enforcer → (pass/fail)
   ↓ pass → VerifiedAction → Execution Bridge
   ↓ fail → Suggestion Engine → User-facing correction
```

### Components

1. **Lexical Anchor Table**: Maps domain tokens to prime-indexed operators `Ap(p)`. Forbidden: `Ap(1)` (strictly expansive, used as the boundary of safe grammar).
2. **Semantic Compiler**: Compiles token sequences into `MOCWord` / `StratumBoundary` ensembles.
3. **Invariant Enforcer**: Runs `PhaseMirrorInvariants::enforce_all`. Only `c < 1.0` and `R_sc ≥ 1.0` are admitted.
4. **DialogueFrame**: Stack of `StratumWithMode` (`Assert`, `Retract`, `Query`). Supports multi-turn dialogue with anaphora resolution.
5. **Revoke Operator**: Typed retraction operator (e.g., `Ap(19)`) that composes with prior commands to yield a neutral `Stratum`, preserving invariant bounds without using `Ap(1)`.
6. **Topological Suggestion Engine**: Pre-computed adjacency graph of valid token sequences, weighted by prime-index distance and invariant drift. On failure, suggests nearest valid `MOCWord`.
7. **Execution Bridge**: Maps `VerifiedAction` variants to real-world side effects (Docker, Kubernetes) via `ActionExecutor` trait.

## Key Design Decisions

- **Negation via `Ap(1)` is forbidden**. Natural negation words ("not", "never") are rejected as topological contradictions. Retraction uses the structured `Revoke` operator instead.
- **Lexicon pre-verification**: Offline tool (`lexicon_verify.rs`) generates all valid permutations up to length L and cross-references against invariants. Lexical holes are flagged before deployment.
- **Deterministic auto-complete**: Suggestions are computed by topological edit distance over the pre-computed adjacency graph, not by statistical sampling.
- **Audit trail**: Every step (input, compilation, invariant check, execution, result) is committed to a signed append-only ledger (`Λ-Archivum`).

## Alternatives Considered

1. **Full LLM replacement**: Rejected due to ambiguity problem and computational cost.
2. **Heuristic guardrails on LLMs**: Rejected due to opacity and bypass risk.
3. **Hybrid LLM + ALP**: Deferred. Could be used for lexicon expansion, but not for runtime execution.

## Consequences

- **Positive**: Deterministic, auditable, CPU-efficient, hallucination-proof NLP for high-stakes domains.
- **Trade-off**: Restricted grammar requires user training. Domain expansion requires lexicon re-verification.
- **Risk**: Over-constraint may frustrate users. Mitigation: suggestion engine provides natural corrections.

## Implementation Evidence

Prototypes built and verified:
- `bin/lexicon_verify.rs` — offline lexicon pre-verification
- `cnl.rs` — multi-turn dialogue, retraction, and mock execution bridge
- `alp-cli` — ALP policy evaluation binary
- `agency-server/src/routes/alp.ts` — REST gateway for ALP

Live output traces:
- ✅ `deploy web-service on cluster with replicas 3` → `c=0.7014`, `R_sc=209.3009` → executed
- ❌ `you shall not pass` → `L0_08_PrimeOneForbidden` → suggestion engine proposes nearest valid tokens

## Future Extensions

- Real executor backends: `DockerExecutor` (bollard/CLI), `KubernetesExecutor` (kube-rs)
- WebSocket UI (`pirtm-ui`) for real-time invariant streaming
- Formal verification of the CNL compiler in Lean 4
- Multi-party audit ledger with consensus-gated execution
- Domain expansion: legal, medical, financial vocabularies
- WASM compilation for browser-native execution

## References

- `Prime/substrates/alp/src/lib.rs` — ALP PolicyEngine
- `Prime/crates/pirtm-apps/src/bin/cnl.rs` — CNL prototype
- `Prime/crates/pirtm-apps/src/bin/lexicon_verify.rs` — lexicon static analyzer
- `packages/phase-mirror-agency/agency-server/src/routes/alp.ts` — ALP REST route
- `Ξ-Constitution.md` — constitutional frame
- `Governance/adr/accepted/ALP CNL Compiler Discussion 1.md` — design discussion

---

*Prepared by the PhaseSpace Commander Coding Agent on 2026-07-02.*
<!-- LawfulRecursionVersion:1.0 -->
