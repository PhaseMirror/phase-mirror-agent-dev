use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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
struct InventoryItem {
    id: String,
    kind: String,
    digest: String,
}

#[derive(Deserialize)]
struct Platform {
    inventory: Vec<InventoryItem>,
}

#[derive(Deserialize)]
struct PrismpmLock {
    schema: String,
    sdk_image: String,
    sdk_version: String,
    platforms: Vec<Platform>,
}

#[derive(Debug)]
struct ResolvedLock {
    prismpm_digests: Vec<String>,
}

fn load_lock(lock_path: &Path) -> Result<ResolvedLock, String> {
    if !lock_path.is_file() {
        return Err("prismpm.lock not found. Oracle boundary requires a pinned SDK lock.".into());
    }
    let lock_content = fs::read_to_string(lock_path)
        .map_err(|e| format!("Failed to read prismpm.lock: {e}"))?;
    let parsed: PrismpmLock = serde_json::from_str(&lock_content).map_err(|e| {
        format!("prismpm.lock is invalid or missing required fields: {e}")
    })?;
    if parsed.schema != "prismpm/sdk-lock/2" {
        return Err(format!(
            "prismpm.lock schema is {}, expected prismpm/sdk-lock/2",
            parsed.schema
        ));
    }
    if !parsed.sdk_image.contains("@sha256:") {
        return Err("prismpm.lock sdk_image is not digest-pinned".into());
    }
    if parsed.sdk_version.is_empty() {
        return Err("prismpm.lock sdk_version is empty".into());
    }
    let prismpm_digests: Vec<String> = parsed
        .platforms
        .iter()
        .flat_map(|p| p.inventory.iter())
        .filter(|item| item.id == "prismpm" && item.kind == "binary")
        .map(|item| item.digest.clone())
        .filter(|d| d.starts_with("sha256:") && d.len() == "sha256:".len() + 64)
        .collect();
    if prismpm_digests.is_empty() {
        return Err("prismpm.lock names no prismpm binary digest".into());
    }
    Ok(ResolvedLock { prismpm_digests })
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
    let digest = Sha256::digest(bytes);
    Ok(format!("sha256:{}", hex::encode(digest)))
}

/// The lock names an image digest and a binary digest. It does not name a PATH entry.
/// `bin_path` must be an absolute file whose SHA-256 equals a `prismpm` inventory digest.
pub fn resolve_binary(lock_path: &Path, bin_path: &Path) -> Result<PathBuf, String> {
    let lock = load_lock(lock_path)?;
    if !bin_path.is_absolute() {
        return Err("PRISMPM_BIN must be an absolute path. PATH lookup is refused.".into());
    }
    if !bin_path.is_file() {
        return Err(format!(
            "PRISMPM_BIN {} is not a file",
            bin_path.display()
        ));
    }
    let digest = sha256_file(bin_path)?;
    if !lock.prismpm_digests.iter().any(|d| d == &digest) {
        return Err(format!(
            "PRISMPM_BIN digest {digest} is not a prismpm binary named by prismpm.lock"
        ));
    }
    Ok(bin_path.to_path_buf())
}

fn prismpm_binary() -> Result<PathBuf, String> {
    let bin = std::env::var("PRISMPM_BIN").map_err(|_| {
        "PRISMPM_BIN is unset. prismpm.lock names an image digest, not the first prismpm on PATH."
            .to_string()
    })?;
    resolve_binary(Path::new("prismpm.lock"), Path::new(&bin))
}

