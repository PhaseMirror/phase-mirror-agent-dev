# PM-LL-PIRTM-001 — PIRTM and LexLean, stratification not substitution

Status: receipt, not a new contract.
Date: 2026-10-12

## Three languages, one joint

LexLean's authoring language is a closed-lexicon CNL. Every word, symbol, and control is declared by a versioned lexicon package, one typed IR emits canonical LaTeX and prose-free Lean, and `SPEC.md` is the contract. It is not ALP.

PIRTM/MOC is the L0 contractive kernel. The predicate LexLean checked against the observed compiler is `‖A·diag(λ)‖₁ < 1` in exact rationals. The compiler does not factor a term into prime-irreducible components, and `prime=` is a stratum selector it does not even require to be prime.

`pirtm-apps` CNL compilation is a third thing. On `phase-mirror-agent-dev` it is a closed refusal: the crate is outside the workspace, and the agent does not become that compiler.

## What SPEC.md already decided

§34 is the joint. It models a lexeme's declarations as atoms of a stratum, reproduces the observed contractivity predicate, and carries the norm as an exact `[numerator, denominator]` pair.

§34.3 records a Zeno-Finton signal as `κ(index) = 2^(-index)`. It carries no legal weight. The RFC 3161 timestamp remains the only time anchor, because a timestamp is meaningful only when it comes from an authority the author does not control.

§34.4 is the load-bearing sentence: dropping every field this section defines must leave every §33 check and the §33.6 verdict bit-identical. An entry that omits the layer is a valid §33 entry. That is stratification. Substitution would break it first.

The receipt hash is a LexLean hash under `lexlean-contractivity-v1`. It is not the compiler's `seal_hash`, and an entry says which of the two it carries.

## What not to explore

`0000-PIRTM LEXLEAN.md` proposes replacing the SHA-256 identity, the RFC 6962 inclusion proof, the browser transcription, and the FreeTSA timestamp with PIRTM primitives. §34 refuses the last of those outright and does not claim the first three. Prime-indexed identity, a contractivity receipt in place of an inclusion proof, and an internal temporal anchor are literature claims the observed compiler does not compute.

## The next witness, if one is wanted

Hand a LexLean snapshot's declaration graph to PIRTM as atoms of a stratum. Take back a receipt whose norm is an exact rational. Store it as a §34 supplementary field. Re-run §33 and require the verdict to be bit-identical with the field removed.

That is the integration. A merged ALP/CNL compiler, or a PIRTM binary inside LexLean, is a different project and a forked reading of §34.4.

## Executed Witness (2026-10-12)

The witness described above is executed and physically in-tree:
- **Crate**: `crates/lexlean-stratum`
- **Witness Test**: `tests/stratification_witness.rs`
- **Conformance Suite**: `tests/conformance_lp.rs` (LP-01 through LP-12 passing)
- **Toolchain Invariant**: Lean 4 `leanprover/lean4:v4.32.1`, LexLean version `1.0`

### Witness Metrics

1. **Snapshot Stratum**:
   - Source: `def a : Nat := 1\ndef b : Nat := a\n`
   - Atoms: `a` ($t_a = 5$), `b` ($t_b = 5$)
   - Contraction factors: $\lambda_a = 1/(1+5) = 1/6$, $\lambda_b = 1/(1+5) = 1/6$
   - Adjacency matrix:
     $$A = \begin{pmatrix} 0 & 0 \\ 1 & 0 \end{pmatrix}$$

2. **PIRTM Contractivity**:
   - Operator coupling: $G = A \cdot \mathrm{diag}(\lambda) = \begin{pmatrix} 0 & 0 \\ 1/6 & 0 \end{pmatrix}$
   - Rational 1-norm: $\|G\|_1 = \max(1/6, 0) = 1/6 < 1$
   - Rational representation: exact pair $[1, 6] \in \mathbb{Q}$ (zero floating-point values)
   - Evaluated via live MCP tool `pirtm_verify_ensemble`:
     - `status`: `ACCEPT`
     - `is_norm_contractive`: `true`
     - `exact_rational_norm_1`: `[1, 6]`
     - `seal_hash`: `a7b91766d4d36cb411d83d41dc3d465b5a5fe04b2ae6b698c1e54ade57c595d7`

3. **§34 Supplementary Layer**:
   - `snapaddr`: `6b16a39b5e4751a8950d30e013c6bff7d1b0eaee91f8a8f49fee2d20ee546766`
   - `prime_index`: `2` (least prime $\ge 2$)
   - `contractivity`: `ContractivityReceipt` (domain: `lexlean-contractivity-v1`, norm: $[1, 6]$, status: `ACCEPT`)
   - `zeno_finton`: `ZenoFinton::at(0)` ($\kappa(0) = 1$)

4. **§34.4 Additive Stratification Invariance**:
   - Base entry signed with Ed25519 and committed to Merkle ledger.
   - `layered.leaf_bytes() == bare.leaf_bytes()` verified bit-identical.
   - `verify_entry(&layered, &ledger) == verify_entry(&bare, &ledger)` verified bit-identical across all 5 checks and final verdict statement.
   - Detaching `entry.pirtm = None` produces the exact bit-for-bit unstratified base entry.
