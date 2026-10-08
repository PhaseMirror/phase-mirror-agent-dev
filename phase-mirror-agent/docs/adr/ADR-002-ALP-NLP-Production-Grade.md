# ADR-002: ALP-NLP Production Grade Completion

- Status: proposed
- Date: 2026-07-01
- Owners: PhaseSpace Commander Coding Agent
- Tags: alp, nlp, production, governance, schema-drift, archivum
- Phase: phase-1

## 1. Context

The ALP-NLP implementation in `packages/phase-mirror-agent` has 4 modules and 21 passing tests, but requires production-grade completion to meet Ξ-Constitution L0 invariants. Key gaps:

### 1.1 Schema Drift Risk
| Field | TypeScript (witness.ts) | Rust (multiplicity-common) | Status |
| :--- | :--- | :--- | :--- |
| `witness_id` | ✅ | ✅ | aligned |
| `action_id` | ✅ | ✅ | aligned |
| `timestamp` | ✅ | ✅ | aligned |
| `veto_status` | ✅ | ✅ | aligned |
| `execution_receipt` | ❌ (partial) | ✅ | **misaligned** |
| `violations` | ✅ | ❌ | **missing in Rust** |
| `contractivity_score` | ❌ | ✅ | **missing in TS** |
| `compliance_evidence` | ❌ | ✅ | **missing in TS** |
| `witness_hash` | ❌ | ✅ | **missing in TS** |
| `p_lineage` | ❌ | ✅ | **missing in TS** |

### 1.2 Missing Production Requirements
- ✅ Witness persistence to Archivum ledger (`state/archivum/witnesses.jsonl`)
- ✅ Semantic compiler integration tests added
- ✅ Duplicate `isPrime` removed (now imports from lexer.ts)
- ✅ ADR-001 accepted status
- JSON schema generation (pending)

## 2. Decision

### 2.1 Schema Alignment
Align TypeScript `UnifiedWitness` with Rust canonical definition:

```typescript
interface UnifiedWitness {
  witness_id: string;
  action_id: string;
  timestamp: string;
  compliance_evidence: string;       // NEW
  execution_receipt: {               // EXTEND
    status: string;
    prime_indices: number[];
    r_sc: number;
    c: number;
    contractivity_score: number;       // NEW
  };
  contractivity_score: number;         // NEW
  veto_status: 'admitted' | 'vetoed';
  witness_hash?: string;               // NEW (optional)
  p_lineage?: string;                  // NEW (optional)
  violations: string[];                // Keep for internal use
}
```

### 2.2 Witness Persistence
Integrate with Archivum via:
1. `src/alp-nlp/archivum.ts` - TypeScript witness writer (mirrors Rust `Archivum`)
2. Git auto-commit on witness write (per L0-5 invariant)

### 2.3 Governance Integration
Route all `analyze()` calls through the governed Rust ALP via FFI or JSON-RPC bridge.

## 3. Implementation Plan

| Phase | Task | File | Acceptance | Status |
| :-- | :-- | :-- | :-- | :-- |
| 1 | ✅ Align UnifiedWitness schema | `src/alp-nlp/witness.ts` | Fields match Rust definition | DONE |
| 2 | ✅ Remove duplicate isPrime | `src/alp-nlp/policy.ts` | Imports from lexer.ts | DONE |
| 3 | ✅ Add witness persistence | `src/alp-nlp/archivum.ts` | Writes to `state/archivum/` | DONE |
| 4 | ✅ Extend execution_receipt | `src/alp-nlp/policy.ts` | Adds contractivity_score | DONE |
| 5 | ✅ Add semantic compiler tests | `tests/alp-nlp/compiler.test.ts` | Integration tests coverage | DONE |
| 6 | ✅ Update ADR-001 to accepted | `ADR-001-*.md` | Status change | DONE |
| 7 | JSON schema generation | `scripts/export-schema.ts` | Schema exported | PENDING |

## 4. Consequences

- **Positive:** Zero schema drift, immutable audit trail, production certification ready
- **Negative:** Breaking change to witness format requires migration
- **Tradeoff:** Additional witness fields increase payload size for audit completeness

## 5. Security & Governance

Must satisfy L0 invariants:
1. `constitution.validate()` called before action admission
2. Every query produces valid `UnifiedWitness` with all required fields
3. Git ledger anchoring for Archivum entries
4. Non-bypassable ALP gate for all `analyze()` paths

## 6. References

- `packages/the-commander/crates/common/src/types.rs` — Rust canonical types
- `packages/the-commander/crates/commander-core/src/archivum.rs` — Witness persistence
- `Prime/crates/governance/src/constitution/mod.rs` — L0 invariant validation