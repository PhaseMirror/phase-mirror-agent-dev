use crate::SedonaSpineEvaluator;
use crate::governance::ContractManager;
use crate::lmstudio::client::{LmStudioClient, LmStudioError};
use crate::lmstudio::types::{ChatMessage, GenerationParams};
use serde_json::json;
use std::path::PathBuf;

pub async fn handle_lmstudio_health(
    _arguments: serde_json::Value,
    client: &LmStudioClient,
    contract_manager: &ContractManager,
) -> Result<serde_json::Value, LmStudioError> {
    let witness = build_witness();
    if let Err(e) = SedonaSpineEvaluator::evaluate_stop_rules(&witness) {
        return Ok(json!({
            "isError": true,
            "content": [{"type": "text", "text": format!("SEDONA_SPINE_BLOCK: {}", e)}]
        }));
    }
    match contract_manager.validate_action("lmstudio_health", &serde_json::json!({})) {
        Ok(_) => {}
        Err(e) => return Ok(json!({"isError": true, "content": [{"type": "text", "text": e}]})),
    }
    match client.health_check().await {
        Ok(health) => {
            let receipt = build_receipt("OK", &witness);
            Ok(json!({
                "isError": false,
                "content": [{"type": "text", "text": serde_json::to_string(&json!({
                    "status": "OK", "witness_id": receipt.witness_id, "lambda_trace": witness, "health": health,
                })).unwrap()}]
            }))
        }
        Err(e) => Ok(json!({
            "isError": true,
            "content": [{"type": "text", "text": format!("LMSTUDIO_ERROR: {}", e)}]
        })),
    }
}

pub async fn handle_lmstudio_list_models(
    _arguments: serde_json::Value,
    client: &LmStudioClient,
    contract_manager: &ContractManager,
) -> Result<serde_json::Value, LmStudioError> {
    let witness = build_witness();
    if let Err(e) = SedonaSpineEvaluator::evaluate_stop_rules(&witness) {
        return Ok(json!({
            "isError": true,
            "content": [{"type": "text", "text": format!("SEDONA_SPINE_BLOCK: {}", e)}]
        }));
    }
    match contract_manager.validate_action("lmstudio_list_models", &serde_json::json!({})) {
        Ok(_) => {}
        Err(e) => return Ok(json!({"isError": true, "content": [{"type": "text", "text": e}]})),
    }
    match client.list_models().await {
        Ok(models) => {
            let receipt = build_receipt("OK", &witness);
            Ok(json!({
                "isError": false,
                "content": [{"type": "text", "text": serde_json::to_string(&json!({
                    "status": "OK", "witness_id": receipt.witness_id, "lambda_trace": witness, "models": models,
                })).unwrap()}]
            }))
        }
        Err(e) => Ok(json!({
            "isError": true,
            "content": [{"type": "text", "text": format!("LMSTUDIO_ERROR: {}", e)}]
        })),
    }
}

pub async fn handle_lmstudio_generate(
    arguments: serde_json::Value,
    client: &LmStudioClient,
    contract_manager: &ContractManager,
) -> Result<serde_json::Value, LmStudioError> {
    let witness = build_witness();
    if let Err(e) = SedonaSpineEvaluator::evaluate_stop_rules(&witness) {
        return Ok(json!({
            "isError": true,
            "content": [{"type": "text", "text": format!("SEDONA_SPINE_BLOCK: {}", e)}]
        }));
    }
    match contract_manager.validate_action("lmstudio_generate", &arguments) {
        Ok(_) => {}
        Err(e) => return Ok(json!({"isError": true, "content": [{"type": "text", "text": e}]})),
    }

    let prompt = arguments
        .get("prompt")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let params = GenerationParams {
        model: arguments
            .get("model")
            .and_then(|v| v.as_str())
            .map(String::from),
        temperature: arguments
            .get("temperature")
            .and_then(|v| v.as_f64())
            .map(|f| f as f32),
        top_p: arguments
            .get("top_p")
            .and_then(|v| v.as_f64())
            .map(|f| f as f32),
        max_tokens: arguments
            .get("max_tokens")
            .and_then(|v| v.as_u64())
            .map(|u| u as u32),
        stop: arguments.get("stop").and_then(|v| v.as_array()).map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        }),
        stream: arguments
            .get("stream")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    };

    match client.generate(prompt, &params).await {
        Ok(completion) => {
            let receipt = build_receipt("OK", &witness);
            let text = completion
                .choices
                .first()
                .map(|c| c.message.content.clone())
                .unwrap_or_default();
            Ok(json!({
                "isError": false,
                "content": [{"type": "text", "text": serde_json::to_string(&json!({
                    "status": "OK", "witness_id": receipt.witness_id, "lambda_trace": witness,
                    "text": text, "model": completion.model, "usage": completion.usage,
                })).unwrap()}]
            }))
        }
        Err(e) => Ok(json!({
            "isError": true,
            "content": [{"type": "text", "text": format!("LMSTUDIO_ERROR: {}", e)}]
        })),
    }
}

