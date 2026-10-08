# ADR-IMPLEMENTATION-PLAN: ALP-Based NLP Replacement for Phase Mirror Agent

## 1. Overview

This implementation plan details the migration from LLM-based NLP to ALP-governed deterministic processing in `packages/phase-mirror-agent/`. The goal is to replace probabilistic inference with mathematically-bounded semantic compilation while maintaining the existing API surface.

## 2. Architectural Objectives

- **Deterministic Execution**: All NLP queries produce provable, repeatable outputs
- **Governance Compliance**: Every query passes through the ALP policy gate
- **Controlled Vocabulary**: Domain-specific terms map to Prime-Indexed Operators
- **Audit Trail**: All decisions generate verifiable witnesses for Archivum

## 3. Implementation Phases

### Phase 1: Core ALP NLP Modules (Days 1-10)

| Module | File | Purpose |
| :-- | :-- | :-- |
| Lexer | `src/alp-nlp/lexer.ts` | Tokenize and map controlled vocabulary |
| Parser | `src/alp-nlp/parser.ts` | Compile to MOCWord with R_sc and c metrics |
| Policy | `src/alp-nlp/policy.ts` | ALP gate validation against L0 invariants |

**Acceptance**: All modules compile and pass unit tests

### Phase 2: Agent Refactor (Days 11-14)

| File | Purpose |
| :-- | :-- |
| `src/index.ts` | Replace LLM pipeline with ALP Semantic Compiler |
| `tests/` | Verify API compatibility and deterministic outputs |

## 4. Technical Specifications

### 4.1 Controlled Vocabulary

```typescript
interface LexicalToken {
  word: string;           // English word
  prime_index: number;    // Prime-indexed operator Ap(p)
  semantic_role: 'noun' | 'verb' | 'adverb' | 'modifier';
  domain: string;         // devops, governance, archivum, mcp, nlp
}
```

### 4.2 Semantic Compiler Output

```typescript
interface MOCWord {
  tokens: LexicalToken[];
  prime_indices: number[];
  r_sc: number;           // Resonance tension
  c: number;              // Contraction bound
  is_coherent: boolean;   // Passes ALP invariants
}
```

### 4.3 ALP Gate Check

```typescript
// Validation passes if:
// - All prime_indices are prime (L0_04)
// - c < 1.0 (L0_02)
// - R_sc >= 1.0 (L0_03)  
// - Constitution passes all L0 checks
```

## 5. Security & Governance

- **Zero Drift Rule**: No LLM dependencies remain; all outputs deterministic
- **Non-Bypassability**: Every `analyze()` call must pass through ALP gate
- **Provenance Chain**: Every query generates verifiable output for Archivum

## 6. Completed Actions

- [x] Create unit tests for `lexer.ts` covering controlled vocabulary
- [x] Create unit tests for `parser.ts` validating R_sc and c calculations  
- [x] Create unit tests for `policy.ts` verifying witness generation
- [x] Remove `@huggingface/transformers` dependency from `package.json`
- [x] Update `package.json` for ALP-only build with vitest
- [x] Align UnifiedWitness schema with Rust canonical types
- [x] Add Archivum persistence (`src/alp-nlp/archivum.ts`)
- [x] Add semantic compiler integration tests (`tests/alp-nlp/compiler.test.ts`)
- [x] Remove duplicate isPrime implementation

## 7. Next Steps

1. **[Action]** Expand controlled vocabulary to cover 90% of Phase Mirror domain operations
2. **[Action]** Add witness_hash and p_lineage optional fields
3. **[Action]** Build and verify production bundle
4. **[Action]** Generate JSON schema export