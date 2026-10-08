# ADR-005: Fail-Closed Governance

## Status
Accepted

## Context
When mathematical invariants (such as schema alignment, permissions, or lineage sync) are violated, or when required validation signatures are entirely missing (resulting in a 0.00% compliance audit rate), the system must react in a way that prioritizes integrity over availability or velocity.

## Decision
We mandate a strict **Fail-Closed** governance model for all Tier 1 (Authoritative) logic paths.
- If any internal validation or audit check fails (e.g., bitmask mismatch, broken hash chain), the system immediately defaults to a "Block" state.
- In the MCP Tool Transport layer, a failure is reported back to the LLM explicitly as a FAIL-CLOSED BLOCK with `isError: true`, preventing further execution down the AI workflow pipeline.
- Specifically, the compliance audit rate governs escalation absolutely. If the measured compliance is below 100%, the core Escalation Router halts dependent pipelines (e.g., GTM onboarding).

## Consequences
- Prevents unauthorized bypasses and structural contradictions from persisting into execution.
- Treats missing or draft configurations precisely the same as active security breaches (both trigger a hard block).
- Resolving the fail-closed state requires explicitly completing and binding the appropriate configuration templates/telemetry before automated processes can resume.
