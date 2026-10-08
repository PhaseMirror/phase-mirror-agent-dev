# ADR-006: Triple-Lock Suite Mechanics

## Status
Accepted

## Context
As the Phase Mirror GPT transitions from a passive gatekeeper to an active cognitive oracle, we need a rigorous, multi-phase verification sequence to ensure that agentic outputs (plans) are not only structuraly valid but semantically safe and formally verified before execution. We need to eliminate the "Mirror's Blind Spot" where an agent might be its own judge without deterministic oversight.

## Decision
We will implement the **Triple-Lock Suite**, a three-phase deterministic verification pipeline:

1.  **Genius (The Architect)**:
    - **Input**: User Prompt / System State.
    - **Output**: `MirrorReflectionDraft` (The Plan).
    - **Metric**: Creativity and completeness.
    - **Lock**: Generative logic (LLM).

2.  **Guardian (The Sentinel)**:
    - **Input**: `MirrorReflectionDraft`.
    - **Output**: `SemanticApproval` or `FailClosedBlock`.
    - **Metric**: L1 Semantic Pattern Match (e.g., "public", "chmod 777") and L0 Bitmask Integrity.
    - **Lock**: Deterministic L0/L1 Validators (Rust Kernel).

3.  **Examiner (The Notary)**:
    - **Input**: `MirrorReflectionDraft` + `GuardianReceipt`.
    - **Output**: `TripleLockWitness` (Signed Hash).
    - **Metric**: p=7 Data Lineage continuity and Λ-Archivum recording.
    - **Lock**: Immutable Provenance (SHA-256 Chain).

## Sequence
`Genius` (Draft) -> `Guardian` (Enforce) -> `Examiner` (Certify).

## Output Format
The `triple_lock_verify` tool returns a JSON object:
```json
{
  "mission_id": "string",
  "witness_hash": "sha256_hash",
  "governance_status": "VERIFIED | BLOCKED",
  "p_lineage": "p=7:seq:N"
}
```

## Pass Criteria
1.  **Genius Lock**: Plan must be well-formed and hashable.
2.  **Guardian Lock**:
    - **L1**: Must NOT contain any forbidden semantic patterns from `policy.toml`.
    - **L0**: System compliance rate must be exactly `1.0`.
3.  **Examiner Lock**: 
    - Must successfully commit a receipt to the Λ-Archivum WAL.
    - Must generate a unique `witness_hash` linking the mission to the provenance chain.

Any single failure results in an immediate **FAIL-CLOSED BLOCK** with `isError: true`.
