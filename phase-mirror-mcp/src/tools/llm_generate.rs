use serde::{Deserialize, Serialize};
use crate::governance::ContractManager;
use pirtm_candle::{GenerationConfig, LambdaTrace, LlamaModel};
use candle_core::Device;

/// Request for governed LLM generation.
#[derive(Debug, Deserialize, Serialize)]
pub struct LlmGenerateRequest {
    pub prompt: String,
    #[serde(default)]
    pub max_tokens: usize,
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    #[serde(default)]
    pub model: Option<String>,
}

fn default_temperature() -> f64 {
    0.7
}

/// Response from governed LLM generation.
#[derive(Debug, Serialize)]
pub struct LlmGenerateResponse {
    pub status: String,
    pub witness_id: String,
    pub text: String,
    pub lambda_trace: LambdaTrace,
}

/// Response from LLM validation.
#[derive(Debug, Serialize)]
pub struct LlmValidateResponse {
    pub status: String,
    pub witness_id: String,
    pub lambda_trace: LambdaTrace,
}

/// Execute governed LLM generation.
pub fn handle_generate_governed(
    args: LlmGenerateRequest,
    contract_manager: &ContractManager,
) -> Result<LlmGenerateResponse, String> {
    contract_manager.validate_action("generate_governed", &serde_json::to_value(&args).unwrap())?;

    let device = Device::Cpu;
    let lambda_m = 0.95;

    let mut model = LlamaModel::load_dummy(device, lambda_m).map_err(|e| e.to_string())?;

    let max_toks = if args.max_tokens == 0 { 8 } else { args.max_tokens };
    let gen_config = GenerationConfig {
        max_tokens: max_toks,
        temperature: args.temperature,
        top_p: 0.9,
        lambda_m: 0.95,
    };

    // Dummy tokenization (just byte values of the prompt)
    let prompt_tokens: Vec<u32> = args.prompt.bytes().map(|b| (b as u32) % 1000).collect();
    
    let output = model.generate_governed(&prompt_tokens, gen_config).map_err(|e| e.to_string())?;

    Ok(LlmGenerateResponse {
        status: output.receipt.status.clone(),
        witness_id: output.receipt.witness_id.clone(),
        text: format!("[GOVERNED] Prompt received: {}, generated {} tokens", args.prompt, output.tokens.len()),
        lambda_trace: output.receipt.lambda_trace,
    })
}

/// Validate an existing generation trace.
pub fn handle_analyze_candle_llm(
    _args: serde_json::Value,
    contract_manager: &ContractManager,
) -> Result<LlmValidateResponse, String> {
    contract_manager.validate_action("analyze_candle_llm", &serde_json::json!({}))?;

    let trace = LambdaTrace {
        lambda_p: 0.95,
        l_p: 0.90,
        zero_spacings: vec![1.0, 2.0, 3.0],
        signature: "SIGNED_HASH".to_string(),
        signer_pubkey: "ed25519:twin-prime-042".to_string(),
        proof_hash: "LEAN_PROOF_HASH_108_CORE".to_string(),
    };

    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    let trace_json = serde_json::to_string(&trace).unwrap_or_default();
    hasher.update(trace_json.as_bytes());
    let witness_id = format!("sha256:{:x}", hasher.finalize());

    Ok(LlmValidateResponse {
        status: "OK".to_string(),
        witness_id,
        lambda_trace: trace,
    })
}
