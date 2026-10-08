use crate::{ContractivityReceipt, GovernanceStatus, LambdaTrace, PirtmError, Result, SedonaSpineEvaluator};
use crate::math::spectral::{SpectralGovernor, SpectralMetrics};

#[derive(Debug, Clone, Copy)]
pub struct LambdaMConfig {
    pub lambda_m: f64,
    pub contraction_factor: f64,
    pub max_lip: f64,
}

impl Default for LambdaMConfig {
    fn default() -> Self {
        Self {
            lambda_m: 0.95,
            contraction_factor: 0.97,
            max_lip: 1.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LambdaMOp {
    config: LambdaMConfig,
}

impl LambdaMOp {
    pub fn new(config: LambdaMConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self::new(LambdaMConfig::default())
    }

    pub fn config(&self) -> &LambdaMConfig {
        &self.config
    }

    pub fn scale_residual(&self, residual_norm: f64, zero_spacings: &mut Vec<f64>) -> Result<f64> {
        let scaled = residual_norm * self.config.lambda_m * self.config.contraction_factor;
        let effective_lip = self.config.lambda_m * self.config.max_lip;
        
        if effective_lip >= 1.0 {
            return Err(PirtmError::ContractivityViolation(effective_lip));
        }

        zero_spacings.push(scaled);
        Ok(scaled)
    }

    pub fn verify_contractivity(&self, lip_estimate: f64) -> Result<GovernanceStatus> {
        let effective_lip = self.config.lambda_m * lip_estimate;
        if effective_lip >= self.config.max_lip {
            return Ok(GovernanceStatus::Kill);
        }
        Ok(GovernanceStatus::Ok)
    }

    pub fn build_trace(
        &self,
        zero_spacings: Vec<f64>,
        lip_estimate: f64,
    ) -> Result<ContractivityReceipt> {
        let trace = LambdaTrace {
            lambda_p: self.config.lambda_m,
            l_p: lip_estimate,
            zero_spacings,
            signature: "SIGNED_HASH".to_string(),
            signer_pubkey: "ed25519:twin-prime-042".to_string(),
            proof_hash: "LEAN_PROOF_HASH_108_CORE".to_string(),
        };

        let status = SedonaSpineEvaluator::evaluate_stop_rules(&trace)?;
        let status_str = match status {
            GovernanceStatus::Ok => "OK",
            GovernanceStatus::Warn => "WARN",
            GovernanceStatus::Kill => "KILL",
        };

        let receipt = ContractivityReceipt {
            status: status_str.to_string(),
            witness_id: format!("sha256:{}", hex::encode([0u8; 32])),
            lambda_trace: trace,
        };

        Ok(receipt)
    }
}

#[derive(Debug, Clone)]
pub struct SessionGraphOp {
    pub session_id: String,
    pub lambda_op: LambdaMOp,
    pub accumulated_drift: f64,
    pub max_drift_allowance: f64,
    pub spectral_metrics: Option<SpectralMetrics>,
}

impl SessionGraphOp {
    pub fn new(session_id: String, lambda_op: LambdaMOp, max_drift_allowance: f64) -> Self {
        Self {
            session_id,
            lambda_op,
            accumulated_drift: 0.0,
            max_drift_allowance,
            spectral_metrics: None,
        }
    }

    pub fn with_spectral(session_id: String, lambda_op: LambdaMOp, max_drift_allowance: f64, gain_matrix: Option<&[Vec<f64>]>, tier: usize) -> Result<Self> {
        let spectral_metrics = if let Some(matrix) = gain_matrix {
            Some(SpectralGovernor::evaluate_stability(matrix, tier).map_err(PirtmError::SpectralViolation)?)
        } else {
            None
        };
        Ok(Self {
            session_id,
            lambda_op,
            accumulated_drift: 0.0,
            max_drift_allowance,
            spectral_metrics,
        })
    }

    pub fn apply_step(&mut self, residual_norm: f64, zero_spacings: &mut Vec<f64>) -> Result<f64> {
        let scaled = self.lambda_op.scale_residual(residual_norm, zero_spacings)?;
        
        self.accumulated_drift += scaled;
        
        if self.accumulated_drift >= self.max_drift_allowance {
            return Err(PirtmError::ContractivityViolation(self.accumulated_drift));
        }

        Ok(scaled)
    }
}

impl LambdaMConfig {
    pub fn lambda_m(&self) -> f64 {
        self.lambda_m
    }
}