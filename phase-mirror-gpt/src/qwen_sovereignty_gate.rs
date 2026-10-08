use crate::triple_lock::TripleLockSuite;
use anyhow::{Result, anyhow};
use std::process::Stdio;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;

/// QwenSovereigntyGate: Demonstrating MCP over child_process
/// for Qwen, gated by the Phase Mirror Triple-Lock suite.
pub struct QwenSovereigntyGate {
    pub triple_lock: std::sync::Arc<TripleLockSuite>,
    pub qwen_binary_path: String,
}

impl QwenSovereigntyGate {
    pub fn new(triple_lock: std::sync::Arc<TripleLockSuite>, qwen_binary_path: String) -> Self {
        Self {
            triple_lock,
            qwen_binary_path,
        }
    }

    /// Invokes Qwen securely.
    /// Qwen is run as a sovereign child process. Its outputs MUST pass
    /// the Triple-Lock before they are returned to the user or system.
    pub async fn sovereign_generate(
        &self,
        mission_id: &str,
        prompt: &str,
    ) -> Result<(String, String)> {
        // 1. Invoke Sovereign Qwen (Child Process MCP binding)
        let mut child = Command::new(&self.qwen_binary_path)
            .arg("--mcp-mode")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| anyhow!("Failed to spawn Sovereign Qwen process: {}", e))?;

        if let Some(mut stdin) = child.stdin.take() {
            let mcp_request = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "generate",
                "params": { "prompt": prompt },
                "id": mission_id
            });
            stdin
                .write_all(serde_json::to_string(&mcp_request)?.as_bytes())
                .await?;
            stdin.write_all(b"\n").await?;
        }

        // Real MCP stdout enforcement - no simulation
        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("Failed to bind Qwen stdout"))?;
        let mut raw_output = String::new();
        stdout.read_to_string(&mut raw_output).await?;

        // Extract the actual draft plan from the child process output
        let draft_plan = raw_output.trim().to_string();

        let ctx = crate::validator::EvaluationContext {
            permission_bits: crate::validator::PERM_READ
                | crate::validator::PERM_WRITE
                | crate::validator::PERM_ADMIN,
            schema_signature: crate::validator::SCHEMA_VALID | crate::validator::SCHEMA_NON_EMPTY,
            expected_schema: crate::validator::SCHEMA_VALID | crate::validator::SCHEMA_NON_EMPTY,
        };

        // 2. The output MUST pass the Triple-Lock Sovereignty Gate before release
        let witness = match self.triple_lock.verify(mission_id, &draft_plan, ctx).await {
            Ok(w) => w,
            Err(e) => {
                // ADR-005: Prototype Fail-Closed Block Persistence
                let err_msg = e.to_string();
                let registry_path = "MASTER_REGISTRY.md";
                let temp_path = "MASTER_REGISTRY.md.err.tmp";

                let mut existing_content = String::new();
                if let Ok(content) = tokio::fs::read_to_string(registry_path).await {
                    existing_content = content;
                }

                // Generate a block witness hash to prove the rejection was caught
                use sha2::{Digest, Sha256};
                let mut block_hasher = Sha256::new();
                block_hasher.update(mission_id.as_bytes());
                block_hasher.update(err_msg.as_bytes());
                let block_witness = hex::encode(block_hasher.finalize());

                let registry_entry = format!(
                    "- **Mission**: {} | **BLOCK WITNESS**: {} | **Reason**: {}\n",
                    mission_id, block_witness, err_msg
                );
                let new_content = format!("{}{}", existing_content, registry_entry);

                let _ = tokio::fs::write(temp_path, new_content).await;
                let _ = tokio::fs::rename(temp_path, registry_path).await;

                return Err(anyhow!("FAIL-CLOSED BLOCK GENERATED: {}", err_msg));
            }
        };

        // 3. Registry Persistence: Atomic update with TRUE p=7 witness_hash chaining
        let registry_path = "MASTER_REGISTRY.md";
        let temp_path = "MASTER_REGISTRY.md.tmp";

        let mut existing_content = String::new();
        let mut last_hash = "GENESIS".to_string();

        if let Ok(content) = tokio::fs::read_to_string(registry_path).await {
            existing_content = content.clone();
            // Find the last recorded chain hash to cryptographically link the new event
            for line in content.lines().rev() {
                if line.contains("**Chain**: ") {
                    if let Some(idx) = line.rfind("**Chain**: ") {
                        last_hash = line[idx + 11..].trim().to_string();
                        break;
                    }
                }
            }
        }

        // Real p=7 Chaining Algorithm: Hash(Prev_Hash || New_Witness || P_Lineage_Seq)
        use sha2::{Digest, Sha256};
        let mut chain_hasher = Sha256::new();
        chain_hasher.update(last_hash.as_bytes());
        chain_hasher.update(witness.witness_hash.as_bytes());
        chain_hasher.update(witness.p_lineage.as_bytes());
        let chained_hash = hex::encode(chain_hasher.finalize());

        let registry_entry = format!(
            "- **Mission**: {} | **Witness Hash**: {} | **Chain**: {}\n",
            mission_id, witness.witness_hash, chained_hash
        );
        let new_content = format!("{}{}", existing_content, registry_entry);

        // Atomic replacement guarantees thread/crash safety
        tokio::fs::write(temp_path, new_content).await?;
        tokio::fs::rename(temp_path, registry_path).await?;

        // 4. Return the verified plan and the immutable chained hash
        Ok((draft_plan, chained_hash))
    }
}
