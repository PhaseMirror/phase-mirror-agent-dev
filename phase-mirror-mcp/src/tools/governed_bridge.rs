use serde::{Deserialize, Serialize};
<<<<<<< HEAD
use serde_json::Value;

pub type Prime = u64;
pub type TissueId = u64;
pub type Tick = u64;
=======
use std::collections::HashMap;
use verification_harness::{
    AppendOnlyLog, AuditBlock, MorphismRecord, Prime, RegHomRegistry, Tick, TissueId,
    evaluate_governed_bridge,
};
>>>>>>> 49da540 (Add conformance tests for LP-01 to LP-12 and stratification witness)

/// `verification-harness` lives outside this workspace
/// (`../../../Foundry/tests/verification-harness`). This module keeps the
/// tool shape and refuses the call rather than depending on a path that is
/// not here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernedBridgeRequest {
    pub src_prime: Prime,
    pub tgt_prime: Prime,
    pub tissue_id: TissueId,
    pub current_tick: Tick,
    pub jubilee_window: (Tick, Tick),
    pub audit_blocks: Vec<AuditBlock>,
    pub morphisms: Vec<MorphismRecord>,
    pub pre_memory: [u8; 4],
    pub post_memory: [u8; 4],
}

<<<<<<< HEAD
pub fn check_governed_bridge(_req: GovernedBridgeRequest) -> Result<bool, String> {
    Err("verification-harness is not in this workspace".into())
=======
pub fn check_governed_bridge(req: GovernedBridgeRequest) -> Result<bool, String> {
    // 1. Reconstruct CRMF and ACE certificates log
    let mut audit_log = AppendOnlyLog::default();
    for block in req.audit_blocks {
        audit_log.commit(block).map_err(|e| e.to_string())?;
    }

    // 2. Reconstruct RegHom registry
    let mut reg_hom = RegHomRegistry {
        morphisms: HashMap::new(),
    };
    for m in req.morphisms {
        reg_hom.morphisms.insert((m.src_prime, m.tgt_prime), m);
    }

    // 3. Evaluate bridge via harness
    evaluate_governed_bridge(
        req.src_prime,
        req.tgt_prime,
        req.tissue_id,
        req.current_tick,
        req.jubilee_window,
        &audit_log,
        &reg_hom,
        &req.pre_memory,
        &req.post_memory,
    )
>>>>>>> 49da540 (Add conformance tests for LP-01 to LP-12 and stratification witness)
}
