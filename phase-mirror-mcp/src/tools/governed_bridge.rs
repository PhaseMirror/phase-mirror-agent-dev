use serde::{Deserialize, Serialize};

/// `verification-harness` lives outside this workspace
/// (`../../../Foundry/tests/verification-harness`). This module keeps the
/// tool shape and refuses the call rather than depending on a path that is
/// not here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernedBridgeRequest {
    pub src_prime: u64,
    pub tgt_prime: u64,
    pub tissue_id: u64,
    pub current_tick: u64,
    pub jubilee_window: (u64, u64),
    pub audit_blocks: Vec<serde_json::Value>,
    pub morphisms: Vec<serde_json::Value>,
    pub pre_memory: [u8; 4],
    pub post_memory: [u8; 4],
}

pub fn check_governed_bridge(_req: GovernedBridgeRequest) -> Result<bool, String> {
    Err("verification-harness is not in this workspace".into())
}
