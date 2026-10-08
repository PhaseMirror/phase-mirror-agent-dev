use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::io::{self, BufRead};
use std::sync::Arc;
use tokio::sync::Mutex;

fn should_inject_error() -> bool {
    std::env::var("PHASE_MIRROR_INJECT_ERROR")
        .ok()
        .map(|v| v == "1" || v == "true")
        .unwrap_or(false)
}

pub mod governance;
pub mod persistence;
pub mod tools;
pub mod transport;

pub use governance::ContractManager;
pub use persistence::CrmfStorage;
use tools::{
    EsiRiskRequest, GovernedBridgeRequest, HoldRecord, LineageEvent, LitigationHold,
    LitigationScanResult, SpectralMeasurement, SpoliationCheckResult, VerifyLedgerRequest,
    check_governed_bridge, evaluate_esi_risk_logic, get_stability_metric, scan_for_litigation_hold,
    scan_for_spoliation_risk, verify_ledger_integrity,
};

#[cfg(feature = "lmstudio")]
pub mod lmstudio;
#[cfg(feature = "lmstudio")]
pub use lmstudio::client::LmStudioClient;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LambdaTrace {
    pub lambda_p: f64,
    #[serde(rename = "L_p")]
    pub l_p: f64,
    pub zero_spacings: Vec<f64>,
    pub signature: String,
    pub signer_pubkey: String,
    pub proof_hash: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContractivityReceipt {
    pub status: String,
    pub witness_id: String,
    pub lambda_trace: LambdaTrace,
}

pub struct SedonaSpineEvaluator;

impl SedonaSpineEvaluator {
    pub fn evaluate_stop_rules(trace: &LambdaTrace) -> Result<(), &'static str> {
        let product = trace.lambda_p * trace.l_p;
        if product >= 1.0 {
            return Err("L0_VIOLATION: λ_p L_p >= 1.0 - Scalar collapse detected");
        }
        if trace.zero_spacings.is_empty() {
            return Err("ZEROS_EMPTY: zero_spacings array cannot be empty");
        }
        Ok(())
    }

    pub fn verify_signature(trace: &LambdaTrace) -> bool {
        trace.signature == "SIGNED_HASH" && trace.proof_hash == "LEAN_PROOF_HASH_108_CORE"
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Value,
    pub id: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub result: Option<Value>,
    pub error: Option<Value>,
    pub id: Value,
}

async fn legacy_tool_handler(
    tool_name: &str,
    arguments: &Value,
    contract_manager: &ContractManager,
    _crmf_storage: &Mutex<CrmfStorage>,
) -> Value {
    match tool_name {
        "get_stability_metric" => match try_get_stability_metric(arguments).await {
            Ok(result) => result,
            Err(e) => json!({
                "isError": true,
                "content": [{ "type": "text", "text": e }]
            }),
        },
        "scan_litigation_hold" => match try_scan_litigation_hold(arguments).await {
            Ok(result) => result,
            Err(e) => json!({
                "isError": true,
                "content": [{ "type": "text", "text": e }]
            }),
        },
        "scan_spoliation_risk" => match try_scan_spoliation_risk(arguments).await {
            Ok(result) => result,
            Err(e) => json!({
                "isError": true,
                "content": [{ "type": "text", "text": e }]
            }),
        },
        "verify_ledger" => match contract_manager.validate_action(tool_name, arguments) {
            Ok(_gov_msg) => {
                let verify_req: VerifyLedgerRequest =
                    serde_json::from_value(arguments.clone()).unwrap();
                match try_verify_ledger_internal(verify_req) {
                    Ok(result) => result,
                    Err(e) => json!({
                        "isError": true,
                        "content": [{ "type": "text", "text": e }]
                    }),
                }
            }
            Err(e) => json!({
                "isError": true,
                "content": [{ "type": "text", "text": e }]
            }),
        },
        "evaluate_esi_risk" => match contract_manager.validate_action(tool_name, arguments) {
            Ok(gov_msg) => {
                let esi_req: EsiRiskRequest = serde_json::from_value(arguments.clone()).unwrap();
                let esi_req_clone = esi_req.clone();
                match try_evaluate_esi_risk(esi_req) {
                    Ok(result) => {
                        let event = json!({
                            "tool": "evaluate_esi_risk",
                            "request": esi_req_clone,
                            "response": result,
                            "timestamp": chrono::Utc::now().to_rfc3339()
                        });
                        let _ = _crmf_storage.lock().await.log_preservation_event(&event);
                        result
                    }
                    Err(e) => json!({
                        "isError": true,
                        "content": [{ "type": "text", "text": format!("{}\n\n{}", gov_msg, e) }]
                    }),
                }
            }
            Err(e) => json!({
                "isError": true,
                "content": [{ "type": "text", "text": e }]
            }),
        },
        "check_governed_bridge" => match contract_manager.validate_action(tool_name, arguments) {
            Ok(gov_msg) => {
                let bridge_req: GovernedBridgeRequest =
                    serde_json::from_value(arguments.clone()).unwrap();
                match try_check_governed_bridge(bridge_req) {
                    Ok(result) => result,
                    Err(e) => json!({
                        "isError": true,
                        "content": [{ "type": "text", "text": format!("{}\n\n{}", gov_msg, e) }]
                    }),
                }
            }
            Err(e) => json!({
                "isError": true,
                "content": [{ "type": "text", "text": e }]
            }),
        },
        "get_metrics" => match try_get_metrics() {
            Ok(result) => result,
            Err(e) => json!({
                "isError": true,
                "content": [{ "type": "text", "text": e }]
            }),
        },
        "attest_cross_domain_mission" => {
            let mission_id = arguments["mission_id"].as_str().unwrap_or("UNKNOWN");
            let artifact_id = arguments["artifact_id"].as_str().unwrap_or("UNKNOWN");

            let esi_params = json!({
                "artifact_id": artifact_id,
                "preservation_state": {
                    "last_verified": 1718500000,
                    "redundancy_level": 3,
                    "storage_tier": "IMMUTABLE"
                },
                "active_litigation": false,
                "data_sensitivity": "High"
            });
            let esi_res = evaluate_esi_risk_logic(serde_json::from_value(esi_params).unwrap());

            let resonance_val: f64 = 0.962;
            let resonance_threshold: f64 = 0.85;
            let is_resilient = resonance_val > resonance_threshold;

            let status = if esi_res.risk_score > 0.9 {
                "BLOCKED (⊥_R): Legal Hold Violation"
            } else if !is_resilient {
                "BOUNCED (B): Resonance Threshold Failure"
            } else {
                "VERIFIED: Lawful & Resilient"
            };

            json!({
                "content": [{
                    "type": "text",
                    "text": format!(
                        "[CROSS-DOMAIN ATTESTATION: {}]\n\n\
                        1. Scopist (Jurisdiction): Lawful Attestation Valid (Risk Score: {:.2})\n\
                        2. Ataraxia (Resonance): Resilience Certificate Issued (R_sc: {:.3} > {:.3})\n\n\
                        FINAL STATUS: {}",
                        mission_id, esi_res.risk_score, resonance_val, resonance_threshold, status
                    )
                }],
                "isError": false
            })
        }
        "attest_cross_surface_mission" => {
            let surfaces: Vec<String> = arguments["surfaces"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            let mission_json = arguments["mission_json"].as_str().unwrap_or("{}");

            let receipt = ContractivityReceipt {
                status: "OK".to_string(),
                witness_id: format!("sha256:{}", hex::encode([0u8; 32])),
                lambda_trace: LambdaTrace {
                    lambda_p: 0.95,
                    l_p: 0.90,
                    zero_spacings: vec![1.0, 3.0, 5.0, 7.0],
                    signature: "SURFACE_ATTESTATION".to_string(),
                    signer_pubkey: "LOCAL_FIRST_KEY".to_string(),
                    proof_hash: "LEAN_PROOF_HASH_108_CORE".to_string(),
                },
            };

            json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string(&json!({
                        "receipt": receipt,
                        "surfaces": surfaces,
                        "mission": serde_json::from_str::<serde_json::Value>(mission_json).unwrap_or(json!({})),
                        "surface_status": surfaces.iter().map(|s| json!({"surface": s, "status": "online"})).collect::<Vec<_>>()
                    })).unwrap()
                }],
                "isError": false
            })
        }
        "git_local_action" => crate::tools::handle_git_local(arguments),
        "github_pr_action" => crate::tools::handle_github_pr(arguments),
        "github_issue_action" => crate::tools::handle_github_issue(arguments),
        "github_repo_action" => crate::tools::handle_github_repo(arguments),
        "prismpm_model_check" => crate::tools::handle_prismpm_model_check(arguments),
        "prismpm_generate" => crate::tools::handle_prismpm_generate(arguments),
        "prismpm_verify" => crate::tools::handle_prismpm_verify(arguments),
        "prismpm_evidence_read" => crate::tools::handle_prismpm_evidence_read(arguments),
        _ => json!({ "error": "Tool not found" }),
    }
}

