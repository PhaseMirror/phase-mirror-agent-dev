//! PM-LL-PIRTM-001 Stratification Witness
//!
//! Grounded witness executing the specification in PM-LL-PIRTM-001_stratification.md:
//! 1. Hands a LexLean declaration graph to PIRTM as atoms of a stratum.
//! 2. Receives a contractivity receipt with an exact rational norm (‖A·diag(λ)‖₁ = 1/6 ∈ ℚ).
//! 3. Stores it as an optional §34 supplementary field on an Entry.
//! 4. Verifies that stripping §34 leaves the §33 verdict and leaf bytes bit-identical.

use ed25519_dalek::Signer;
use lexlean_stratum::artifact::canonical_json::Json;
use lexlean_stratum::canonical::canonicalize;
use lexlean_stratum::contractivity::{
    Adjacency, Contraction, ContractivityReceipt, STATUS_ACCEPT,
};
use lexlean_stratum::entry::Entry;
use lexlean_stratum::ledger::Ledger;
use lexlean_stratum::pirtm::PirtmLayer;
use lexlean_stratum::rational::Rational;
use lexlean_stratum::signature::{KeyRecord, Signature};
use lexlean_stratum::stratum::Stratum;
use lexlean_stratum::verify::verify_entry;
use lexlean_stratum::zeno::ZenoFinton;

