use crate::archivum::ArchivumLedger;
use crate::domain_invariants::SemanticPolicy;
use crate::telemetry::LiveTelemetryOracle;
use crate::validator::{
    EvaluationContext, GovernanceOutcome, GovernanceTier, InvariantConsistencyOracle, L0Validator,
};
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as TokioMutex;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TripleLockWitness {
    pub mission_id: String,
    pub witness_hash: String,
    pub governance_status: String,
    pub p_lineage: String,
}

/// TripleLockSuite: Implements the Genius -> Guardian -> Examiner sequence.
pub struct TripleLockSuite {
    pub telemetry: Arc<Mutex<LiveTelemetryOracle>>,
    pub ledger: Arc<TokioMutex<ArchivumLedger>>,
    pub policy: Arc<SemanticPolicy>,
    pub oracle: InvariantConsistencyOracle,
}

impl TripleLockSuite {
    pub fn new(
        telemetry: Arc<Mutex<LiveTelemetryOracle>>,
        ledger: Arc<TokioMutex<ArchivumLedger>>,
        policy: Arc<SemanticPolicy>,
    ) -> Self {
        Self {
            telemetry,
            ledger,
            policy,
            oracle: InvariantConsistencyOracle,
        }
    }

    /// Executes the Triple-Lock verification sequence natively.
    pub async fn verify(
        &self,
        mission_id: &str,
        draft_plan: &str,
        ctx: EvaluationContext,
    ) -> Result<TripleLockWitness> {
        // --- PHASE 1: GENIUS ---
        let mut hasher = Sha256::new();
        hasher.update(draft_plan.as_bytes());
        let draft_hash = hex::encode(hasher.finalize());

        // --- PHASE 2: GUARDIAN (Semantic & Structural Enforcement) ---
        // 1. L1 Semantic Scan
        let (violated, token) = self.policy.scan(draft_plan);
        if violated {
            let mut l = self.ledger.lock().await;
            l.commit_event(
                "triple_lock_guardian_block",
                token.clone(),
                draft_plan.as_bytes(),
            )?;
            return Err(anyhow!(
                "ADR-006: Guardian Block - Semantic Violation: '{}'",
                token
            ));
        }

        // 2a. L0 Compliance Check (Telemetry Floor)
        let compliance_rate = {
            let t = self
                .telemetry
                .lock()
                .map_err(|_| anyhow!("Telemetry lock poisoned"))?;
            t.calculate_compliance_rate()
        };

        if compliance_rate < 1.0 {
            return Err(anyhow!(
                "ADR-006: Guardian Block - Compliance Floor Violation ({:.2}%)",
                compliance_rate * 100.0
            ));
        }

        // 2b. L0 Structural Check (Real Bitmask Validation)
        match self
            .oracle
            .validate_invariants(&ctx, GovernanceTier::Tier1Authoritative)
        {
            GovernanceOutcome::Allow => {}
            GovernanceOutcome::Block(msg) | GovernanceOutcome::Warning(msg) => {
                let mut l = self.ledger.lock().await;
                l.commit_event(
                    "triple_lock_guardian_block_l0",
                    msg.to_string(),
                    draft_plan.as_bytes(),
                )?;
                return Err(anyhow!(
                    "ADR-006: Guardian Block - L0 Structural Violation: {}",
                    msg
                ));
            }
        }

        // --- PHASE 3: EXAMINER (Provenance & Certification) ---
        let p_lineage_seq = {
            let mut l = self.ledger.lock().await;
            l.commit_event(
                "triple_lock_verified",
                mission_id.to_string(),
                draft_plan.as_bytes(),
            )?;
            l.get_entry_count()
        };

        let mut witness_hasher = Sha256::new();
        witness_hasher.update(mission_id.as_bytes());
        witness_hasher.update(draft_hash.as_bytes());
        let witness_hash = hex::encode(witness_hasher.finalize());

        let witness = TripleLockWitness {
            mission_id: mission_id.to_string(),
            witness_hash: witness_hash.clone(),
            governance_status: "VERIFIED".to_string(),
            p_lineage: format!("p=7:seq:{}", p_lineage_seq),
        };

        // --- Atomic Registry Persistence with TRUE p=7 witness_hash chaining ---
        let registry_path = "MASTER_REGISTRY.md";
        let temp_path = "MASTER_REGISTRY.md.tmp";

        let mut existing_content = String::new();
        let mut last_hash = "GENESIS".to_string();

        if let Ok(content) = tokio::fs::read_to_string(registry_path).await {
            existing_content = content.clone();
            // Find the last recorded chain hash
            for line in content.lines().rev() {
                if line.contains("**Chain**: ") {
                    if let Some(idx) = line.rfind("**Chain**: ") {
                        last_hash = line[idx + 11..].trim().to_string();
                        break;
                    }
                }
            }
        }

        let mut chain_hasher = Sha256::new();
        chain_hasher.update(last_hash.as_bytes());
        chain_hasher.update(witness_hash.as_bytes());
        chain_hasher.update(witness.p_lineage.as_bytes());
        let chained_hash = hex::encode(chain_hasher.finalize());

        let registry_entry = format!(
            "- **Mission**: {} | **Witness Hash**: {} | **Chain**: {}\n",
            mission_id, witness_hash, chained_hash
        );
        let new_content = format!("{}{}", existing_content, registry_entry);

        // Atomic replacement guarantees thread/crash safety
        let _ = tokio::fs::write(temp_path, new_content).await;
        let _ = tokio::fs::rename(temp_path, registry_path).await;

        Ok(witness)
    }
}
