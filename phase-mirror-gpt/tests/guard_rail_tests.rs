use phase_mirror_gpt::resource_limiter::{ResourceContext, ResourceLimiter};
use phase_mirror_gpt::validator::GovernanceTier;

#[test]
fn test_resource_limiter_nominal() {
    let limiter = ResourceLimiter::default();
    let ctx = ResourceContext {
        memory_bytes: 1024,
        concurrent_tasks: 1,
    };

    assert!(matches!(
        limiter.check_invariants(&ctx, GovernanceTier::Tier1Authoritative),
        phase_mirror_gpt::validator::GovernanceOutcome::Allow
    ));
}

#[test]
fn test_resource_limiter_memory_block() {
    let limiter = ResourceLimiter::default();
    let ctx = ResourceContext {
        memory_bytes: 1024 * 1024 * 1024,
        concurrent_tasks: 1,
    };

    assert!(matches!(
        limiter.check_invariants(&ctx, GovernanceTier::Tier1Authoritative),
        phase_mirror_gpt::validator::GovernanceOutcome::Block(_)
    ));

    let ctx = ResourceContext {
        memory_bytes: 1024 * 1024 * 1024,
        concurrent_tasks: 1,
    };
    assert!(matches!(
        limiter.check_invariants(&ctx, GovernanceTier::Tier2Experimental),
        phase_mirror_gpt::validator::GovernanceOutcome::Warning(_)
    ));
}

#[test]
fn test_resource_limiter_concurrency_block() {
    let limiter = ResourceLimiter::default();
    let ctx = ResourceContext {
        memory_bytes: 1,
        concurrent_tasks: 9999,
    };

    assert!(matches!(
        limiter.check_invariants(&ctx, GovernanceTier::Tier1Authoritative),
        phase_mirror_gpt::validator::GovernanceOutcome::Block(_)
    ));
}
