//! Conformance tests for LP-01 through LP-12 (SPEC.md §34.5).
//!
//! Validates the complete PIRTM stratification layer:
//! snapaddr, prime index, layout invariance, reference adjacency matrix,
//! contraction factors, exact rational norm, acceptance boundary, receipt hash,
//! theorem anchor gating, Zeno-Finton signaling, and additive stratification invariance.

use lexlean_stratum::artifact::canonical_json::Json;
use lexlean_stratum::artifact::content_id::Sha256Digest;
use lexlean_stratum::canonical::canonicalize;
use lexlean_stratum::contractivity::{
    norm, Adjacency, Contraction, ContractivityReceipt, Norm, RECEIPT_DOMAIN, STATUS_ACCEPT,
    STATUS_REJECT,
};
use lexlean_stratum::entry::Entry;
use lexlean_stratum::pirtm::PirtmLayer;
use lexlean_stratum::rational::Rational;
use lexlean_stratum::stratum::{is_prime, prime_index, Stratum};
use lexlean_stratum::verify::verify_entry;
use lexlean_stratum::zeno::{ZenoFinton, MAX_LEAF_INDEX};

fn stratum_of(source: &str) -> Stratum {
    let canonical = canonicalize(source).expect("the source canonicalizes");
    Stratum::of(&canonical).expect("the source is a stratum")
}

fn verdict_fingerprint(entry: &Entry) -> String {
    match verify_entry(entry, None) {
        Ok(verdict) => format!("ok:{}", verdict.statement()),
        Err(error) => {
            let codes: Vec<String> = error
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code.as_str().to_owned())
                .collect();
            format!("err:{}", codes.join(","))
        }
    }
}

/// LP-01: the snapaddr is the SHA-256 of the §34.1 frame encoding under
/// `lexlean-pirtm-v1` and is recomputable from the entry's own fields.
#[test]
fn test_lp_01_snapaddr() {
    let source = "def a : Nat := 1\ndef b : Nat := a\n";
    let stratum = stratum_of(source);
    let snapaddr = stratum.snapaddr();

    let recomputed = Sha256Digest::of(&stratum.canonical_form());
    assert_eq!(recomputed.to_hex(), snapaddr.to_hex());
    assert_eq!(
        snapaddr.to_hex().len(),
        64,
        "SHA-256 renders as 64 hex digits"
    );

    let mut entry = Entry::new(
        "probe",
        "leanprover/lean4:v4.32.1",
        &[("Probe.lean", source)],
    )
    .expect("the entry builds");
    entry.with_pirtm(PirtmLayer::identity(snapaddr, stratum.prime_index));
    let decoded = Entry::from_json(&entry.to_json()).expect("the entry round trips");
    let layer = decoded.pirtm.expect("the layer was recorded");
    assert_eq!(layer.snapaddr.to_hex(), snapaddr.to_hex());
    assert_eq!(layer.prime_index, stratum.prime_index);

    let rebuilt = stratum_of(source).snapaddr();
    assert_eq!(layer.snapaddr, rebuilt);
}

/// LP-02: the prime index is the least prime not below the atom count, is
/// itself prime, and a lexeme with no declarations is refused.
#[test]
fn test_lp_02_prime_index() {
    let source = "def a : Nat := 1\ndef b : Nat := 2\ndef c : Nat := 3\n";
    let stratum = stratum_of(source);
    assert_eq!(stratum.atom_count(), 3);
    assert_eq!(
        stratum.prime_index, 3,
        "3 atoms take the least prime not below 3"
    );
    assert!(is_prime(stratum.prime_index), "the index is itself prime");

    for count in 1..=400usize {
        let index = prime_index(count).expect("every atom count has a prime index");
        assert!(is_prime(index), "the index for {count} is not prime");
        assert!(index >= u64::try_from(count).expect("in range"));
        let mut candidate = u64::try_from(count).expect("in range");
        while candidate < index {
            assert!(
                !is_prime(candidate),
                "a prime at or above the count was skipped: {candidate} < {index}"
            );
            candidate += 1;
        }
    }

    let empty = canonicalize("").expect("the empty source canonicalizes");
    assert!(empty.declarations.is_empty());
    assert!(
        Stratum::of(&empty).is_err(),
        "an empty lexeme is not a stratum"
    );
    assert!(prime_index(0).is_err());
}