/// `prismpm/sdk-lock/2` does not name an evidence directory. The digest-pinned
/// environment must. The caller does not.
pub fn resolve_evidence(evidence_dir: &Path, artifact_name: &str) -> Result<PathBuf, String> {
    if artifact_name.is_empty()
        || artifact_name.starts_with('.')
        || !artifact_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
    {
        return Err(
            "Access denied: evidence_read accepts only a single filename in the bound evidence directory."
                .into(),
        );
    }
    if !evidence_dir.is_absolute() || !evidence_dir.is_dir() {
        return Err("PRISMPM_EVIDENCE_DIR must be an absolute directory".into());
    }
    let root = evidence_dir
        .canonicalize()
        .map_err(|e| format!("PRISMPM_EVIDENCE_DIR cannot be canonicalized: {e}"))?;
    let candidate = root.join(artifact_name);
    if !candidate.is_file() {
        return Err(
            "Access denied: artifact is not a file in the bound evidence directory.".into(),
        );
    }
    let canon = candidate
        .canonicalize()
        .map_err(|e| format!("evidence artifact cannot be canonicalized: {e}"))?;
    if !canon.starts_with(&root) {
        return Err(
            "Access denied: evidence_read resolved outside the bound evidence directory.".into(),
        );
    }
    Ok(canon)
}

fn evidence_dir() -> Result<PathBuf, String> {
    let dir = std::env::var("PRISMPM_EVIDENCE_DIR").map_err(|_| {
        "PRISMPM_EVIDENCE_DIR is unset. prismpm/sdk-lock/2 does not name an evidence directory."
            .to_string()
    })?;
    Ok(PathBuf::from(dir))
}

fn closed_token(value: &str, field: &str) -> Result<(), String> {
    if value.is_empty()
        || !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | ':'))
    {
        return Err(format!("{field} is not a closed token"));
    }
    Ok(())
}

pub fn handle_prismpm_model_check(args: &Value) -> Value {
    let bin = match prismpm_binary() {
        Ok(b) => b,
        Err(e) => return error(&e),
    };
    let mut cmd_args = vec!["model".to_string(), "check".to_string()];
    if let Ok(req) = serde_json::from_value::<ModelCheckArgs>(args.clone()) {
        if let Some(model_id) = req.model_id {
            if let Err(e) = closed_token(&model_id, "model_id") {
                return error(&e);
            }
            cmd_args.push(model_id);
        }
    }
    execute_cli(&bin, &cmd_args)
}

pub fn handle_prismpm_generate(args: &Value) -> Value {
    let bin = match prismpm_binary() {
        Ok(b) => b,
        Err(e) => return error(&e),
    };
    let mut cmd_args = vec!["generate".to_string()];
    if let Ok(req) = serde_json::from_value::<GenerateArgs>(args.clone()) {
        if let Some(target) = req.target {
            if let Err(e) = closed_token(&target, "target") {
                return error(&e);
            }
            cmd_args.push(target);
        }
    }
    execute_cli(&bin, &cmd_args)
}

pub fn handle_prismpm_verify(args: &Value) -> Value {
    let bin = match prismpm_binary() {
        Ok(b) => b,
        Err(e) => return error(&e),
    };
    let mut cmd_args = vec!["verify".to_string()];
    if let Ok(req) = serde_json::from_value::<VerifyArgs>(args.clone()) {
        if let Some(claim_id) = req.claim_id {
            if let Err(e) = closed_token(&claim_id, "claim_id") {
                return error(&e);
            }
            cmd_args.push(claim_id);
        }
    }
    execute_cli(&bin, &cmd_args)
}

pub fn handle_prismpm_evidence_read(args: &Value) -> Value {
    let artifact = match serde_json::from_value::<EvidenceReadArgs>(args.clone()) {
        Ok(req) => req.artifact_name,
        Err(_) => return error("Missing artifact_name"),
    };
    let dir = match evidence_dir() {
        Ok(dir) => dir,
        Err(e) => return error(&e),
    };
    if let Err(e) = resolve_evidence(&dir, &artifact) {
        return error(&e);
    }
    let bin = match prismpm_binary() {
        Ok(b) => b,
        Err(e) => return error(&e),
    };
    execute_cli(
        &bin,
        &["evidence".to_string(), "read".to_string(), artifact],
    )
}

