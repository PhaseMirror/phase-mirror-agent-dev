use crate::archivum::{
    ArchivumLedger, DistributedSyncOracle, MerkleSnapshot, SyncManager, SyncOutcome,
};
use crate::mirror::MirrorCoordinator;
use crate::triple_lock::TripleLockSuite;
use crate::validator::{
    EvaluationContext, GovernanceOutcome, GovernanceTier, InvariantConsistencyOracle, OracleEvent,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;

// --- MCP / JSON-RPC 2.0 Protocol Types (ADR-004) ---

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Value,
    pub id: Value,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub result: Option<Value>,
    pub error: Option<JsonRpcError>,
    pub id: Value,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct McpContent {
    pub r#type: String,
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct McpCallToolResult {
    pub content: Vec<McpContent>,
    #[serde(rename = "isError")]
    pub is_error: bool,
}

// --- Tool-Specific Input Payloads ---

#[derive(Debug, Deserialize)]
pub struct ValidateInvariantsPayload {
    pub event_type: String,
    pub tier: String,
    pub permission_bits: u32,
    pub schema_signature: u32,
    pub expected_schema: u32,
}

#[derive(Debug, Deserialize)]
pub struct ReconcileSyncPayload {
    pub remote_root_hash: String,
    pub remote_leaves: BTreeMap<String, String>,
}

// --- Transport Wrapper Handler ---

pub struct McpTransportWrapper {
    pub oracle: InvariantConsistencyOracle,
    pub sync_oracle: DistributedSyncOracle,
    pub ledger: Arc<TokioMutex<ArchivumLedger>>,
    pub mirror: Arc<MirrorCoordinator>,
    pub triple_lock: Arc<TripleLockSuite>,
}

impl McpTransportWrapper {
    /// Internal constructor for initializing with shared state
    pub fn init_internal(
        sync_oracle: DistributedSyncOracle,
        ledger: Arc<TokioMutex<ArchivumLedger>>,
        mirror: MirrorCoordinator,
        triple_lock: TripleLockSuite,
    ) -> Self {
        Self {
            oracle: InvariantConsistencyOracle,
            sync_oracle,
            ledger,
            mirror: Arc::new(mirror),
            triple_lock: Arc::new(triple_lock),
        }
    }

    pub async fn handle_mcp_call(&self, request_str: &str) -> String {
        let req: JsonRpcRequest = match serde_json::from_str(request_str) {
            Ok(valid_req) => valid_req,
            Err(_) => {
                return self.build_error_response(
                    Value::Null,
                    -32700,
                    "Parse error: Invalid JSON structure",
                );
            }
        };

        let id = req.id.clone();

        if req.method != "tools/call" {
            return self.build_error_response(id, -32601, "Method not found");
        }

        let tool_name = req
            .params
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let arguments = req.params.get("arguments").cloned().unwrap_or(Value::Null);

        // ADR-003: Record the tool execution attempt in Λ-Archivum
        {
            let mut l = self.ledger.lock().await;
            let _ = l.commit_event(
                "tool_invocation",
                tool_name.to_string(),
                request_str.as_bytes(),
            );
        }

        let result = match tool_name {
            "validate_l0_invariants" => self.execute_validate_l0(arguments),
            "reconcile_merkle_sync" => self.execute_reconcile_sync(arguments),
            "reflect_plan" => self.execute_reflect_plan(arguments).await,
            "get_governance_status" => self.execute_get_governance_status().await,
            "triple_lock_verify" => self.execute_triple_lock_verify(arguments).await,
            _ => return self.build_error_response(id, -32602, "Invalid tool name parameter"),
        };

        match result {
            Ok(mcp_res) => {
                let res = JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: Some(serde_json::to_value(mcp_res).unwrap()),
                    error: None,
                    id,
                };
                serde_json::to_string(&res).unwrap()
            }
            Err(err_msg) => {
                let fail_closed_res = McpCallToolResult {
                    content: vec![McpContent {
                        r#type: "text".to_string(),
                        text: format!("FAIL-CLOSED BLOCK: {}", err_msg),
                    }],
                    is_error: true,
                };
                let res = JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: Some(serde_json::to_value(fail_closed_res).unwrap()),
                    error: None,
                    id,
                };
                serde_json::to_string(&res).unwrap()
            }
        }
    }

    fn execute_validate_l0(&self, args: Value) -> Result<McpCallToolResult, &'static str> {
        let payload: ValidateInvariantsPayload = serde_json::from_value(args)
            .map_err(|_| "Failed to parse validate_l0_invariants input arguments")?;

        let event = match payload.event_type.as_str() {
            "pull_request" => OracleEvent::PullRequest,
            "merge_group" => OracleEvent::MergeGroup,
            "drift" => OracleEvent::Drift,
            _ => return Err("Unknown governance event mode"),
        };

        let tier = match payload.tier.as_str() {
            "tier1" => GovernanceTier::Tier1Authoritative,
            "tier2" => GovernanceTier::Tier2Experimental,
            _ => return Err("Unknown governance tier alignment"),
        };

        let ctx = EvaluationContext {
            permission_bits: payload.permission_bits,
            schema_signature: payload.schema_signature,
            expected_schema: payload.expected_schema,
        };

        let outcome = self.oracle.handle_event(event, &ctx, tier);

        let (text_msg, is_error) = match outcome {
            GovernanceOutcome::Allow => (
                "Governance validation verified successfully. Code: ALLOW.",
                false,
            ),
            GovernanceOutcome::Warning(msg) => (msg, false),
            GovernanceOutcome::Block(msg) => (msg, true),
        };

        Ok(McpCallToolResult {
            content: vec![McpContent {
                r#type: "text".to_string(),
                text: text_msg.to_string(),
            }],
            is_error,
        })
    }

    fn execute_reconcile_sync(&self, args: Value) -> Result<McpCallToolResult, &'static str> {
        let payload: ReconcileSyncPayload = serde_json::from_value(args)
            .map_err(|_| "Failed to parse reconcile_merkle_sync input arguments")?;

        let remote_snapshot = MerkleSnapshot {
            root_hash: payload.remote_root_hash,
            leaves: payload.remote_leaves,
        };

        let outcome = self.sync_oracle.reconcile_delta(&remote_snapshot);

        let (text_msg, is_error) = match outcome {
            SyncOutcome::Synchronized => (
                "Distributed synchronization matching perfectly. Nodes coherent.".to_string(),
                false,
            ),
            SyncOutcome::DeltaIdentified(mismatches) => (
                format!(
                    "Sync drift detected. Out of sync elements: {:?}",
                    mismatches
                ),
                false,
            ),
            SyncOutcome::IntegrityFailure(err) => (format!("FAIL-CLOSED BLOCK: {}", err), true),
        };

        Ok(McpCallToolResult {
            content: vec![McpContent {
                r#type: "text".to_string(),
                text: text_msg,
            }],
            is_error,
        })
    }

    async fn execute_reflect_plan(&self, args: Value) -> Result<McpCallToolResult, &'static str> {
        let plan = args
            .get("plan")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'plan' argument for reflect_plan")?;

        match self.mirror.reflect(plan).await {
            Ok(critique_prompt) => Ok(McpCallToolResult {
                content: vec![McpContent {
                    r#type: "text".to_string(),
                    text: critique_prompt,
                }],
                is_error: false,
            }),
            Err(e) => Ok(McpCallToolResult {
                content: vec![McpContent {
                    r#type: "text".to_string(),
                    text: format!("FAIL-CLOSED BLOCK: {}", e),
                }],
                is_error: true,
            }),
        }
    }

    async fn execute_get_governance_status(&self) -> Result<McpCallToolResult, &'static str> {
        let (compliance_rate, velocity_delayed) = {
            let t = self
                .mirror
                .telemetry
                .lock()
                .map_err(|_| "Telemetry lock poisoned")?;
            t.get_system_status()
        };

        let last_archivum_sequence = {
            let l = self.ledger.lock().await;
            l.get_entry_count()
        };

        let pattern_count = self.mirror.policy.get_pattern_count();

        let payload = serde_json::json!({
            "compliance_rate": compliance_rate,
            "is_degraded": velocity_delayed,
            "active_blocklist_size": pattern_count,
            "last_archivum_sequence": last_archivum_sequence,
        });

        Ok(McpCallToolResult {
            content: vec![McpContent {
                r#type: "text".to_string(),
                text: serde_json::to_string_pretty(&payload).unwrap_or_default(),
            }],
            is_error: false,
        })
    }

    async fn execute_triple_lock_verify(
        &self,
        args: Value,
    ) -> Result<McpCallToolResult, &'static str> {
        let mission_id = args
            .get("mission_id")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'mission_id' argument")?;
        let plan = args
            .get("plan")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'plan' argument")?;

        let pb = args
            .get("permission_bits")
            .and_then(|v| v.as_u64())
            .unwrap_or(15) as u32;
        let ss = args
            .get("schema_signature")
            .and_then(|v| v.as_u64())
            .unwrap_or(3) as u32;
        let es = args
            .get("expected_schema")
            .and_then(|v| v.as_u64())
            .unwrap_or(3) as u32;
        let ctx = EvaluationContext {
            permission_bits: pb,
            schema_signature: ss,
            expected_schema: es,
        };

        match self.triple_lock.verify(mission_id, plan, ctx).await {
            Ok(witness) => Ok(McpCallToolResult {
                content: vec![McpContent {
                    r#type: "text".to_string(),
                    text: serde_json::to_string_pretty(&witness).unwrap(),
                }],
                is_error: false,
            }),
            Err(e) => Ok(McpCallToolResult {
                content: vec![McpContent {
                    r#type: "text".to_string(),
                    text: format!("TRIPLE-LOCK BLOCK: {}", e),
                }],
                is_error: true,
            }),
        }
    }

    fn build_error_response(&self, id: Value, code: i32, message: &str) -> String {
        let res = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.to_string(),
            }),
            id,
        };
        serde_json::to_string(&res).unwrap()
    }
}