/// LP-03: two sources differing only in comments, layout, and declaration order
/// have one snapaddr, and changing any atom body changes it.
#[test]
fn test_lp_03_invariance() {
    let plain = "def a : Nat := 1\ndef b : Nat := 2\n";
    let noisy = "\
/- a block comment that mentions a and b -/
-- a line comment
def a : Nat := 1     -- trailing comment
                    -- a comment between declarations

def   b : Nat := 2
";
    let reordered = "def b : Nat := 2\ndef a : Nat := 1\n";
    let expected = stratum_of(plain).snapaddr();
    assert_eq!(stratum_of(noisy).snapaddr(), expected);
    assert_eq!(
        stratum_of(reordered).snapaddr(),
        expected,
        "declaration order is normalized by §33.1's sort"
    );

    for changed in [
        "def a : Nat := 2\ndef b : Nat := 2\n",
        "def a : Nat := 1 + 1\ndef b : Nat := 2\n",
        "def a : Nat := 1\ndef b : Nat := 3\n",
        "def a : Nat := 1\ndef b : Nat := 2\ndef c : Nat := 4\n",
    ] {
        assert_ne!(
            stratum_of(changed).snapaddr(),
            expected,
            "changing an atom body must change the snapaddr: {changed}"
        );
    }
}

/// LP-04: the reference adjacency matrix is the non-negative integer matrix of
/// whole-token references over each atom's body proper in canonical atom order,
/// and is recomputable from the canonical form.
#[test]
fn test_lp_04_adjacency() {
    let stratum = stratum_of(
        "def alpha : Nat := 1\ndef beta : Nat := alpha + alpha\ndef gamma : Nat := beta + gamma\n",
    );
    assert_eq!(
        stratum
            .atoms
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>(),
        vec!["alpha", "beta", "gamma"],
        "canonical order is ascending by fully qualified name"
    );
    let adjacency = Adjacency::of(&stratum.atoms);
    assert_eq!(adjacency.atom_count(), 3);
    assert_eq!(
        adjacency.row(0).expect("row"),
        &[0u64, 0, 0][..],
        "alpha mentions nothing"
    );
    assert_eq!(
        adjacency.row(1).expect("row"),
        &[2u64, 0, 0][..],
        "beta mentions alpha twice, through its body proper and not its head"
    );
    assert_eq!(
        adjacency.row(2).expect("row"),
        &[0u64, 1, 1][..],
        "gamma mentions beta and itself"
    );
    assert_eq!(adjacency.references_to(0).expect("column"), 2);
    assert_eq!(adjacency.references_to(1).expect("column"), 1);
    assert_eq!(adjacency.references_to(2).expect("column"), 1);

    assert_eq!(
        Adjacency::of(
            &stratum_of(
                "def alpha : Nat := 1\ndef beta : Nat := alpha + alpha\ndef gamma : Nat := beta + gamma\n"
            )
            .atoms
        ),
        adjacency
    );
    assert!(
        adjacency.row(9).is_err(),
        "a row outside the matrix is refused"
    );
    assert!(adjacency.references_to(9).is_err());

    let substrings = stratum_of("def alpha : Nat := 1\ndef beta : Nat := alphax\n");
    assert_eq!(
        Adjacency::of(&substrings.atoms)
            .references_to(0)
            .expect("column"),
        0
    );
}

/// LP-05: each contraction factor is the exact rational `1/(1+t)` for its atom's
/// body proper token count, with numerator one.
#[test]
fn test_lp_05_contraction() {
    let source = "def a : Nat := 1\ndef b : Nat := a + b + c\n";
    let stratum = stratum_of(source);
    let contraction = Contraction::of(&stratum.atoms).expect("factors");
    for (index, atom) in stratum.atoms.iter().enumerate() {
        let tokens = atom.body_proper().len() as i128;
        let factor = contraction.of_atom(index).expect("factor");
        assert_eq!(
            factor.to_pair(),
            (1, tokens + 1),
            "λ for {} is 1/(1+t) with t = {tokens}",
            atom.name
        );
        let (numerator, denominator) = factor.to_pair();
        assert_eq!(numerator, 1, "every factor has numerator one");
        assert!(denominator > 0);
        let recomposed = Rational::new(numerator, denominator).expect("valid");
        assert_eq!(recomposed.to_pair(), (numerator, denominator));
    }
}