#[test]
fn test_stratification_witness_full_pipeline() {
    // Step 1: LexLean declaration graph
    let source = "def a : Nat := 1\ndef b : Nat := a\n";
    let toolchain = "leanprover/lean4:v4.32.1";

    let canonical = canonicalize(source).expect("canonicalization succeeds");
    let stratum = Stratum::of(&canonical).expect("stratum formation succeeds");

    assert_eq!(stratum.atom_count(), 2);
    assert_eq!(stratum.atoms[0].name, "a");
    assert_eq!(stratum.atoms[1].name, "b");

    // Extract reference adjacency matrix A and contraction factors λ
    let adjacency = Adjacency::of(&stratum.atoms);
    assert_eq!(adjacency.atom_count(), 2);
    assert_eq!(adjacency.row(0).unwrap(), &[0, 0]);
    assert_eq!(adjacency.row(1).unwrap(), &[1, 0]);

    let contraction = Contraction::of(&stratum.atoms).expect("contraction factors");
    let lambda_0 = contraction.of_atom(0).unwrap();
    let lambda_1 = contraction.of_atom(1).unwrap();
    assert_eq!(lambda_0.to_pair(), (1, 6));
    assert_eq!(lambda_1.to_pair(), (1, 6));

    // Step 2: Hand to PIRTM contractivity evaluation
    // G = A · diag(λ) = [[0, 0], [1/6, 0]]
    // Column 0 sum: 0 + 1 * 1/6 = 1/6
    // Column 1 sum: 0 + 0 = 0
    // ||G||_1 = max(1/6, 0) = 1/6 < 1
    let receipt = ContractivityReceipt::of(&stratum, "a").expect("contractivity receipt");
    assert_eq!(receipt.status, STATUS_ACCEPT);
    assert!(receipt.accepted());
    assert_eq!(receipt.norm.to_pair(), (1, 6));
    assert_eq!(receipt.norm, Rational::new(1, 6).unwrap());
    assert!(receipt.norm.is_below_one());

    // Verified PIRTM MCP counterpart anchor:
    // Calling `pirtm_verify_ensemble` with ensemble name "probe", theorem anchor "a",
    // adjacency [[0, 0], [1, 0]], and lambdas [1/6, 1/6] yields status ACCEPT,
    // exact_rational_norm_1: [1, 6], and seal_hash: "a7b91766d4d36cb411d83d41dc3d465b5a5fe04b2ae6b698c1e54ade57c595d7".
    // Note (SPEC.md §34.0): LexLean receipts use hash authority `lexlean-contractivity-v1`
    // because the compiler's seal_hash preimage is sovereign to PIRTM.
    const PIRTM_MCP_SEAL_HASH: &str =
        "a7b91766d4d36cb411d83d41dc3d465b5a5fe04b2ae6b698c1e54ade57c595d7";
    assert_eq!(PIRTM_MCP_SEAL_HASH.len(), 64);

    // Step 3: Construct the optional §34 layer
    let snapaddr = stratum.snapaddr();
    assert_eq!(
        snapaddr.to_hex(),
        "6b16a39b5e4751a8950d30e013c6bff7d1b0eaee91f8a8f49fee2d20ee546766",
        "SnapAddr must match the canonical stratum hash"
    );
    let prime_index = stratum.prime_index;
    assert_eq!(prime_index, 2); // least prime >= 2

    let zeno_signal = ZenoFinton::at(0).expect("valid Zeno-Finton signal");
    assert_eq!(zeno_signal.kappa().to_pair(), (1, 1)); // κ(0) = 1

    let pirtm_layer = PirtmLayer::identity(snapaddr, prime_index)
        .with_contractivity(receipt.clone())
        .with_zeno_finton(zeno_signal);

    // Step 4: Assemble base §33 Entry
    let mut unsigned_entry =
        Entry::new("Witness Snapshot", toolchain, &[("Snapshot.lean", source)]).expect("entry");

    // Sign the entry with an Ed25519 keypair
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&[42u8; 32]);
    let verifying_key = signing_key.verifying_key();
    let sig_bytes = signing_key.sign(&unsigned_entry.content_digest.0);
    unsigned_entry.signature = Signature {
        key: KeyRecord::software(),
        public_key: verifying_key.to_bytes(),
        value: sig_bytes.to_vec(),
    };

    // Append to an RFC 6962 ledger to obtain inclusion proof
    let tempdir = tempfile::tempdir().expect("tempdir");
    let mut ledger = Ledger::open(tempdir.path()).expect("open ledger");
    let log_key = [7u8; 32];
    let bare_entry = ledger
        .append(unsigned_entry, &log_key)
        .expect("ledger append");

    // Step 5: Attach the §34 layer as supplementary field
    let mut layered_entry = bare_entry.clone();
    layered_entry.with_pirtm(pirtm_layer);

    // Step 6: Verify the load-bearing stratification invariant (§34.4)
    // A. Leaf bytes are bit-for-bit identical
    assert_eq!(
        layered_entry.leaf_bytes(),
        bare_entry.leaf_bytes(),
        "§34.4: pirtm layer MUST be excluded from §33.5 leaf bytes"
    );

    // B. Content digests are bit-for-bit identical
    assert_eq!(
        layered_entry.content_digest.to_hex(),
        bare_entry.content_digest.to_hex(),
        "Content digest must be invariant to §34 attachment"
    );

    // C. The pirtm layer is present in JSON of layered, absent in bare
    let Json::Obj(layered_json) = layered_entry.to_json() else {
        panic!("must be obj");
    };
    let Json::Obj(bare_json) = bare_entry.to_json() else {
        panic!("must be obj");
    };
    assert!(layered_json.contains_key("pirtm"));
    assert!(!bare_json.contains_key("pirtm"));

    // D. Run §33 verification on both entries against the published ledger
    let verdict_layered =
        verify_entry(&layered_entry, Some(&ledger)).expect("layered entry verification");
    let verdict_bare = verify_entry(&bare_entry, Some(&ledger)).expect("bare entry verification");

    // E. Verify that all 5 §33 checks are bit-identical
    assert_eq!(verdict_layered.checks.len(), verdict_bare.checks.len());
    for (check_l, check_b) in verdict_layered
        .checks
        .iter()
        .zip(verdict_bare.checks.iter())
    {
        assert_eq!(check_l.step, check_b.step);
        assert_eq!(check_l.name, check_b.name);
        assert_eq!(check_l.passed, check_b.passed);
        assert_eq!(check_l.detail, check_b.detail);
        assert_eq!(check_l.code, check_b.code);
    }

    // F. Verdict statements are bit-identical
    assert_eq!(
        verdict_layered.statement(),
        verdict_bare.statement(),
        "Verdict statement must be bit-identical"
    );

    // G. Dropping §34 layer leaves valid §33 entry
    let mut stripped_entry = layered_entry.clone();
    stripped_entry.pirtm = None;
    assert_eq!(stripped_entry, bare_entry);

    let recomputed_digest = stripped_entry
        .recompute_digest()
        .expect("recompute digest succeeds");
    assert_eq!(recomputed_digest, bare_entry.content_digest);
}
