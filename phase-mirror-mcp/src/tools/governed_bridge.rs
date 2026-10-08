use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use verification_harness::{
    AppendOnlyLog, AuditBlock, MorphismRecord, Prime, RegHomRegistry, Tick, TissueId,
    evaluate_governed_bridge,
};

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
}
