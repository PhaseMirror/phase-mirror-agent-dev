use crate::archivum::ArchivumLedger;
use crate::domain_invariants::SemanticPolicy;
use crate::telemetry::LiveTelemetryOracle;
use anyhow::{Result, anyhow};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as TokioMutex;

/// MirrorCoordinator: Orchestrates the Dual-Agent Reflection pipeline (Phase/Mirror).
pub struct MirrorCoordinator {
    pub telemetry: Arc<Mutex<LiveTelemetryOracle>>,
    pub ledger: Arc<TokioMutex<ArchivumLedger>>,
    pub policy: Arc<SemanticPolicy>,
}

impl MirrorCoordinator {
    pub fn new(
        telemetry: Arc<Mutex<LiveTelemetryOracle>>,
        ledger: Arc<TokioMutex<ArchivumLedger>>,
        policy: Arc<SemanticPolicy>,
    ) -> Self {
        Self {
            telemetry,
            ledger,
            policy,
        }
    }

    /// Reflects on a draft plan, cryptographically linking it to the provenance chain.
    pub async fn reflect(&self, draft_plan: &str) -> Result<String> {
        // 1. Check current governance floor (ADR-005)
        let (status, blocked) = {
            let t = self
                .telemetry
                .lock()
                .map_err(|_| anyhow!("Telemetry lock poisoned"))?;
            t.determine_escalation_vector()
        };

        if blocked {
            return Err(anyhow!(
                "ADR-005: Reflection blocked due to compliance failure: {}",
                status
            ));
        }

        // 2. Autonomous Semantic Pre-Screening (L1 Domain Invariants)
        let (violated, token) = self.policy.scan(draft_plan);
        if violated {
            {
                let mut l = self.ledger.lock().await;
                l.commit_event(
                    "mirror_semantic_violation",
                    token.clone(),
                    draft_plan.as_bytes(),
                )?;
            }
            return Err(anyhow!(
                "ADR-005: Semantic Policy Violation detected: '{}'",
                token
            ));
        }

        // 3. Hash the draft for immutable linking (p=7 Lineage)
        let mut hasher = Sha256::new();
        hasher.update(draft_plan.as_bytes());
        let draft_hash = hex::encode(hasher.finalize());

        // 4. Log the reflection attempt to the Λ-Archivum
        {
            let mut l = self.ledger.lock().await;
            l.commit_event(
                "mirror_reflection_draft",
                draft_hash.clone(),
                draft_plan.as_bytes(),
            )?;
        }

        // 5. Formulate the Critique Prompt (The "Mirror" Phase)
        let critique_prompt = format!(
            "Analyze and critique the following draft plan against governance invariants.\nDraft Hash: {}\nPlan: {}",
            draft_hash, draft_plan
        );

        Ok(critique_prompt)
    }
}
