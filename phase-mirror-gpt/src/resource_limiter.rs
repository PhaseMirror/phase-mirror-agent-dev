use crate::validator::{GovernanceOutcome, GovernanceTier};
use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct ResourceContext {
    pub memory_bytes: u64,
    pub concurrent_tasks: usize,
}

#[derive(Debug, Clone)]
pub struct ResourceLimiter {
    pub max_memory_mb: usize,
    pub max_concurrent_requests: usize,
    pub timeout_duration: Duration,
}

impl Default for ResourceLimiter {
    fn default() -> Self {
        Self {
            max_memory_mb: 512,
            max_concurrent_requests: 1024,
            timeout_duration: Duration::from_secs(30),
        }
    }
}

impl ResourceLimiter {
    #[inline(always)]
    pub fn check_invariants(
        &self,
        ctx: &ResourceContext,
        tier: GovernanceTier,
    ) -> GovernanceOutcome {
        if ctx.memory_bytes > (self.max_memory_mb as u64 * 1024 * 1024) {
            return match tier {
                GovernanceTier::Tier1Authoritative => {
                    GovernanceOutcome::Block("Resource limit exceeded: memory_bytes")
                }
                GovernanceTier::Tier2Experimental => {
                    GovernanceOutcome::Warning("Memory pressure advisory threshold")
                }
            };
        }
        if ctx.concurrent_tasks > self.max_concurrent_requests {
            return match tier {
                GovernanceTier::Tier1Authoritative => {
                    GovernanceOutcome::Block("Resource limit exceeded: concurrent_tasks")
                }
                GovernanceTier::Tier2Experimental => {
                    GovernanceOutcome::Warning("Concurrency pressure advisory threshold")
                }
            };
        }
        GovernanceOutcome::Allow
    }
}
