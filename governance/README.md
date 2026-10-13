# Governance & P2-P3 Signature Verification (Lever H2)

This directory documents the governance specification, key management, and detached signature verification architecture for the Phase Mirror repository, implementing the requirements specified in [`docs/P3 Signature.tex`](../docs/P3%20Signature.tex) and [`ADR_PM_AGENT_003`](../phase-mirror-gpt/ADR/003-archivum-lineage.tex) (Track H, Lever H2).

---

## 1. Governance Architecture

Phase Mirror enforces a strict separation of concerns between document governance and runtime execution:

### Act 1. Document Accept (Governance)
- **Scope**: Operates the **H2 lever** for Architectural Decision Record (ADR) acceptance.
- **Mechanism**: Two named persons, holding two distinct cryptographic keys (Ed25519), each sign the canonical SHA-256 hash of the document bytes.
- **Detached Signatures**: Signatures **must remain detached** (recorded in `governance/signatures.json` or companion `.signatures.json`). Signatures are **never** appended to the document body, as altering document bytes would invalidate the signed content digest.
- **State Machine**: The document status remains `Proposed` until both distinct signatures verify.
- **Metric**: `git verify <document>` (or `git verify` to audit all governed documents).

### Act 2. Runtime Unfreeze (Engine Execution) — SUSPENDED (DO NOT RUN)
- **Constraint**: No P_2/P_3 runtime key generation, no out-of-band handshake, no Sedona compile for a ProofHash, and no R3-FREEZE clear.
- **Condition**: Engine execution is suspended fail-closed until a public path names the verifier, the freeze flag, and the proof object. That path does not currently exist.
- **Invariant**: Document acceptance does not, and cannot, trigger an engine unfreeze.

---

## 2. Using `git verify`

The `git verify` command is integrated directly into git via `.git/config` and `/home/citizen/.local/bin/git-verify`.

### Verification Commands

```bash
# Verify all governed ADRs in the repository
git verify

# Verify a specific ADR document
git verify phase-mirror-gpt/ADR/003-archivum-lineage.tex

# Machine-readable JSON output (verdict format)
git verify --json phase-mirror-gpt/ADR/003-archivum-lineage.tex

# Strict mode: exits with non-zero code if any document remains 'Proposed'
git verify --strict

# Check Act 2 Runtime Unfreeze suspension invariants
git verify --check-unfreeze
```

### Signer Key Generation & Document Signing

```bash
# 1. Generate an Ed25519 governance keypair for a named key holder
# (Note: Strictly for governance document acceptance, NOT runtime unfreeze)
git verify --keygen p2_signer --signer "Alice (P2 Keyholder)"

# 2. Sign a document detached
git verify --sign phase-mirror-gpt/ADR/003-archivum-lineage.tex --key p2_signer.key --signer "Alice (P2 Keyholder)"

# 3. Sign with a second distinct keyholder
git verify --sign phase-mirror-gpt/ADR/003-archivum-lineage.tex --key p3_signer.key --signer "Bob (P3 Keyholder)"

# 4. Verify document acceptance under Lever H2
git verify phase-mirror-gpt/ADR/003-archivum-lineage.tex
```

---

## 3. Ledgers and Files

- `governance/signers.json`: Registry of recognized governance keyholders and public keys.
- `governance/signatures.json`: Central detached signatures ledger indexed by relative document path and SHA-256 digest.
- `scripts/git-verify`: Core verification engine.
- `scripts/test_git_verify.py`: Automated verification test suite.
