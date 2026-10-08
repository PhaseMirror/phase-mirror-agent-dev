use anyhow::{Result, anyhow};
use pirtm_candle::model::{LlamaModel, GenerationConfig};
use candle_core::Device;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct LlmClient {
    pub model_path: String,
    pub temperature: f32,
    pub max_tokens: usize,
    model: Arc<Mutex<LlamaModel>>,
    system_prompt: String,
}

impl Default for LlmClient {
    fn default() -> Self {
        let model_path = std::env::var("LLM_MODEL_PATH")
            .unwrap_or_else(|_| "".to_string());
        let config_path = std::env::var("LLM_CONFIG_PATH")
            .unwrap_or_else(|_| "".to_string());
        
        let system_prompt_path = std::env::var("LLM_SYSTEM_PROMPT_PATH")
            .unwrap_or_else(|_| "../../The Phase of Mirror Dissonance.md".to_string());
        let raw_system_prompt = std::fs::read_to_string(&system_prompt_path)
            .unwrap_or_else(|_| "You are the Phase Mirror Agent, an AI cognitive assistant.".to_string());
        let system_prompt = truncate_to_token_budget(&raw_system_prompt, 512);
        
        let device = Device::Cpu;
        
        let model = if model_path.is_empty() || config_path.is_empty() {
            LlamaModel::load_dummy(device, 0.8).expect("Failed to load dummy model")
        } else {
            LlamaModel::load(&config_path, device.clone(), &model_path, 0.8).unwrap_or_else(|_| {
                eprintln!("Warning: Failed to load real model from {}, falling back to dummy", model_path);
                LlamaModel::load_dummy(Device::Cpu, 0.8).expect("Failed to load dummy")
            })
        };

        Self {
            model_path,
            temperature: 0.7,
            max_tokens: 512,
            model: Arc::new(Mutex::new(model)),
            system_prompt,
        }
    }
}

impl LlmClient {
    pub fn new(
        model_path: impl Into<String>,
        config_path: impl Into<String>,
        temperature: f32,
        max_tokens: usize,
    ) -> Self {
        let model_path = model_path.into();
        let config_path = config_path.into();
        
        let system_prompt_path = std::env::var("LLM_SYSTEM_PROMPT_PATH")
            .unwrap_or_else(|_| "../../The Phase of Mirror Dissonance.md".to_string());
        let raw_system_prompt = std::fs::read_to_string(&system_prompt_path)
            .unwrap_or_else(|_| "You are the Phase Mirror Agent, an AI cognitive assistant.".to_string());
        let system_prompt = truncate_to_token_budget(&raw_system_prompt, 512);
        
        let device = Device::Cpu;
        
        let model = if model_path.is_empty() || config_path.is_empty() {
            LlamaModel::load_dummy(device, 0.8).expect("Failed to load dummy model")
        } else {
            LlamaModel::load(&config_path, device.clone(), &model_path, 0.8).unwrap_or_else(|_| {
                eprintln!("Warning: Failed to load real model from {}, falling back to dummy", model_path);
                LlamaModel::load_dummy(Device::Cpu, 0.8).expect("Failed to load dummy")
            })
        };

        Self {
            model_path,
            temperature,
            max_tokens,
            model: Arc::new(Mutex::new(model)),
            system_prompt,
        }
    }

    pub async fn generate(&self, prompt: &str) -> Result<String> {
        self.generate_with_params(prompt, self.max_tokens, self.temperature).await
    }

    pub async fn generate_with_params(&self, prompt: &str, max_tokens: usize, temperature: f32) -> Result<String> {
        let mut model = self.model.lock().map_err(|_| anyhow!("Poisoned mutex"))?;
        
        let mut formatted_prompt = self.system_prompt.trim().to_string();
        formatted_prompt.push_str("\n\nUser: ");
        formatted_prompt.push_str(prompt.trim());
        
        let tokenizer_path = std::path::Path::new(&self.model_path).parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("tokenizer.json");

        let (prompt_tokens, tokenizer) = if tokenizer_path.exists() {
            let tokenizer = tokenizers::Tokenizer::from_file(&tokenizer_path)
                .map_err(|e| anyhow!("Failed to load tokenizer: {}", e))?;
            let fp = formatted_prompt.clone();
            let encoding = tokenizer.encode(fp.as_str(), true)
                .map_err(|e| anyhow!("Failed to encode prompt: {}", e))?;
            (encoding.get_ids().to_vec(), Some(tokenizer))
        } else {
            (formatted_prompt.bytes().map(|b| b as u32).collect(), None)
        };
        
        let gen_config = GenerationConfig {
            max_tokens,
            temperature: temperature as f64,
            top_p: 0.9,
            lambda_m: 0.01,
        };
        
        let generation = model.generate_governed(&prompt_tokens, gen_config)
            .map_err(|e| anyhow!("Governed generation failed: {:?}", e))?;
            
        let generated_len = generation.tokens.len().saturating_sub(prompt_tokens.len());
        let new_tokens = &generation.tokens[prompt_tokens.len()..];

        if let Some(tokenizer) = tokenizer {
            let decoded = tokenizer.decode(new_tokens, true)
                .map_err(|e| anyhow!("Failed to decode output: {}", e))?;
            Ok(format!("{}\n\n(Phase Mirror Governance OK. Tokens generated: {})", decoded, generated_len))
        } else {
            Ok(format!("Generated {} tokens successfully.\n(Detokenization disabled: missing tokenizer.json in model directory). Phase Mirror Governance OK.", generated_len))
        }
    }
}

fn truncate_to_token_budget(text: &str, max_chars: usize) -> String {
    if text.len() <= max_chars {
        return text.to_string();
    }
    let mut truncated = text[..max_chars].to_string();
    truncated.push_str("...[truncated for context budget]");
    truncated
}