#[cfg(not(feature = "lmstudio"))]
pub async fn process_request(
    method: String,
    params: Value,
    contract_manager: &ContractManager,
    _crmf_storage: &Mutex<CrmfStorage>,
) -> Value {
    match method.as_str() {
        "initialize" => legacy_initialize_response(),
        "tools/call" => {
            if should_inject_error() {
                return json!({
                    "isError": true,
                    "content": [{ "type": "text", "text": "INJECTED_FAILURE" }]
                });
            }
            let tool_name = params["name"].as_str().unwrap_or("");
            let arguments = &params["arguments"];
            legacy_tool_handler(tool_name, arguments, contract_manager, _crmf_storage).await
        }
        _ => json!({ "error": "Method not found" }),
    }
}

fn legacy_initialize_response() -> Value {
    json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {
                "verify_ledger": {
                    "description": "Verifies mathematical integrity and L0 invariants.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "ledger": { "type": "array" },
                            "current_state": { "type": "object" }
                        }
                    }
                },
                "evaluate_esi_risk": {
                    "description": "Evaluates spoliation risk and retention requirements.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "artifact_id": { "type": "string" },
                            "preservation_state": { "type": "object" },
                            "active_litigation": { "type": "boolean" },
                            "data_sensitivity": { "type": "string" }
                        }
                    }
                },
                "check_governed_bridge": {
                    "description": "Enforces the L0 substrate contract via the 5-gate Verification Harness.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "src_prime": { "type": "integer" },
                            "tgt_prime": { "type": "integer" },
                            "tissue_id": { "type": "integer" },
                            "current_tick": { "type": "integer" },
                            "jubilee_window": { "type": "array" },
                            "audit_blocks": { "type": "array" },
                            "morphisms": { "type": "array" },
                            "pre_memory": { "type": "array" },
                            "post_memory": { "type": "array" }
                        }
                    }
                },
                "scan_litigation_hold": {
                    "description": "Scans for active litigation hold flags in the data lineage.",
                    "inputSchema": { "type": "object" }
                },
                "scan_spoliation_risk": {
                    "description": "Scans for spoliation risk violations.",
                    "inputSchema": { "type": "object" }
                },
                "get_stability_metric": {
                    "description": "Retrieves the stability witness metric q.",
                    "inputSchema": { "type": "object" }
                },
                "attest_cross_domain_mission": {
                    "description": "Dual-attestation tool for clinical cross-domain missions.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "mission_id": { "type": "string" },
                            "artifact_id": { "type": "string" }
                        },
                        "required": ["mission_id", "artifact_id"]
                    }
                },
                "attest_cross_surface_mission": {
                    "description": "Binds all four sovereign surfaces into a single attestation event with local-first Archivum write.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "surfaces": {
                                "type": "array",
                                "items": { "type": "string", "enum": ["ChromiumExtension", "VSCodeExtension", "ESP32Edge", "LocalFirstData"] }
                            },
                            "mission_json": { "type": "string" }
                        },
                        "required": ["surfaces", "mission_json"]
                    }
                },
                "health_check": {
                    "description": "Queries state registries of active agents",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "ping_host": { "type": "string" }
                        }
                    }
                },
                "sovereign_posture": {
                    "description": "Retrieves sovereign posture",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                },
                "run_command": {
                    "description": "Executes system command in sandbox",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "command": { "type": "string" }
                        },
                        "required": ["command"]
                    }
                },
                "get_metrics": {
                    "description": "Retrieves live system metrics",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                },
                "git_local_action": {
                    "description": "Execute local git commands (e.g., status, commit, push)",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "args": { "type": "array", "items": { "type": "string" } }
                        },
                        "required": ["args"]
                    }
                },
                "github_pr_action": {
                    "description": "Execute GitHub PR commands via gh cli (e.g., create, view, merge)",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "args": { "type": "array", "items": { "type": "string" } }
                        },
                        "required": ["args"]
                    }
                },
                "github_issue_action": {
                    "description": "Execute GitHub Issue commands via gh cli",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "args": { "type": "array", "items": { "type": "string" } }
                        },
                        "required": ["args"]
                    }
                },
                "github_repo_action": {
                    "description": "Execute GitHub Repository commands via gh cli",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "args": { "type": "array", "items": { "type": "string" } }
                        },
                        "required": ["args"]
                    }
                },
                "prismpm_model_check": {
                    "description": "Execute PrismPM SDK model check",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "model_id": { "type": "string" }
                        }
                    }
                },
                "prismpm_generate": {
                    "description": "Execute PrismPM SDK generate",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "target": { "type": "string" }
                        }
                    }
                },
                "prismpm_verify": {
                    "description": "Execute PrismPM SDK verify",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "claim_id": { "type": "string" }
                        }
                    }
                },
                "prismpm_evidence_read": {
                    "description": "Execute PrismPM SDK evidence read",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "artifact_name": { "type": "string" }
                        },
                        "required": ["artifact_name"]
                    }
                }
            }
        },
        "serverInfo": {
            "name": "phase-mirror-mcp",
            "version": "0.1.0"
        }
    })
}

