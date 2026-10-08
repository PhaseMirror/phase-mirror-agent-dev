# ADR-003: Λ-Archivum Provenance

## Status
Accepted

## Context
A critical requirement of the Phase Mirror architecture is deterministic provenance, maintaining a "p=7 Data Lineage." Every state transition, validation step, and agentic action must be trackable and tamper-evident without degrading system performance.

## Decision
We will integrate the **Λ-Archivum**, an immutable, tamper-evident ledger, to record every verification event.
- **Immutable Hash Chaining**: Every `VerificationEvent` includes a `prev_hash` to link it to the prior entry, creating a verifiable chain of custody (using SHA-256).
- **Zero-Surveillance Redaction**: We will use deterministic HMAC-style nonces to redact sensitive data at the tool boundary, ensuring the audit trail remains privacy-preserving.
- **Merkle-Delta Synchronization**: A `SyncManager` will compute tree deltas via a batch-oriented or asynchronous architecture (using deterministic key-value sorting in `BTreeMap`), keeping the primary runtime decoupled from network penalties.

## Consequences
- Every AI-generated work product is permanently bound to its verification audit trail.
- Distributed nodes can accurately detect schema or lineage drift efficiently (O(N) matching for deltas).
- Computation of cryptographic primitives (microseconds) is decoupled from the L0 validation hot path (<100ns) to prevent stalling.
