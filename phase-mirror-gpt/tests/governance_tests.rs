use phase_mirror_gpt::validator::{
    EvaluationContext, GovernanceOutcome, GovernanceTier, InvariantConsistencyOracle, L0Validator,
    PERM_ADMIN, PERM_READ, PERM_WRITE, SCHEMA_NON_EMPTY, SCHEMA_VALID,
};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Barrier;

#[test]
fn test_l0_validator_isolated_edge_cases() {
    let oracle = InvariantConsistencyOracle;

    // 1. Nominal Allow Case
    let valid_ctx = EvaluationContext {
        permission_bits: PERM_READ | PERM_WRITE | PERM_ADMIN,
        schema_signature: SCHEMA_VALID | SCHEMA_NON_EMPTY,
        expected_schema: SCHEMA_VALID,
    };
    assert_eq!(
        oracle.validate_invariants(&valid_ctx, GovernanceTier::Tier1Authoritative),
        GovernanceOutcome::Allow
    );

    // 2. Fail-Closed Schema Violation (Tier 1 vs Tier 2)
    let bad_schema_ctx = EvaluationContext {
        permission_bits: PERM_READ,
        schema_signature: SCHEMA_VALID,
        expected_schema: SCHEMA_VALID | SCHEMA_NON_EMPTY, // missing non-empty bit
    };

    // Tier 1 must enforce a hard Block
    assert!(matches!(
        oracle.validate_invariants(&bad_schema_ctx, GovernanceTier::Tier1Authoritative),
        GovernanceOutcome::Block(_)
    ));

    // Tier 2 must downgrade to a Warning
    assert!(matches!(
        oracle.validate_invariants(&bad_schema_ctx, GovernanceTier::Tier2Experimental),
        GovernanceOutcome::Warning(_)
    ));

    // 3. Privilege Escalation Block (Write without Admin)
    let escalation_ctx = EvaluationContext {
        permission_bits: PERM_WRITE, // Has write but missing admin bit
        schema_signature: SCHEMA_VALID,
        expected_schema: SCHEMA_VALID,
    };
    assert!(matches!(
        oracle.validate_invariants(&escalation_ctx, GovernanceTier::Tier1Authoritative),
        GovernanceOutcome::Block(_)
    ));
}

#[tokio::test]
async fn test_high_concurrency_governance_load() {
    let oracle = Arc::new(InvariantConsistencyOracle);
    let num_tasks = 32;
    let iterations_per_task = 100_000;

    let barrier = Arc::new(Barrier::new(num_tasks));
    let mut handles = vec![];

    let shared_ctx = EvaluationContext {
        permission_bits: PERM_READ | PERM_WRITE | PERM_ADMIN,
        schema_signature: SCHEMA_VALID | SCHEMA_NON_EMPTY,
        expected_schema: SCHEMA_VALID | SCHEMA_NON_EMPTY,
    };

    let start_time = Instant::now();

    for _ in 0..num_tasks {
        let oracle_clone = Arc::clone(&oracle);
        let barrier_clone = Arc::clone(&barrier);
        let ctx = shared_ctx;

        let handle = tokio::spawn(async move {
            barrier_clone.wait().await;

            let mut local_count = 0;
            for _ in 0..iterations_per_task {
                let outcome = std::hint::black_box(oracle_clone.validate_invariants(
                    std::hint::black_box(&ctx),
                    std::hint::black_box(GovernanceTier::Tier1Authoritative),
                ));
                if outcome == GovernanceOutcome::Allow {
                    local_count += 1;
                }
            }
            local_count
        });
        handles.push(handle);
    }

    let mut total_allowed = 0;
    for handle in handles {
        total_allowed += handle.await.unwrap();
    }

    let duration = start_time.elapsed();
    let total_operations = num_tasks * iterations_per_task;
    let avg_ns_per_op = (duration.as_nanos() as f64) / (total_operations as f64);

    println!("\n=== High-Concurrency Performance Verification ===");
    println!("Total Concurrent Ops: {}", total_operations);
    println!("Total Execution Time: {:?}", duration);
    println!(
        "Average Latency per Invariant Check: {:.4} ns",
        avg_ns_per_op
    );

    assert_eq!(total_allowed, total_operations);
    // Performance gate: ensure sub-250ns execution (ADR-002 relaxed for CI)
    assert!(
        avg_ns_per_op < 250.0,
        "Performance drifted outside sub-250ns threshold!"
    );
}