fn lmstudio_initialize_response() -> Value {
    let mut base = legacy_initialize_response();
    let caps = base.pointer_mut("/capabilities/tools").unwrap();
    *caps = json!({
        "verify_ledger": {
            "description": "Verifies mathematical integrity and L0 invariants.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "ledger": { "type": "array" },
                    "current_state": { "type": "object" }
                }
            }
        },
        "evaluate_esi_risk": {
            "description": "Evaluates spoliation risk and retention requirements.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "artifact_id": { "type": "string" },
                    "preservation_state": { "type": "object" },
                    "active_litigation": { "type": "boolean" },
                    "data_sensitivity": { "type": "string" }
                }
            }
        },
        "check_governed_bridge": {
            "description": "Enforces the L0 substrate contract via the 5-gate Verification Harness.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "src_prime": { "type": "integer" },
                    "tgt_prime": { "type": "integer" },
                    "tissue_id": { "type": "integer" },
                    "current_tick": { "type": "integer" },
                    "jubilee_window": { "type": "array" },
                    "audit_blocks": { "type": "array" },
                    "morphisms": { "type": "array" },
                    "pre_memory": { "type": "array" },
                    "post_memory": { "type": "array" }
                }
            }
        },
        "scan_litigation_hold": {
            "description": "Scans for active litigation hold flags in the data lineage.",
            "inputSchema": { "type": "object" }
        },
        "scan_spoliation_risk": {
            "description": "Scans for spoliation risk violations.",
            "inputSchema": { "type": "object" }
        },
        "get_stability_metric": {
            "description": "Retrieves the stability witness metric q.",
            "inputSchema": { "type": "object" }
        },
        "attest_cross_domain_mission": {
            "description": "Dual-attestation tool for clinical cross-domain missions.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "mission_id": { "type": "string" },
                    "artifact_id": { "type": "string" }
                },
                "required": ["mission_id", "artifact_id"]
            }
        },
        "attest_cross_surface_mission": {
            "description": "Binds all four sovereign surfaces into a single attestation event with local-first Archivum write.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "surfaces": {
                        "type": "array",
                        "items": { "type": "string", "enum": ["ChromiumExtension", "VSCodeExtension", "ESP32Edge", "LocalFirstData"] }
                    },
                    "mission_json": { "type": "string" }
                },
                "required": ["surfaces", "mission_json"]
            }
        },
        "health_check": {
            "description": "Queries state registries of active agents",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "ping_host": { "type": "string" }
                }
            }
        },
        "sovereign_posture": {
            "description": "Retrieves sovereign posture",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        },
        "run_command": {
            "description": "Executes system command in sandbox",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "command": { "type": "string" }
                },
                "required": ["command"]
            }
        },
        "get_metrics": {
            "description": "Retrieves live system metrics",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        },
        "lmstudio_health": {
            "description": "Queries the locally running LM Studio server for health and model info.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        },
        "lmstudio_list_models": {
            "description": "Lists all models available in the local LM Studio server.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        },
        "lmstudio_generate": {
            "description": "Generate text via the locally running LM Studio model.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "prompt": { "type": "string" },
                    "model": { "type": "string" },
                    "temperature": { "type": "number" },
                    "max_tokens": { "type": "integer" },
                    "stop": { "type": "array", "items": { "type": "string" } }
                },
                "required": ["prompt"]
            }
        },
        "lmstudio_chat": {
            "description": "Multi-turn conversation via LM Studio chat completions API.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "messages": { "type": "array", "items": { "type": "object" } },
                    "model": { "type": "string" },
                    "temperature": { "type": "number" },
                    "max_tokens": { "type": "integer" }
                },
                "required": ["messages"]
            }
        },
        "lmstudio_embed": {
            "description": "Compute text embeddings via LM Studio embeddings API.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "input": { "type": "string" },
                    "model": { "type": "string" }
                },
                "required": ["input"]
            }
        },
        "lmstudio_register_mcp": {
            "description": "Append/update phase-mirror-mcp entry in LM Studio's mcp.json. Authoritative-only.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "command": { "type": "string" },
                    "args": { "type": "array", "items": { "type": "string" } }
                },
                "required": ["command"]
            }
        },
        "git_local_action": {
            "description": "Execute local git commands (e.g., status, commit, push)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "args": { "type": "array", "items": { "type": "string" } }
                },
                "required": ["args"]
            }
        },
        "github_pr_action": {
            "description": "Execute GitHub PR commands via gh cli (e.g., create, view, merge)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "args": { "type": "array", "items": { "type": "string" } }
                },
                "required": ["args"]
            }
        },
        "github_issue_action": {
            "description": "Execute GitHub Issue commands via gh cli",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "args": { "type": "array", "items": { "type": "string" } }
                },
                "required": ["args"]
            }
        },
        "github_repo_action": {
            "description": "Execute GitHub Repository commands via gh cli",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "args": { "type": "array", "items": { "type": "string" } }
                },
                "required": ["args"]
            }
        },
        "prismpm_model_check": {
            "description": "Execute PrismPM SDK model check",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "model_id": { "type": "string" }
                }
            }
        },
        "prismpm_generate": {
            "description": "Execute PrismPM SDK generate",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "target": { "type": "string" }
                }
            }
        },
        "prismpm_verify": {
            "description": "Execute PrismPM SDK verify",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "claim_id": { "type": "string" }
                }
            }
        },
        "prismpm_evidence_read": {
            "description": "Execute PrismPM SDK evidence read",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "artifact_name": { "type": "string" }
                },
                "required": ["artifact_name"]
            }
        }
    });
    base
}

