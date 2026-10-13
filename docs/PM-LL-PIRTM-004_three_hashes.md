# PM-LL-PIRTM-004 — The Three Hashes: Witness, Address, and External Seal

Status: architectural demarcation.
Date: 2026-10-12
Published anchor: PhaseMirror/phase-mirror-agent-dev `48a1752d08206a0baff1d22e33834309fd85e5b5`

## Boundary Statement

The stratification witness (`crates/lexlean-stratum/tests/stratification_witness.rs`) binds a LexLean receipt to its own domain. It does not bind, recompute, or verify a PIRTM seal.

## What the Witness Binds

At `48a1752` the witness executes:

```rust
assert_eq!(ContractivityReceipt::HASH_AUTHORITY, "lexlean-contractivity-v1");
assert_eq!(
    receipt_fields.get("hash_authority"),
    Some(&Json::Str("lexlean-contractivity-v1".to_string()))
);
```

`HASH_AUTHORITY` is a constant on the receipt. The second check guarantees that the canonical JSON carried by the entry names that authority. That is the whole binding: it names the authority of the hash. It does not hash anything against a PIRTM value.

The receipt hash is SHA-256 of a §21.1 frame encoding under domain `lexlean-contractivity-v1`, over six frames:
- `prime-index`
- `atom-count`
- `anchor`
- `norm-numerator`
- `norm-denominator`
- `status`

For the fixture `def a : Nat := 1\ndef b : Nat := a\n`, that is prime `2`, two atoms, anchor `a`, norm `1/6`, status `ACCEPT`. SPEC.md §34.2 and `LP-08` require the entry to state that this is not the compiler's `seal_hash`.

SnapAddr is a second LexLean identity under a distinct domain, `lexlean-pirtm-v1`. Its frames are the canonicalization identifier, prime index, atom count, and each atom's name and body. For the fixture, the snapaddr is `6b16a39b…`. It is an address of the stratum, not a seal of a contractivity receipt.

## What the Seal Is, and What It Is Not

SPEC.md §34.0 item 3 is the governing invariant: the compiler's `seal_hash` preimage is not recoverable from the binary that was probed. A LexLean receipt does not reproduce it and does not claim to. The entry carries `hash_authority` so that nobody looks a LexLean receipt up in a PIRTM ledger. Computing that seal inside `lexlean-stratum` would be the substitution §34.4 forbids.

The public `SNAPKITTYWEST/sovereign-pirtm` tree is a distinct artifact and not the preimage of the MCP seal. Its `ReceiptGenerator` builds a SHA-256 over a string including operator name, prime, `now_ns()`, and an incrementing counter. A hash that incorporates wall-clock time and a local counter is not reproducible. Its `verify` checks the prime, operator name, and that the hash length is 64 characters. The README's `seal(input, output)` and JSON fields `PIRTM-CONTRACTIVITY-1` / `sha256-merkle-v1` do not match that C++.

## The Three Hashes

| Hash | Domain | Recomputable from the lexeme | What it certifies |
|---|---|---|---|
| LexLean receipt | `lexlean-contractivity-v1` | Yes | $\|G\|_1$ exact rational norm of the declaration graph |
| SnapAddr | `lexlean-pirtm-v1` | Yes | Identity of the stratum |
| PIRTM `seal_hash` | Not published for the probed binary | No | Whatever that binary sealed; not claimed here |

## Process-Boundary Quarantine

A process-boundary call to `pirtm_verify_ensemble` can store the returned seal as external evidence, labeled as PIRTM's (Option 3 pattern). It cannot become the LexLean receipt, and the crate cannot grow a function that pretends to produce it.