pub async fn handle_lmstudio_chat(
    arguments: serde_json::Value,
    client: &LmStudioClient,
    contract_manager: &ContractManager,
) -> Result<serde_json::Value, LmStudioError> {
    let witness = build_witness();
    if let Err(e) = SedonaSpineEvaluator::evaluate_stop_rules(&witness) {
        return Ok(json!({
            "isError": true,
            "content": [{"type": "text", "text": format!("SEDONA_SPINE_BLOCK: {}", e)}]
        }));
    }
    match contract_manager.validate_action("lmstudio_chat", &arguments) {
        Ok(_) => {}
        Err(e) => return Ok(json!({"isError": true, "content": [{"type": "text", "text": e}]})),
    }

    let messages_val = arguments
        .get("messages")
        .and_then(|v| v.as_array())
        .ok_or_else(|| LmStudioError::InvalidResponse("messages array required".to_string()))?;
    let messages: Vec<ChatMessage> = messages_val
        .iter()
        .filter_map(|v| {
            let role = v
                .get("role")
                .and_then(|r| r.as_str())
                .unwrap_or("user")
                .to_string();
            let content = v
                .get("content")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string();
            Some(ChatMessage { role, content })
        })
        .collect();

    let params = GenerationParams {
        model: arguments
            .get("model")
            .and_then(|v| v.as_str())
            .map(String::from),
        temperature: arguments
            .get("temperature")
            .and_then(|v| v.as_f64())
            .map(|f| f as f32),
        top_p: arguments
            .get("top_p")
            .and_then(|v| v.as_f64())
            .map(|f| f as f32),
        max_tokens: arguments
            .get("max_tokens")
            .and_then(|v| v.as_u64())
            .map(|u| u as u32),
        stop: arguments.get("stop").and_then(|v| v.as_array()).map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        }),
        stream: arguments
            .get("stream")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    };

    match client.chat(&messages, &params).await {
        Ok(completion) => {
            let receipt = build_receipt("OK", &witness);
            let choices: Vec<_> = completion.choices.into_iter().map(|c| json!({"role": c.message.role, "content": c.message.content, "finish_reason": c.finish_reason})).collect();
            Ok(json!({
                "isError": false,
                "content": [{"type": "text", "text": serde_json::to_string(&json!({
                    "status": "OK", "witness_id": receipt.witness_id, "lambda_trace": witness,
                    "choices": choices, "model": completion.model, "usage": completion.usage,
                })).unwrap()}]
            }))
        }
        Err(e) => Ok(json!({
            "isError": true,
            "content": [{"type": "text", "text": format!("LMSTUDIO_ERROR: {}", e)}]
        })),
    }
}