fn execute_cli(bin: &Path, args: &[String]) -> Value {
    match Command::new(bin).args(args).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            if output.status.success() {
                json!({
                    "isError": false,
                    "content": [{ "type": "text", "text": format!("STDOUT:\n{stdout}\nSTDERR:\n{stderr}") }]
                })
            } else {
                json!({
                    "isError": true,
                    "content": [{ "type": "text", "text": format!("Command failed.\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}") }]
                })
            }
        }
        Err(e) => error(&format!("Failed to execute {}: {e}", bin.display())),
    }
}

fn error(text: &str) -> Value {
    json!({ "isError": true, "content": [{ "type": "text", "text": text }] })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Write;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "prismpm-actions-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_file(path: &Path, bytes: &[u8]) {
        let mut file = fs::File::create(path).unwrap();
        file.write_all(bytes).unwrap();
    }

    fn lock_naming(digest: &str) -> String {
        format!(
            r#"{{
                "schema": "prismpm/sdk-lock/2",
                "sdk_image": "ghcr.io/example/prismpm@sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                "sdk_version": "0.3.0",
                "platforms": [{{
                    "inventory": [{{
                        "id": "prismpm",
                        "kind": "binary",
                        "digest": "{digest}",
                        "version": "0.3.0"
                    }}]
                }}]
            }}"#
        )
    }

    #[test]
    fn evidence_read_rejects_traversal_before_any_binary() {
        let result = resolve_evidence(Path::new("/tmp"), "../secret.txt");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Access denied"));
    }

    #[test]
    fn evidence_read_rejects_absolute_name() {
        let result = resolve_evidence(Path::new("/tmp"), "/etc/passwd");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Access denied"));
    }

    #[test]
    fn evidence_read_rejects_a_file_outside_the_bound_directory() {
        let root = temp_dir();
        let outside = temp_dir();
        write_file(&outside.join("plant.txt"), b"outside");
        let err = resolve_evidence(&root, "plant.txt").unwrap_err();
        assert!(err.contains("Access denied"));
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(outside);
    }

    #[test]
    fn evidence_read_accepts_only_a_file_inside_the_bound_directory() {
        let root = temp_dir();
        write_file(&root.join("vv-evidence.json"), b"{}");
        let resolved = resolve_evidence(&root, "vv-evidence.json").unwrap();
        assert!(resolved.starts_with(root.canonicalize().unwrap()));
        assert!(resolved.ends_with("vv-evidence.json"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn binary_resolution_refuses_path_and_a_mismatched_digest() {
        let dir = temp_dir();
        let named = dir.join("prismpm-named");
        let plant = dir.join("prismpm-plant");
        write_file(&named, b"named-binary");
        write_file(&plant, b"other-binary");
        let digest = sha256_file(&named).unwrap();
        let lock = dir.join("prismpm.lock");
        write_file(&lock, lock_naming(&digest).as_bytes());

        let err = resolve_binary(&lock, &plant).unwrap_err();
        assert!(err.contains("is not a prismpm binary named by prismpm.lock"));

        let relative = Path::new("prismpm");
        let err = resolve_binary(&lock, relative).unwrap_err();
        assert!(err.contains("PATH lookup is refused"));

        let resolved = resolve_binary(&lock, &named).unwrap();
        assert_eq!(resolved, named);
        assert_ne!(resolved.to_string_lossy(), "prismpm");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn undigested_lock_is_refused() {
        let dir = temp_dir();
        let lock = dir.join("prismpm.lock");
        write_file(
            &lock,
            br#"{"schema":"prismpm/sdk-lock/2","sdk_image":"ghcr.io/example/prismpm:latest","sdk_version":"0.3.0","platforms":[]}"#,
        );
        let err = load_lock(&lock).unwrap_err();
        assert!(err.contains("not digest-pinned"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn handler_fails_without_a_lock_or_binary_binding() {
        let args = json!({ "model_id": "test" });
        let result = handle_prismpm_model_check(&args);
        assert_eq!(result["isError"], true);
        let msg = result["content"][0]["text"].as_str().unwrap();
        assert!(msg.contains("prismpm.lock") || msg.contains("PRISMPM_BIN"));
    }
}