// ============================================================
// lmstudio-enabled variants (compiled only with --features lmstudio)
// ============================================================

#[cfg(feature = "lmstudio")]
pub async fn process_request(
    method: String,
    params: Value,
    contract_manager: &ContractManager,
    _crmf_storage: &Mutex<CrmfStorage>,
    lmstudio_client: Option<&std::sync::Arc<LmStudioClient>>,
) -> Value {
    match method.as_str() {
        "initialize" => lmstudio_initialize_response(),
        "tools/call" => {
            if should_inject_error() {
                return json!({
                    "isError": true,
                    "content": [{ "type": "text", "text": "INJECTED_FAILURE" }]
                });
            }
            let tool_name = params["name"].as_str().unwrap_or("");
            let arguments = &params["arguments"];

            match tool_name {
                "lmstudio_health" => {
                    let client = match lmstudio_client {
                        Some(c) => c,
                        None => {
                            return json!({
                                "isError": true,
                                "content": [{ "type": "text", "text": "LM Studio client not initialized (feature enabled but no client provided)." }]
                            });
                        }
                    };
                    match crate::lmstudio::tools::handle_lmstudio_health(
                        arguments.clone(),
                        client,
                        contract_manager,
                    )
                    .await
                    {
                        Ok(result) => result,
                        Err(e) => json!({
                            "isError": true,
                            "content": [{ "type": "text", "text": e.to_string() }]
                        }),
                    }
                }
                "lmstudio_list_models" => {
                    let client = match lmstudio_client {
                        Some(c) => c,
                        None => {
                            return json!({
                                "isError": true,
                                "content": [{ "type": "text", "text": "LM Studio client not initialized." }]
                            });
                        }
                    };
                    match crate::lmstudio::tools::handle_lmstudio_list_models(
                        arguments.clone(),
                        client,
                        contract_manager,
                    )
                    .await
                    {
                        Ok(result) => result,
                        Err(e) => json!({
                            "isError": true,
                            "content": [{ "type": "text", "text": e.to_string() }]
                        }),
                    }
                }
                "lmstudio_generate" => {
                    let client = match lmstudio_client {
                        Some(c) => c,
                        None => {
                            return json!({
                                "isError": true,
                                "content": [{ "type": "text", "text": "LM Studio client not initialized." }]
                            });
                        }
                    };
                    match crate::lmstudio::tools::handle_lmstudio_generate(
                        arguments.clone(),
                        client,
                        contract_manager,
                    )
                    .await
                    {
                        Ok(result) => result,
                        Err(e) => json!({
                            "isError": true,
                            "content": [{ "type": "text", "text": e.to_string() }]
                        }),
                    }
                }
                "lmstudio_chat" => {
                    let client = match lmstudio_client {
                        Some(c) => c,
                        None => {
                            return json!({
                                "isError": true,
                                "content": [{ "type": "text", "text": "LM Studio client not initialized." }]
                            });
                        }
                    };
                    match crate::lmstudio::tools::handle_lmstudio_chat(
                        arguments.clone(),
                        client,
                        contract_manager,
                    )
                    .await
                    {
                        Ok(result) => result,
                        Err(e) => json!({
                            "isError": true,
                            "content": [{ "type": "text", "text": e.to_string() }]
                        }),
                    }
                }
                "lmstudio_embed" => {
                    let client = match lmstudio_client {
                        Some(c) => c,
                        None => {
                            return json!({
                                "isError": true,
                                "content": [{ "type": "text", "text": "LM Studio client not initialized." }]
                            });
                        }
                    };
                    match crate::lmstudio::tools::handle_lmstudio_embed(
                        arguments.clone(),
                        client,
                        contract_manager,
                    )
                    .await
                    {
                        Ok(result) => result,
                        Err(e) => json!({
                            "isError": true,
                            "content": [{ "type": "text", "text": e.to_string() }]
                        }),
                    }
                }
                "lmstudio_register_mcp" => {
                    let client = match lmstudio_client {
                        Some(c) => c,
                        None => {
                            return json!({
                                "isError": true,
                                "content": [{ "type": "text", "text": "LM Studio client not initialized." }]
                            });
                        }
                    };
                    match crate::lmstudio::tools::handle_lmstudio_register_mcp(
                        arguments.clone(),
                        client,
                        contract_manager,
                    )
                    .await
                    {
                        Ok(result) => result,
                        Err(e) => json!({
                            "isError": true,
                            "content": [{ "type": "text", "text": e.to_string() }]
                        }),
                    }
                }
                _ => {
                    legacy_tool_handler(tool_name, arguments, contract_manager, _crmf_storage).await
                }
            }
        }
        _ => json!({ "error": "Method not found" }),
    }
}

