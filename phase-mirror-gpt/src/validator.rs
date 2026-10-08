/// Bitflags representing L0 Permission Bits (ADR-002)
pub const PERM_READ: u32 = 1 << 0;
pub const PERM_WRITE: u32 = 1 << 1;
pub const PERM_EXECUTE: u32 = 1 << 2;
pub const PERM_ADMIN: u32 = 1 << 3;

/// Bitflags representing L0 Schema Integrity signatures
pub const SCHEMA_VALID: u32 = 1 << 0;
pub const SCHEMA_NON_EMPTY: u32 = 1 << 1;
pub const SCHEMA_NO_CYCLES: u32 = 1 << 2;

/// Governance Tier Classifications (ADR-001)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GovernanceTier {
    Tier1Authoritative,
    Tier2Experimental,
}

/// Strict enforcement outcomes matching ADR-005 (Fail-Closed)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GovernanceOutcome {
    Allow,
    Warning(&'static str),
    Block(&'static str),
}

/// The state signature evaluated by the L0 Oracle Kernel
#[derive(Debug, Clone, Copy)]
pub struct EvaluationContext {
    pub permission_bits: u32,
    pub schema_signature: u32,
    pub expected_schema: u32,
}

/// Operational Command Context (Events mapped from the CLI)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OracleEvent {
    PullRequest,
    MergeGroup,
    Drift,
}

/// L0Validator: Built for sub-100ns performance-critical invariant checks (ADR-002)
pub trait L0Validator {
    fn validate_invariants(
        &self,
        ctx: &EvaluationContext,
        tier: GovernanceTier,
    ) -> GovernanceOutcome;
}

pub struct InvariantConsistencyOracle;

impl L0Validator for InvariantConsistencyOracle {
    #[inline(always)]
    fn validate_invariants(
        &self,
        ctx: &EvaluationContext,
        tier: GovernanceTier,
    ) -> GovernanceOutcome {
        // 1. Bitwise Schema Integrity Check
        if (ctx.schema_signature & ctx.expected_schema) != ctx.expected_schema {
            return match tier {
                GovernanceTier::Tier1Authoritative => {
                    GovernanceOutcome::Block("ADR-005: Critical Schema Violation")
                }
                GovernanceTier::Tier2Experimental => {
                    GovernanceOutcome::Warning("Experimental Schema Mismatch detected")
                }
            };
        }

        // 2. Permission Bit Integrity Check (e.g., preventing write escalation without admin bits)
        if (ctx.permission_bits & PERM_WRITE == PERM_WRITE)
            && (ctx.permission_bits & PERM_ADMIN != PERM_ADMIN)
        {
            return match tier {
                GovernanceTier::Tier1Authoritative => {
                    GovernanceOutcome::Block("ADR-005: Privilege Escalation Attempt")
                }
                GovernanceTier::Tier2Experimental => {
                    GovernanceOutcome::Warning("Unprivileged write pattern observed")
                }
            };
        }

        GovernanceOutcome::Allow
    }
}

impl InvariantConsistencyOracle {
    /// Dispatches validation based on the specific operational command/event
    #[inline]
    pub fn handle_event(
        &self,
        event: OracleEvent,
        ctx: &EvaluationContext,
        tier: GovernanceTier,
    ) -> GovernanceOutcome {
        match event {
            OracleEvent::PullRequest | OracleEvent::MergeGroup => {
                self.validate_invariants(ctx, tier)
            }
            OracleEvent::Drift => self.validate_invariants(ctx, tier),
        }
    }
}
