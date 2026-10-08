# ADR-002: L0 Bitmask Validation

## Status
Accepted

## Context
To ensure that invariant checks do not bottleneck the agent's hot path, validation must be incredibly fast. Relying on complex data structures, heap allocations, and heavy serialization/deserialization introduces unacceptable latency.

## Decision
We will implement an `L0Validator` kernel using zero-allocation bitmask validation. 
- Validation operations will target a **sub-100ns latency threshold**.
- We will utilize standard bitflags (`u32`) for checking schema structures and permission bits.
- We will use fixed-size stack allocations and compile-time evaluation (`const fn` and `#[inline(always)]`) where possible.

## Consequences
- The system achieves high-throughput validation.
- All structural schema errors and privilege escalation attempts are caught instantaneously.
- Requires strict adherence to bitwise operator conventions for defining rules and invariants instead of verbose string or JSON matching in the critical path.