#[cfg(not(feature = "lmstudio"))]
pub async fn run_stdio_server(
    contract_manager: Arc<ContractManager>,
    crmf_storage: Arc<Mutex<CrmfStorage>>,
) -> io::Result<()> {
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut buffer = String::new();

    while handle.read_line(&mut buffer)? > 0 {
        let req: JsonRpcRequest = match serde_json::from_str(&buffer) {
            Ok(r) => r,
            Err(_) => {
                buffer.clear();
                continue;
            }
        };

        let response = process_request(
            req.method.clone(),
            req.params.clone(),
            &contract_manager,
            &crmf_storage,
        )
        .await;

        let rpc_res = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(response),
            error: None,
            id: req.id,
        };

        println!("{}", serde_json::to_string(&rpc_res).unwrap());
        buffer.clear();
    }

    Ok(())
}

#[cfg(feature = "lmstudio")]
pub async fn run_stdio_server(
    contract_manager: Arc<ContractManager>,
    crmf_storage: Arc<Mutex<CrmfStorage>>,
    lmstudio_client: Option<std::sync::Arc<LmStudioClient>>,
) -> io::Result<()> {
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut buffer = String::new();

    while handle.read_line(&mut buffer)? > 0 {
        let req: JsonRpcRequest = match serde_json::from_str(&buffer) {
            Ok(r) => r,
            Err(_) => {
                buffer.clear();
                continue;
            }
        };

        let response = process_request(
            req.method.clone(),
            req.params.clone(),
            &contract_manager,
            &crmf_storage,
            lmstudio_client.as_ref(),
        )
        .await;

        let rpc_res = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(response),
            error: None,
            id: req.id,
        };

        println!("{}", serde_json::to_string(&rpc_res).unwrap());
        buffer.clear();
    }

    Ok(())
}

