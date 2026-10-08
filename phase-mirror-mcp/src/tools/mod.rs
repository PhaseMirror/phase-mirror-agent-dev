//! Thin wrapper for Engine → SDK integration.
//!
//! All UI/Agent tool calls must route through this path:
//! Engine → SDK (`src/tools/mod.rs`) → Contract (`models/legalese-scopist/CONTRACT.md`) → UI.
//! This ensures zero‑drift and policy‑driven decision making.

use crate::governance::ContractManager;
use crate::persistence::CrmfStorage;
use serde_json::Value;
use tokio::sync::Mutex;

/// Forward a tool request through the mandated path.
///
/// This function is the only public entry point for UI agents to invoke
/// backend tools. It delegates to the internal `legacy_tool_handler`
/// which performs contract validation and engine execution.
pub async fn handle_tool_call(
    tool_name: &str,
    arguments: &Value,
    contract_manager: &ContractManager,
    crmf_storage: &Mutex<CrmfStorage>,
) -> Value {
    crate::legacy_tool_handler(tool_name, arguments, contract_manager, crmf_storage).await
}

// Re‑export tool modules for external use.
pub mod evaluate_esi_risk;
pub mod governed_bridge;
pub mod litigation_scan;
pub mod spoliation_check;
pub mod stability_witness;
pub mod verify_ledger;

pub use evaluate_esi_risk::{EsiRiskRequest, EsiRiskResponse, evaluate_esi_risk_logic};
pub use governed_bridge::{GovernedBridgeRequest, check_governed_bridge};
pub use litigation_scan::{
    ActiveHoldSummary, HoldRecord, LitigationScanResult, scan_for_litigation_hold,
};
pub use spoliation_check::{
    LineageEvent, LitigationHold, SpoliationCheckResult, SpoliationViolation,
    scan_for_spoliation_risk,
};
pub use stability_witness::{
    SpectralMeasurement, StabilityBreakdown, StabilityMetricResult, get_stability_metric,
};
pub use verify_ledger::{VerifyLedgerRequest, VerifyLedgerResponse, verify_ledger_integrity};
pub mod github_actions;
pub use github_actions::*;
pub mod prismpm_actions;
pub use prismpm_actions::*;
