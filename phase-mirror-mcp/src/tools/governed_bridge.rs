use serde::{Deserialize, Serialize};
use serde_json::Value;

pub type Prime = u64;
pub type TissueId = u64;
pub type Tick = u64;

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
    pub audit_blocks: Vec<Value>,
    pub morphisms: Vec<Value>,
    pub pre_memory: [u8; 4],
    pub post_memory: [u8; 4],
}

pub fn check_governed_bridge(_req: GovernedBridgeRequest) -> Result<bool, String> {
    Err("verification-harness is not in this workspace".into())
}