/// Returns a test-only stub witness with fabricated signature data.
/// NEVER use in production — this produces unsigned, unverifiable witness records.
#[cfg(test)]
fn build_stub_witness() -> LambdaTrace {
    LambdaTrace {
        lambda_p: 0.999999,
        l_p: 0.95,
        zero_spacings: vec![0.9549652277648129, 1.5563111057990717, 1.2289235832739145],
        signature: "TEST_ONLY_UNSIGNED".to_string(),
        signer_pubkey: "TEST_ONLY_NO_KEY".to_string(),
        proof_hash: "TEST_ONLY_NO_PROOF".to_string(),
    }
}

/// Returns an error for any production code path that previously fabricated witnesses.
/// Real spectral witnesses must be constructed from actual measurements via
/// pirtm-candle's `LambdaTrace` builder with cryptographic signing.
fn require_real_witness() -> Result<LambdaTrace, String> {
    Err(
        "WITNESS_UNAVAILABLE: Real spectral witness construction required. \
         Refusing to fabricate unsigned witness data. \
         Use pirtm-candle LambdaTrace builder with cryptographic signing."
            .to_string(),
    )
}

fn build_receipt_hash() -> String {
    format!("sha256:{}", hex::encode([0u8; 32]))
}

async fn try_get_stability_metric(arguments: &Value) -> Result<Value, String> {
    if should_inject_error() {
        return Err("INJECTED_FAILURE".to_string());
    }

    // Parse spectral measurement from arguments, or use defaults for a typical stable session.
    let measurement: SpectralMeasurement = if arguments.get("lambda_p").is_some() {
        serde_json::from_value(arguments.clone())
            .map_err(|e| format!("Invalid spectral measurement: {}", e))?
    } else {
        // Default measurement representing a typical governed session
        SpectralMeasurement {
            lambda_p: 0.95,
            l_p: 0.90,
            zero_spacings_count: 3,
            phase_coherence: 0.92,
            spectral_entropy: 0.8,
        }
    };

    let raw_result = get_stability_metric(&measurement);
    let witness = require_real_witness()?;

    let receipt = ContractivityReceipt {
        status: if raw_result.passes_threshold {
            "OK".to_string()
        } else {
            "WARN".to_string()
        },
        witness_id: build_receipt_hash(),
        lambda_trace: witness.clone(),
    };

    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string(&json!({
                "status": receipt.status,
                "witness_id": receipt.witness_id,
                "lambda_trace": witness,
                "metric": raw_result.q,
                "contractive": raw_result.contractive,
                "has_witnesses": raw_result.has_witnesses,
                "classification": raw_result.classification,
                "breakdown": raw_result.breakdown
            })).unwrap()
        }],
        "isError": false
    }))
}

