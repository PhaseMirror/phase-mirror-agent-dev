use crate::{
    contractivity::{LambdaMOp, SessionGraphOp}, witness::WitnessEmitter, ContractivityReceipt,
    GenerationWitness, PirtmError, Result,
};
use crate::math::spectral::SpectralMetrics;
use candle_core::{Device, Tensor};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy)]
pub struct GenerationConfig {
    pub max_tokens: usize,
    pub temperature: f64,
    pub top_p: f64,
    pub lambda_m: f64,
}

impl Default for GenerationConfig {
    fn default() -> Self {
        Self {
            max_tokens: 256,
            temperature: 0.7,
            top_p: 0.9,
            lambda_m: 0.95,
        }
    }
}

use candle_nn::VarBuilder;
use candle_transformers::models::llama::{Llama, LlamaConfig, Config as LlamaModelConfig, Cache};
use candle_transformers::generation::LogitsProcessor;

pub struct LlamaModel {
    device: Device,
    lambda_op: LambdaMOp,
    model: Llama,
    config: LlamaModelConfig,
}

impl LlamaModel {
    pub fn load(
        config_path: &str,
        device: Device,
        model_path: &str,
        lambda_m: f64,
    ) -> Result<Self> {
        if model_path.is_empty() {
            return Err(PirtmError::ModelLoadError("Model path empty".into()));
        }

        let config_str = std::fs::read_to_string(config_path)
            .map_err(|e| PirtmError::ModelLoadError(format!("Config error: {}", e)))?;
        let llama_config: LlamaConfig = serde_json::from_str(&config_str)
            .map_err(|e| PirtmError::ModelLoadError(format!("Config parse error: {}", e)))?;
        let config = llama_config.into_config(false);

        let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[model_path], candle_core::DType::F32, &device).map_err(|e| PirtmError::ModelLoadError(e.to_string()))? };
        let model = Llama::load(vb, &config).map_err(|e| PirtmError::ModelLoadError(e.to_string()))?;

        let lambda_op = LambdaMOp::new(crate::contractivity::LambdaMConfig {
            lambda_m,
            ..Default::default()
        });

        Ok(Self {
            device,
            lambda_op,
            model,
            config,
        })
    }

    pub fn load_dummy(
        device: Device,
        lambda_m: f64,
    ) -> Result<Self> {
        let config_str = r#"{
            "vocab_size": 32000,
            "hidden_size": 4096,
            "intermediate_size": 11008,
            "num_hidden_layers": 32,
            "num_attention_heads": 32,
            "num_key_value_heads": 32,
            "max_position_embeddings": 2048,
            "rms_norm_eps": 1e-6,
            "rope_theta": 10000.0,
            "bos_token_id": 1,
            "eos_token_id": 2,
            "tie_word_embeddings": false
        }"#;
        let llama_config: LlamaConfig = serde_json::from_str(config_str)
            .map_err(|e| PirtmError::ModelLoadError(format!("Dummy Config parse error: {}", e)))?;
        let config = llama_config.into_config(false);

        let vb = VarBuilder::zeros(candle_core::DType::F32, &device);
        let model = Llama::load(vb, &config).map_err(|e| PirtmError::ModelLoadError(e.to_string()))?;

        let lambda_op = LambdaMOp::new(crate::contractivity::LambdaMConfig {
            lambda_m,
            ..Default::default()
        });

        Ok(Self {
            device,
            lambda_op,
            model,
            config,
        })
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn forward_step(
        &mut self,
        input_ids: &[u32],
        index_pos: usize,
        cache: &mut Cache,
    ) -> Result<(Tensor, f64)> {
        if input_ids.is_empty() {
            return Err(PirtmError::TensorError("Input IDs empty".into()));
        }

        let input_tensor = Tensor::new(input_ids, &self.device).unwrap().unsqueeze(0).unwrap();
        let logits = self.model.forward(&input_tensor, index_pos, cache).map_err(|e| PirtmError::TensorError(e.to_string()))?;
        
        let sqr = logits.sqr().map_err(|e| PirtmError::TensorError(e.to_string()))?;
        let sum = sqr.sum_all().map_err(|e| PirtmError::TensorError(e.to_string()))?;
        let norm_tensor = sum.sqrt().map_err(|e| PirtmError::TensorError(e.to_string()))?;
        let residual_norm = norm_tensor.to_scalar::<f32>().map_err(|e| PirtmError::TensorError(e.to_string()))? as f64;
        let residual_norm = residual_norm.min(1.0);

        Ok((logits, residual_norm))
    }

    pub fn scaled_residual(&self, residual_norm: f64, zero_spacings: &mut Vec<f64>) -> Result<f64> {
        self.lambda_op.scale_residual(residual_norm, zero_spacings)
    }

    pub fn generate_governed(
        &mut self,
        prompt_tokens: &[u32],
        gen_config: GenerationConfig,
    ) -> Result<GovernedGeneration> {
        self.generate_governed_with_spectral(prompt_tokens, gen_config, None, 2)
    }

    pub fn generate_governed_with_spectral(
        &mut self,
        prompt_tokens: &[u32],
        gen_config: GenerationConfig,
        gain_matrix: Option<&[Vec<f64>]>,
        tier: usize,
    ) -> Result<GovernedGeneration> {
        let mut session = SessionGraphOp::with_spectral(
            format!("session-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()),
            self.lambda_op.clone(),
            1.0 / gen_config.lambda_m,
            gain_matrix,
            tier,
        )?;

        let mut emitter = WitnessEmitter::new(self.lambda_op.clone(), gen_config.max_tokens);
        let mut zero_spacings = Vec::with_capacity(gen_config.max_tokens);
        let mut generated: Vec<u32> = prompt_tokens.to_vec();
        let mut witnesses = Vec::new();
        
        let mut logits_processor = LogitsProcessor::new(
            299792458, 
            Some(gen_config.temperature), 
            Some(gen_config.top_p)
        );

        let mut cache = Cache::new(true, candle_core::DType::F32, &self.config, &self.device)
            .map_err(|e| PirtmError::ModelLoadError(e.to_string()))?;

        for step in 0..gen_config.max_tokens {
            let (input_ids, index_pos) = if step == 0 {
                (prompt_tokens, 0)
            } else {
                (&generated[generated.len() - 1..], generated.len() - 1)
            };

            let (logits, residual_norm) = self.forward_step(input_ids, index_pos, &mut cache)?;

            let scaled_residual = session.apply_step(residual_norm, &mut zero_spacings)?;

            let witness = emitter.emit_step(
                step,
                generated.last().copied().unwrap_or(0),
                scaled_residual,
                &mut zero_spacings,
                1.0,
            )?;

            if witness.contractivity_status == "KILL" {
                return Err(PirtmError::ContractivityViolation(witness.l_p));
            }

            witnesses.push(witness);
            
            let logits_squeezed = logits.squeeze(0).map_err(|e| PirtmError::TensorError(e.to_string()))?;
            let next_token_logits = logits_squeezed;

            let next_token = logits_processor
                .sample(&next_token_logits)
                .map_err(|e| PirtmError::TensorError(e.to_string()))?;

            generated.push(next_token);
        }

        let receipt = emitter.finalize(zero_spacings, session.accumulated_drift)?;

        Ok(GovernedGeneration {
            tokens: generated,
            witnesses,
            receipt,
            spectral_metrics: session.spectral_metrics,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernedGeneration {
    pub tokens: Vec<u32>,
    pub witnesses: Vec<GenerationWitness>,
    pub receipt: ContractivityReceipt,
    pub spectral_metrics: Option<SpectralMetrics>,
}

impl GovernedGeneration {
    pub fn text(&self) -> String {
        format!("Generated {} tokens with {} witnesses", self.tokens.len(), self.witnesses.len())
    }

    pub fn receipt_status(&self) -> &str {
        &self.receipt.status
    }
}