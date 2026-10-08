use phase_mirror_gpt::archivum::ArchivumLedger;
use phase_mirror_gpt::domain_invariants::SemanticPolicy;
use phase_mirror_gpt::telemetry::LiveTelemetryOracle;
use phase_mirror_gpt::triple_lock::TripleLockSuite;
use phase_mirror_gpt::validator::{
    EvaluationContext, PERM_ADMIN, PERM_READ, PERM_WRITE, SCHEMA_NON_EMPTY, SCHEMA_VALID,
};
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as TokioMutex;

#[tokio::test]
async fn test_triple_lock_harness_scenarios() {
    println!("=== Initializing Triple-Lock Harness ===");
    let temp_dir = tempfile::tempdir().unwrap();
    let log_path = temp_dir.path().join("archivum_test.log");
    let mut ledger = ArchivumLedger::new();
    ledger.init_persistence(log_path.clone()).await.unwrap();
    let ledger_arc = Arc::new(TokioMutex::new(ledger));

    let telemetry = Arc::new(Mutex::new(LiveTelemetryOracle::init_baseline()));
    let policy = Arc::new(SemanticPolicy::new());
    policy
        .load_from_toml("[semantic_policy]\nforbidden_patterns = [\"public\", \"drop table\"]")
        .unwrap();

    let suite = TripleLockSuite::new(
        Arc::clone(&telemetry),
        Arc::clone(&ledger_arc),
        Arc::clone(&policy),
    );

    let valid_ctx = EvaluationContext {
        permission_bits: PERM_READ | PERM_WRITE | PERM_ADMIN,
        schema_signature: SCHEMA_VALID | SCHEMA_NON_EMPTY,
        expected_schema: SCHEMA_VALID | SCHEMA_NON_EMPTY,
    };

    println!("\n=== Scenario 1: Nominal Pass ===");
    let res_pass = suite
        .verify("M-001", "Initialize safe Rust kernel", valid_ctx)
        .await;
    assert!(res_pass.is_ok());
    let witness = res_pass.unwrap();
    println!("SUCCESS. Generated Witness: {:#?}", witness);
    assert_eq!(witness.governance_status, "VERIFIED");

    println!("\n=== Scenario 2: Semantic Block (L1) ===");
    let res_semantic = suite
        .verify("M-002", "Move data to public bucket", valid_ctx)
        .await;
    assert!(res_semantic.is_err());
    let err_msg = res_semantic.unwrap_err().to_string();
    println!("EXPECTED L1 BLOCK: {}", err_msg);
    assert!(err_msg.contains("Guardian Block - Semantic Violation"));

    println!("\n=== Scenario 3: Compliance Block (L0 - Telemetry) ===");
    {
        let mut t = telemetry.lock().unwrap();
        t.artifact_registry
            .get_mut("baa_agreement")
            .unwrap()
            .verification_state = 0b00;
    }
    let res_compliance = suite
        .verify("M-003", "Benign plan with bad telemetry", valid_ctx)
        .await;
    assert!(res_compliance.is_err());
    let l0_err = res_compliance.unwrap_err().to_string();
    println!("EXPECTED L0 TELEMETRY BLOCK: {}", l0_err);
    assert!(l0_err.contains("Compliance Floor Violation"));

    println!("\n=== Scenario 4: Structural Block (L0 - Bitmask) ===");
    // Re-verify telemetry first to bypass the compliance floor check
    {
        let mut t = telemetry.lock().unwrap();
        t.artifact_registry
            .get_mut("baa_agreement")
            .unwrap()
            .verification_state = 0b10;
    }
    let bad_ctx = EvaluationContext {
        permission_bits: PERM_READ, // missing WRITE and ADMIN
        schema_signature: SCHEMA_VALID,
        expected_schema: SCHEMA_VALID | SCHEMA_NON_EMPTY,
    };
    let res_structural = suite
        .verify("M-004", "Benign plan with missing schema bits", bad_ctx)
        .await;
    assert!(res_structural.is_err());
    let struct_err = res_structural.unwrap_err().to_string();
    println!("EXPECTED L0 STRUCTURAL BLOCK: {}", struct_err);
    assert!(struct_err.contains("Guardian Block - L0 Structural Violation: ADR-005"));

    println!("\n=== Scenario 5: Durable Recovery ===");
    let mut entries = 0;
    for _ in 0..50 {
        let mut new_ledger = ArchivumLedger::new();
        if new_ledger.init_persistence(log_path.clone()).await.is_ok() {
            entries = new_ledger.get_entry_count();
            if entries >= 3 {
                break;
            }
        }
        tokio::task::yield_now().await;
    }

    println!(
        "Recovered Ledger Entries (Including Block Receipts): {}",
        entries
    );
    // 1 Nominal pass, 1 L1 block, 1 L0 structural block = at least 3 entries
    assert!(entries >= 3);

    println!("\n=== All Scenarios Passed 100% ===");
}