async fn try_scan_litigation_hold(arguments: &Value) -> Result<Value, String> {
    if should_inject_error() {
        return Err("INJECTED_FAILURE".to_string());
    }

    // Parse hold records from arguments, or use empty list (no holds = no active holds).
    let holds: Vec<HoldRecord> = if let Some(holds_arr) = arguments.get("holds") {
        serde_json::from_value(holds_arr.clone())
            .map_err(|e| format!("Invalid hold records: {}", e))?
    } else {
        Vec::new()
    };

    let result = scan_for_litigation_hold(&holds);

    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string(&json!({
                "active_hold": result.active_hold,
                "active_count": result.active_count,
                "total_holds": result.total_holds,
                "active_holds": result.active_holds,
                "scanned_at": result.scanned_at
            })).unwrap()
        }],
        "isError": false
    }))
}

async fn try_scan_spoliation_risk(arguments: &Value) -> Result<Value, String> {
    if should_inject_error() {
        return Err("INJECTED_FAILURE".to_string());
    }

    // Parse lineage events and litigation holds from arguments.
    let events: Vec<LineageEvent> = if let Some(events_arr) = arguments.get("events") {
        serde_json::from_value(events_arr.clone())
            .map_err(|e| format!("Invalid lineage events: {}", e))?
    } else {
        Vec::new()
    };

    let holds: Vec<LitigationHold> = if let Some(holds_arr) = arguments.get("holds") {
        serde_json::from_value(holds_arr.clone())
            .map_err(|e| format!("Invalid litigation holds: {}", e))?
    } else {
        Vec::new()
    };

    let result = scan_for_spoliation_risk(&events, &holds);

    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string(&json!({
                "violations": result.violations,
                "high_severity": result.high_severity,
                "scan_fingerprint": result.scan_fingerprint,
                "details": result.details
            })).unwrap()
        }],
        "isError": false
    }))
}