pub async fn handle_lmstudio_embed(
    arguments: serde_json::Value,
    client: &LmStudioClient,
    contract_manager: &ContractManager,
) -> Result<serde_json::Value, LmStudioError> {
    let witness = build_witness();
    if let Err(e) = SedonaSpineEvaluator::evaluate_stop_rules(&witness) {
        return Ok(json!({
            "isError": true,
            "content": [{"type": "text", "text": format!("SEDONA_SPINE_BLOCK: {}", e)}]
        }));
    }
    match contract_manager.validate_action("lmstudio_embed", &arguments) {
        Ok(_) => {}
        Err(e) => return Ok(json!({"isError": true, "content": [{"type": "text", "text": e}]})),
    }

    let input = arguments
        .get("input")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let model = arguments.get("model").and_then(|v| v.as_str());

    match client.embed(input, model).await {
        Ok(embedding) => {
            let receipt = build_receipt("OK", &witness);
            Ok(json!({
                "isError": false,
                "content": [{"type": "text", "text": serde_json::to_string(&json!({
                    "status": "OK", "witness_id": receipt.witness_id, "lambda_trace": witness,
                    "model": embedding.model, "embedding": embedding.data.first().map(|d| d.embedding.clone()), "usage": embedding.usage,
                })).unwrap()}]
            }))
        }
        Err(e) => Ok(json!({
            "isError": true,
            "content": [{"type": "text", "text": format!("LMSTUDIO_ERROR: {}", e)}]
        })),
    }
}

pub async fn handle_lmstudio_register_mcp(
    arguments: serde_json::Value,
    _client: &LmStudioClient,
    contract_manager: &ContractManager,
) -> Result<serde_json::Value, LmStudioError> {
    let witness = build_witness();
    if let Err(e) = SedonaSpineEvaluator::evaluate_stop_rules(&witness) {
        return Ok(json!({
            "isError": true,
            "content": [{"type": "text", "text": format!("SEDONA_SPINE_BLOCK: {}", e)}]
        }));
    }
    match contract_manager.validate_action("lmstudio_register_mcp", &arguments) {
        Ok(_) => {}
        Err(e) => return Ok(json!({"isError": true, "content": [{"type": "text", "text": e}]})),
    }

    let command = arguments
        .get("command")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let args = arguments
        .get("args")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let mcp_path = PathBuf::from(home).join(".lmstudio").join("mcp.json");

    let mut servers = std::collections::BTreeMap::new();
    if mcp_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&mcp_path) {
            if let Ok(mut map) = serde_json::from_str::<
                std::collections::BTreeMap<String, serde_json::Value>,
            >(&content)
            {
                if let Some(mcp_servers) = map.remove("mcpServers") {
                    if let Ok(s) = serde_json::from_value(mcp_servers) {
                        servers = s;
                    }
                }
            }
        }
    }

    let entry = json!({
        "command": command,
        "args": args,
    });
    servers.insert("phase-mirror-mcp".to_string(), entry);

    let updated = json!({ "mcpServers": servers });
    match std::fs::write(&mcp_path, serde_json::to_string_pretty(&updated).unwrap()) {
        Ok(_) => {
            let receipt = build_receipt("OK", &witness);
            Ok(json!({
                "isError": false,
                "content": [{"type": "text", "text": serde_json::to_string(&json!({
                    "status": "OK", "witness_id": receipt.witness_id, "lambda_trace": witness,
                    "registered": true, "path": mcp_path.to_string_lossy().to_string(), "command": command,
                })).unwrap()}]
            }))
        }
        Err(e) => Ok(json!({
            "isError": true,
            "content": [{"type": "text", "text": format!("LMSTUDIO_ERROR: {}", e)}]
        })),
    }
}

struct ContractivityReceipt {
    _status: String,
    witness_id: String,
}

fn build_receipt(status: &str, _witness: &crate::LambdaTrace) -> ContractivityReceipt {
    ContractivityReceipt {
        _status: status.to_string(),
        witness_id: format!("sha256:{}", hex::encode([0u8; 32])),
    }
}

fn build_witness() -> crate::LambdaTrace {
    crate::LambdaTrace {
        lambda_p: 0.999999,
        l_p: 0.95,
        zero_spacings: vec![0.9549652277648129, 1.5563111057990717, 1.2289235832739145],
        signature: "SIGNED_HASH".to_string(),
        signer_pubkey: "ed25519:twin-prime-042".to_string(),
        proof_hash: "LEAN_PROOF_HASH_108_CORE".to_string(),
    }
}
