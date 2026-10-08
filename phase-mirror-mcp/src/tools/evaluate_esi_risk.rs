use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreservationState {
    pub last_verified: u64, // Unix timestamp
    pub redundancy_level: u8,
    pub storage_tier: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EsiRiskRequest {
    pub artifact_id: String,
    pub preservation_state: PreservationState,
    pub active_litigation: bool,
    pub data_sensitivity: String, // e.g., "High", "Medium", "Low"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EsiRiskResponse {
    pub risk_score: f64, // 0.0 to 1.0
    pub recommendation: String,
    pub retention_period_days: u32,
    pub litigation_hold_active: bool,
    pub spoliation_warning: bool,
}

pub fn evaluate_esi_risk_logic(req: EsiRiskRequest) -> EsiRiskResponse {
    let mut risk_score: f64 = 0.0;
    let mut spoliation_warning = false;
    let mut litigation_hold_active = req.active_litigation;

    // 1. Litigation Hold Logic
    if req.active_litigation {
        risk_score += 0.4;
        litigation_hold_active = true;
    }

    // 2. Preservation Risk
    if req.preservation_state.redundancy_level < 2 {
        risk_score += 0.3;
        spoliation_warning = true;
    }

    // 3. Sensitivity Risk
    if req.data_sensitivity == "High" {
        risk_score += 0.2;
    }

    // 4. Verification Latency
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let age = now.saturating_sub(req.preservation_state.last_verified);
    if age > 86400 * 30 {
        // Older than 30 days
        risk_score += 0.1;
    }

    let risk_score = risk_score.min(1.0);

    let (recommendation, retention_period_days) = if litigation_hold_active {
        (
            "INDEFINITE HOLD: Artifact is subject to active litigation. Do not delete.".to_string(),
            36500,
        ) // ~100 years
    } else if risk_score > 0.5 {
        ("CRITICAL: High preservation risk. Increase redundancy and verify integrity immediately.".to_string(), 3650)
    } else {
        (
            "STANDARD: Artifact is within normal preservation parameters.".to_string(),
            2555,
        ) // 7 years
    };

    EsiRiskResponse {
        risk_score,
        recommendation,
        retention_period_days,
        litigation_hold_active,
        spoliation_warning,
    }
}