/// LP-06: the norm is the maximum absolute column sum of `A·diag(λ)` computed in
/// exact rationals and reported as a reduced pair, and no float appears in a
/// receipt.
#[test]
fn test_lp_06_exact_rational_norm() {
    let stratum = stratum_of("def a : Nat := 1\ndef b : Nat := a\n");
    let Norm { value, column } = norm(&stratum).expect("a stratum has a norm");

    let adjacency = Adjacency::of(&stratum.atoms);
    let mut expected: Option<Rational> = None;
    for index in 0..stratum.atom_count() {
        let tokens = stratum.atoms[index].body_proper().len() as i128;
        let numerator = adjacency.references_to(index).expect("column") as i128;
        let candidate = Rational::new(numerator, tokens + 1).expect("valid");
        expected = Some(match expected {
            Some(best) if best > candidate => best,
            _ => candidate,
        });
    }
    let expected = expected.expect("at least one column");
    assert_eq!(
        value.to_pair(),
        expected.to_pair(),
        "the norm must be max over j of refs(j)/(1+t_j)"
    );

    let (numerator, denominator) = value.to_pair();
    assert!(denominator > 0);
    assert_eq!(
        Rational::new(numerator, denominator)
            .expect("valid")
            .to_pair(),
        (numerator, denominator),
        "the reported pair is already reduced"
    );
    let receipt = ContractivityReceipt::of(&stratum, "a").expect("valid");
    let Json::Obj(fields) = receipt.to_json() else {
        panic!("a receipt is a JSON object")
    };
    for key in ["norm", "norm_numerator", "norm_denominator"] {
        assert!(
            matches!(fields.get(key), Some(Json::Int(_) | Json::Arr(_))),
            "`{key}` must be exact integers, not a float"
        );
    }
    assert_eq!(receipt.column, column);
}

/// LP-07: a receipt is accepted exactly when the norm is below one, a norm of
/// exactly one is refused, and a refusal states the exact value.
#[test]
fn test_lp_07_acceptance_boundary() {
    let stratum = stratum_of("def a : Nat := 1\ndef b : Nat := a\n");
    let receipt = ContractivityReceipt::of(&stratum, "a").expect("valid");
    assert_eq!(
        receipt.accepted(),
        receipt.norm.is_below_one(),
        "acceptance is exactly the norm being below one"
    );
    assert_eq!(receipt.status, STATUS_ACCEPT);
    assert!(
        receipt.rejection().is_none(),
        "an accepted receipt states no reason"
    );

    let boundary = Rational::new(1, 1).expect("valid");
    assert!(!boundary.is_below_one(), "one is not below one");
    let refused = ContractivityReceipt {
        prime_index: stratum.prime_index,
        atom_count: stratum.atom_count(),
        anchor: "a".to_owned(),
        norm: boundary,
        column: 0,
        status: STATUS_REJECT.to_owned(),
    };
    assert!(!refused.accepted());
    assert_eq!(
        refused.rejection(),
        Some("NormContractivityViolation: ||G||_1 = 1/1 >= 1".to_owned())
    );

    let three_halves = ContractivityReceipt {
        norm: Rational::new(3, 2).expect("valid"),
        status: STATUS_REJECT.to_owned(),
        ..refused.clone()
    };
    assert_eq!(
        three_halves.rejection(),
        Some("NormContractivityViolation: ||G||_1 = 3/2 >= 1".to_owned()),
        "the exact value is stated as a reduced pair"
    );
}

