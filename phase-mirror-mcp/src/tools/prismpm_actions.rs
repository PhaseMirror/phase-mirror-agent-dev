use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::process::Command;
use std::path::Path;

#[derive(Deserialize)]
pub struct ModelCheckArgs {
    pub model_id: Option<String>,
}

#[derive(Deserialize)]
pub struct GenerateArgs {
    pub target: Option<String>,
}

#[derive(Deserialize)]
pub struct VerifyArgs {
    pub claim_id: Option<String>,
}

#[derive(Deserialize)]
pub struct EvidenceReadArgs {
    pub artifact_name: String,
}

#[derive(Deserialize)]
struct PrismpmLock {
    pub sdk_image: String,
    pub sdk_version: String,
}

fn get_prismpm_binary() -> Result<String, String> {
    let lock_path = Path::new("prismpm.lock");
    if !lock_path.exists() {
        return Err("prismpm.lock not found. Oracle boundary requires a pinned SDK lock.".into());
    }

    let lock_content = std::fs::read_to_string(lock_path)
        .map_err(|e| format!("Failed to read prismpm.lock: {}", e))?;

    let parsed: PrismpmLock = serde_json::from_str(&lock_content)
        .map_err(|e| format!("prismpm.lock is invalid or missing required fields (sdk_image, sdk_version): {}", e))?;

    if parsed.sdk_image.is_empty() {
        return Err("prismpm.lock sdk_image digest is empty.".into());
    }

    // Relying on the devcontainer environment to map the "prismpm" binary
    // from the image to the path, but bounded by the parsed validity of the lock.
    Ok("prismpm".to_string())
}

pub fn handle_prismpm_model_check(args: &Value) -> Value {
    let bin = match get_prismpm_binary() {
        Ok(b) => b,
        Err(e) => return json!({ "isError": true, "content": [{ "type": "text", "text": e }] }),
    };

    let req: Result<ModelCheckArgs, _> = serde_json::from_value(args.clone());
    let mut cmd_args = vec!["model", "check"];
    
    if let Ok(r) = req {
        if let Some(ref m) = r.model_id {
            cmd_args.push(m);
        }
    }

    execute_cli(&bin, &cmd_args)
}

pub fn handle_prismpm_generate(args: &Value) -> Value {
    let bin = match get_prismpm_binary() {
        Ok(b) => b,
        Err(e) => return json!({ "isError": true, "content": [{ "type": "text", "text": e }] }),
    };

    let req: Result<GenerateArgs, _> = serde_json::from_value(args.clone());
    let mut cmd_args = vec!["generate"];
    
    if let Ok(r) = req {
        if let Some(ref t) = r.target {
            cmd_args.push(t);
        }
    }

    execute_cli(&bin, &cmd_args)
}

pub fn handle_prismpm_verify(args: &Value) -> Value {
    let bin = match get_prismpm_binary() {
        Ok(b) => b,
        Err(e) => return json!({ "isError": true, "content": [{ "type": "text", "text": e }] }),
    };

    let req: Result<VerifyArgs, _> = serde_json::from_value(args.clone());
    let mut cmd_args = vec!["verify"];
    
    if let Ok(r) = req {
        if let Some(ref c) = r.claim_id {
            cmd_args.push(c);
        }
    }

    execute_cli(&bin, &cmd_args)
}

pub fn handle_prismpm_evidence_read(args: &Value) -> Value {
    let bin = match get_prismpm_binary() {
        Ok(b) => b,
        Err(e) => return json!({ "isError": true, "content": [{ "type": "text", "text": e }] }),
    };

    let req: Result<EvidenceReadArgs, _> = serde_json::from_value(args.clone());
    let artifact = match req {
        Ok(r) => r.artifact_name,
        Err(_) => return json!({ "isError": true, "content": [{ "type": "text", "text": "Missing artifact_name" }] }),
    };

    // Strict validation: cannot read outside evidence directory
    if artifact.contains('/') || artifact.contains('\\') || artifact.contains("..") {
        return json!({ "isError": true, "content": [{ "type": "text", "text": "Access denied: evidence_read must target a filename strictly within the locked evidence directory." }] });
    }

    let cmd_args = vec!["evidence", "read", &artifact];
    execute_cli(&bin, &cmd_args)
}

fn execute_cli(bin: &str, args: &[&str]) -> Value {
    match Command::new(bin).args(args).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            if output.status.success() {
                json!({
                    "isError": false,
                    "content": [{ "type": "text", "text": format!("STDOUT:\n{}\nSTDERR:\n{}", stdout, stderr) }]
                })
            } else {
                json!({
                    "isError": true,
                    "content": [{ "type": "text", "text": format!("Command failed.\nSTDOUT:\n{}\nSTDERR:\n{}", stdout, stderr) }]
                })
            }
        }
        Err(e) => json!({
            "isError": true,
            "content": [{ "type": "text", "text": format!("Failed to execute {}: {}", bin, e) }]
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_evidence_read_rejects_traversal() {
        let args = json!({
            "artifact_name": "../secret.txt"
        });
        
        let result = handle_prismpm_evidence_read(&args);
        
        assert_eq!(result["isError"], true);
        assert!(result["content"][0]["text"].as_str().unwrap().contains("Access denied"));
    }

    #[test]
    fn test_evidence_read_rejects_absolute_path() {
        let args = json!({
            "artifact_name": "/etc/passwd"
        });
        
        let result = handle_prismpm_evidence_read(&args);
        
        assert_eq!(result["isError"], true);
        assert!(result["content"][0]["text"].as_str().unwrap().contains("Access denied"));
    }

    #[test]
    fn test_fails_without_valid_lock() {
        let args = json!({ "model_id": "test" });
        let result = handle_prismpm_model_check(&args);
        
        assert_eq!(result["isError"], true);
        let msg = result["content"][0]["text"].as_str().unwrap();
        assert!(msg.contains("prismpm.lock") || msg.contains("invalid or missing"));
    }
}
