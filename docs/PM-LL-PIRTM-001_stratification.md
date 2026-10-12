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