/// LP-08: the receipt hash is LexLean's own domain-separated hash over the named
/// frames, and an entry states that it is not the PIRTM `seal_hash`.
#[test]
fn test_lp_08_receipt_hash() {
    let stratum = stratum_of("def a : Nat := 1\ndef b : Nat := a\n");
    let receipt = ContractivityReceipt::of(&stratum, "a").expect("valid");
    let Json::Obj(fields) = receipt.to_json() else {
        panic!("a receipt is a JSON object")
    };

    let recomputed = Sha256Digest::of(&receipt.canonical_form());
    assert_eq!(recomputed.to_hex(), receipt.hash().to_hex());
    assert_ne!(
        receipt.hash().to_hex(),
        Sha256Digest::of(receipt.to_json().to_canonical_string().as_bytes()).to_hex(),
        "the hash is over the named frames, not over the JSON that reports it"
    );

    assert_eq!(
        fields.get("hash_authority"),
        Some(&Json::Str(RECEIPT_DOMAIN.to_owned()))
    );
    assert_eq!(ContractivityReceipt::HASH_AUTHORITY, RECEIPT_DOMAIN);

    let canonical = String::from_utf8(receipt.canonical_form()).expect("frames are UTF-8");
    for frame in [
        "prime-index",
        "atom-count",
        "anchor",
        "norm-numerator",
        "norm-denominator",
        "status",
    ] {
        assert!(
            canonical.contains(frame),
            "the `{}` frame must be hashed",
            frame
        );
    }
    let other = ContractivityReceipt::of(&stratum, "b").expect("valid");
    assert_ne!(
        other.hash(),
        receipt.hash(),
        "the anchor is one of the frames"
    );
}

/// LP-09: the theorem anchor must name an atom of the stratum, so an anchor
/// naming an absent declaration is refused.
#[test]
fn test_lp_09_theorem_anchor() {
    let stratum = stratum_of("def alpha : Nat := 1\ndef beta : Nat := 2\n");
    assert!(ContractivityReceipt::of(&stratum, "alpha").is_ok());
    assert!(ContractivityReceipt::of(&stratum, "beta").is_ok());

    for absent in ["gamma", "", "alpha.extra", "Alpha"] {
        let error = ContractivityReceipt::of(&stratum, absent)
            .err()
            .unwrap_or_else(|| panic!("the absent anchor `{absent}` must be refused"));
        assert!(error.contains("names no atom"), "`{absent}`: {error}");
    }

    let nested = stratum_of("namespace N\ndef inner : Nat := 1\nend N\n");
    assert_eq!(nested.atoms[0].name, "N.inner");
    assert!(
        ContractivityReceipt::of(&nested, "N.inner").is_ok(),
        "the fully qualified name is an atom"
    );
}

/// LP-10: the Zeno-Finton gain is the exact rational `2^(-index)`, strictly
/// decreasing, positive at every finite index, and never zero.
#[test]
fn test_lp_10_zeno_finton_gain() {
    for index in 0..=MAX_LEAF_INDEX {
        let signal = ZenoFinton::at(index).expect("exact");
        assert_eq!(
            signal.kappa().to_pair(),
            (1, 1i128 << index),
            "κ({index}) = 2^(-{index})"
        );
    }
    assert!(!ZenoFinton::at(0).expect("valid").kappa().is_below_one());

    let mut previous = ZenoFinton::at(0).expect("valid");
    for index in 1..=MAX_LEAF_INDEX {
        let current = ZenoFinton::at(index).expect("valid");
        assert!(
            current.is_below(previous),
            "κ({index}) must be strictly below κ({})",
            index - 1
        );
        assert!(
            current.kappa() > Rational::new(0, 1).expect("valid"),
            "κ({index}) must stay positive"
        );
        previous = current;
    }

    assert!(ZenoFinton::at(MAX_LEAF_INDEX + 1).is_err());
}

