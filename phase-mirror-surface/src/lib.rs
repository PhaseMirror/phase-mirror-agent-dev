use serde::{Deserialize, Serialize};
use std::fmt;

/// The four sovereign surfaces defined in ADR-002.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SurfaceState {
    ChromiumExtension,
    VSCodeExtension,
    ESP32Edge,
    LocalFirstData,
}

impl fmt::Display for SurfaceState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SurfaceState::ChromiumExtension => write!(f, "ChromiumExtension"),
            SurfaceState::VSCodeExtension => write!(f, "VSCodeExtension"),
            SurfaceState::ESP32Edge => write!(f, "ESP32Edge"),
            SurfaceState::LocalFirstData => write!(f, "LocalFirstData"),
        }
    }
}

/// The four phases of the Triple-Lock sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TripleLockPhase {
    Genius,
    Guardian,
    Examiner,
    Completed,
    Failed,
}

impl fmt::Display for TripleLockPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TripleLockPhase::Genius => write!(f, "Genius"),
            TripleLockPhase::Guardian => write!(f, "Guardian"),
            TripleLockPhase::Examiner => write!(f, "Examiner"),
            TripleLockPhase::Completed => write!(f, "Completed"),
            TripleLockPhase::Failed => write!(f, "Failed"),
        }
    }
}

/// The outcome of a governance check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GovernanceOutcome {
    Admissible,
    Blocked(Vec<String>),
    Degraded(String),
}

/// Unified receipt envelope emitted by every governed action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractivityReceipt {
    pub status: String,
    pub witness_id: String,
    pub lambda_trace: LambdaTrace,
    pub proof_hash: String,
    pub lean_manifest_hash: String,
    pub surface: SurfaceState,
    pub archivum_log: String,
    pub triple_lock_phase: TripleLockPhase,
}

impl ContractivityReceipt {
    pub fn ok(
        witness_id: String,
        surface: SurfaceState,
        archivum_log: String,
        lambda_p: f64,
        l_p: f64,
        zero_spacings: Vec<u64>,
    ) -> Self {
        Self {
            status: "OK".to_string(),
            witness_id,
            lambda_trace: LambdaTrace {
                lambda_p,
                l_p,
                zero_spacings,
            },
            proof_hash: String::new(),
            lean_manifest_hash: String::new(),
            surface,
            archivum_log,
            triple_lock_phase: TripleLockPhase::Completed,
        }
    }

    pub fn blocked(witness_id: String, surface: SurfaceState, _reasons: Vec<String>) -> Self {
        Self {
            status: "BLOCKED".to_string(),
            witness_id,
            lambda_trace: LambdaTrace {
                lambda_p: 0.0,
                l_p: 0.0,
                zero_spacings: Vec::new(),
            },
            proof_hash: String::new(),
            lean_manifest_hash: String::new(),
            surface,
            archivum_log: String::new(),
            triple_lock_phase: TripleLockPhase::Failed,
        }
    }
}

/// Lambda trace embedded in every ContractivityReceipt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LambdaTrace {
    pub lambda_p: f64,
    pub l_p: f64,
    pub zero_spacings: Vec<u64>,
}

impl LambdaTrace {
    pub fn is_contractive(&self) -> bool {
        self.lambda_p * self.l_p < 1.0 && !self.zero_spacings.is_empty()
    }
}

/// Canonical append-only event stored in the local-first Archivum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchivumEvent {
    pub id: String,
    pub timestamp_ms: i64,
    pub surface: SurfaceState,
    pub witness_id: String,
    pub receipt_json: String,
    pub signature: String,
}

impl ArchivumEvent {
    pub fn new(
        surface: SurfaceState,
        witness_id: String,
        receipt_json: String,
        signature: String,
        timestamp_ms: i64,
    ) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        Self {
            id,
            timestamp_ms,
            surface,
            witness_id,
            receipt_json,
            signature,
        }
    }
}

/// Surface-agnostic L0 check contract.
///
/// Implemented by `phase-mirror`, `phase-mirror-edge`, and the WASM wrapper.
pub trait L0Check {
    type Input;
    type Output;
    fn check(state: Self::Input) -> Self::Output;
}

/// Surface-agnostic Triple-Lock state machine contract.
///
/// Implemented by `phase-mirror` and `phase-mirror-edge`.
pub trait TripleLockMachine {
    type Receipt;
    fn advance(&mut self, witness: &Self::Receipt) -> Result<TripleLockPhase, LockError>;
    fn current_phase(&self) -> TripleLockPhase;
}

/// Errors that can occur during Triple-Lock advancement.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("Lock error: {message}")]
pub struct LockError {
    pub message: String,
}

impl LockError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Every sovereign surface must implement this adapter.
pub trait SurfaceAdapter {
    fn surface_id(&self) -> SurfaceState;
    fn emit_receipt(&self, receipt: &ContractivityReceipt) -> Result<(), SurfaceError>;
    fn read_receipts(&self, since_ms: i64) -> Result<Vec<ContractivityReceipt>, SurfaceError>;
    fn triple_lock_status(&self) -> Result<TripleLockPhase, SurfaceError>;
}

/// Errors that can occur when interacting with a sovereign surface.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("Surface error: {message}")]
pub struct SurfaceError {
    pub message: String,
}

impl SurfaceError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lambda_trace_contractivity() {
        let trace = LambdaTrace {
            lambda_p: 0.95,
            l_p: 0.90,
            zero_spacings: vec![1, 3, 5, 7],
        };
        assert!(trace.is_contractive());

        let trace2 = LambdaTrace {
            lambda_p: 0.99,
            l_p: 0.99,
            zero_spacings: vec![],
        };
        assert!(!trace2.is_contractive());
    }

    #[test]
    fn receipt_ok_construction() {
        let receipt = ContractivityReceipt::ok(
            "witness-1".to_string(),
            SurfaceState::LocalFirstData,
            "archivum-1".to_string(),
            0.95,
            0.90,
            vec![1, 3, 5, 7],
        );
        assert_eq!(receipt.status, "OK");
        assert_eq!(receipt.surface, SurfaceState::LocalFirstData);
        assert!(receipt.lambda_trace.is_contractive());
    }

    #[test]
    fn receipt_blocked_construction() {
        let receipt = ContractivityReceipt::blocked(
            "witness-2".to_string(),
            SurfaceState::ESP32Edge,
            vec!["drift too high".to_string()],
        );
        assert_eq!(receipt.status, "BLOCKED");
        assert_eq!(receipt.surface, SurfaceState::ESP32Edge);
        assert!(!receipt.lambda_trace.is_contractive());
    }
}