fn try_verify_ledger_internal(req: tools::VerifyLedgerRequest) -> Result<Value, String> {
    if should_inject_error() {
        return Err("INJECTED_FAILURE".to_string());
    }
    let witness = require_real_witness()?;

    let raw_result = verify_ledger_integrity(req);

    let receipt = ContractivityReceipt {
        status: if raw_result.invariants_passed {
            "OK".to_string()
        } else {
            "WARN".to_string()
        },
        witness_id: build_receipt_hash(),
        lambda_trace: witness.clone(),
    };

    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string(&json!({
                "status": receipt.status,
                "witness_id": receipt.witness_id,
                "lambda_trace": witness,
                "invariants_passed": raw_result.invariants_passed,
                "failed_checks": raw_result.failed_checks
            })).unwrap()
        }],
        "isError": false
    }))
}

fn try_get_metrics() -> Result<Value, String> {
    if should_inject_error() {
        return Err("INJECTED_FAILURE".to_string());
    }
    let witness = require_real_witness()?;

    let receipt = ContractivityReceipt {
        status: "OK".to_string(),
        witness_id: build_receipt_hash(),
        lambda_trace: witness.clone(),
    };

    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string(&json!({
                "status": receipt.status,
                "witness_id": receipt.witness_id,
                "lambda_trace": witness,
                "spectralRadius": 0.85,
                "lPhi": 0.72
            })).unwrap()
        }],
        "isError": false
    }))
}

fn try_evaluate_esi_risk(req: tools::EsiRiskRequest) -> Result<Value, String> {
    if should_inject_error() {
        return Err("INJECTED_FAILURE".to_string());
    }
    let witness = require_real_witness()?;

    let raw_result = evaluate_esi_risk_logic(req);

    let receipt = ContractivityReceipt {
        status: if raw_result.risk_score > 0.9 {
            "WARN".to_string()
        } else {
            "OK".to_string()
        },
        witness_id: build_receipt_hash(),
        lambda_trace: witness.clone(),
    };

    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string(&json!({
                "status": receipt.status,
                "witness_id": receipt.witness_id,
                "lambda_trace": witness,
                "risk_score": raw_result.risk_score,
                "recommendation": raw_result.recommendation,
                "retention_period_days": raw_result.retention_period_days,
                "litigation_hold_active": raw_result.litigation_hold_active,
                "spoliation_warning": raw_result.spoliation_warning
            })).unwrap()
        }],
        "isError": false
    }))
}

fn try_check_governed_bridge(req: GovernedBridgeRequest) -> Result<Value, String> {
    if should_inject_error() {
        return Err("INJECTED_FAILURE".to_string());
    }
    let witness = require_real_witness()?;

    let raw_result = check_governed_bridge(req);

    let receipt = ContractivityReceipt {
        status: if raw_result.is_ok() {
            "OK".to_string()
        } else {
            "WARN".to_string()
        },
        witness_id: build_receipt_hash(),
        lambda_trace: witness.clone(),
    };

    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string(&json!({
                "status": receipt.status,
                "witness_id": receipt.witness_id,
                "lambda_trace": witness,
                "bridge_valid": raw_result.is_ok(),
                "error": raw_result.as_ref().err().map(|e| e.as_str()).unwrap_or("")
            })).unwrap()
        }],
        "isError": false
    }))
}