/// LP-11: the Zeno-Finton signal never satisfies a §33.4 check, never appears in
/// the §33.6 verdict, and an entry carrying only it is untimestamped.
#[test]
fn test_lp_11_zeno_finton_supplementary() {
    let source = "def a : Nat := 1\n";
    let mut entry = Entry::new(
        "probe",
        "leanprover/lean4:v4.32.1",
        &[("Probe.lean", source)],
    )
    .expect("the entry builds");
    assert!(
        entry.timestamp.is_none(),
        "no timestamp was obtained, so none is recorded"
    );
    let stratum = stratum_of(source);
    entry.with_pirtm(
        PirtmLayer::identity(stratum.snapaddr(), stratum.prime_index)
            .with_zeno_finton(ZenoFinton::at(9).expect("valid")),
    );

    let decoded = Entry::from_json(&entry.to_json()).expect("the entry round trips");
    assert_eq!(
        decoded
            .pirtm
            .as_ref()
            .and_then(|layer| layer.zeno_finton)
            .expect("the signal was recorded")
            .leaf_index(),
        9
    );

    let Json::Obj(fields) = decoded.to_json() else {
        panic!("an entry is a JSON object")
    };
    assert!(
        !fields.contains_key("timestamp"),
        "a Zeno-Finton signal must not produce a timestamp field"
    );

    let without_signal = {
        let mut bare = Entry::new(
            "probe",
            "leanprover/lean4:v4.32.1",
            &[("Probe.lean", source)],
        )
        .expect("the entry builds");
        bare.with_pirtm(PirtmLayer::identity(
            stratum.snapaddr(),
            stratum.prime_index,
        ));
        verdict_fingerprint(&bare)
    };
    let with_signal = verdict_fingerprint(&decoded);
    assert_eq!(
        with_signal, without_signal,
        "§34.3: the signal must not move the §33.6 verdict"
    );
    let statement = with_signal.to_lowercase();
    for forbidden in ["zeno", "finton", "kappa", "2^-", "2^"] {
        assert!(
            !statement.contains(forbidden),
            "the §33.6 verdict must not mention `{forbidden}`: {with_signal}"
        );
    }
}

/// LP-12: dropping every §34 field leaves every §33 check and the §33.6 verdict
/// bit-identical, and an entry omitting the layer is a valid §33 entry.
#[test]
fn test_lp_12_stratification_additive() {
    let source = "def a : Nat := 1\ndef b : Nat := a\n";
    let toolchain = "leanprover/lean4:v4.32.1";

    let bare = Entry::new("probe", toolchain, &[("Probe.lean", source)]).expect("builds");

    let stratum = stratum_of(source);
    let receipt = ContractivityReceipt::of(&stratum, "a").expect("valid");
    let mut layered = bare.clone();
    layered.with_pirtm(
        PirtmLayer::identity(stratum.snapaddr(), stratum.prime_index)
            .with_contractivity(receipt)
            .with_zeno_finton(ZenoFinton::at(3).expect("valid")),
    );

    assert_eq!(
        layered.leaf_bytes(),
        bare.leaf_bytes(),
        "§34.4: the §34 layer must not reach the §33.5 leaf bytes"
    );
    assert_eq!(
        layered.content_digest.to_hex(),
        bare.content_digest.to_hex(),
        "the content digest is a §33 field and cannot move"
    );

    let Json::Obj(layered_fields) = layered.to_json() else {
        panic!("an entry is a JSON object")
    };
    let Json::Obj(bare_fields) = bare.to_json() else {
        panic!("an entry is a JSON object")
    };
    assert!(
        layered_fields.contains_key("pirtm"),
        "the layer is recorded"
    );
    assert!(
        !bare_fields.contains_key("pirtm"),
        "the bare entry has none"
    );
    let differing: Vec<&String> = layered_fields
        .keys()
        .chain(bare_fields.keys())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .filter(|key| layered_fields.get(*key) != bare_fields.get(*key))
        .collect();
    assert_eq!(
        differing,
        vec!["pirtm"],
        "the layer must be the only field that differs"
    );

    assert_eq!(verdict_fingerprint(&layered), verdict_fingerprint(&bare));

    let decoded = Entry::from_json(&bare.to_json()).expect("a §33 entry with no layer is valid");
    assert!(decoded.pirtm.is_none());
    assert_eq!(
        decoded.content_digest.to_hex(),
        bare.content_digest.to_hex()
    );
}
