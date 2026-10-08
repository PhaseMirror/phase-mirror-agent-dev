use anyhow::{bail, Context, Result};
use std::path::Path;

/// Every environment key the agent understands. `.env` files must not smuggle
/// in unrecognized keys (ADR-006 §2.5).
const KNOWN_ENV_KEYS: &[&str] = &[
    "RUST_LOG",
    "PHASE_MIRROR_BIND",
    "PHASE_MIRROR_WS_PORT",
    "PHASE_MIRROR_CORS_ORIGINS",
    "PHASE_MIRROR_TLS_CERT",
    "PHASE_MIRROR_TLS_KEY",
    "PHASE_MIRROR_STATE_DIR",
    "PHASE_MIRROR_TOOL_CONFIG",
    "PHASE_MIRROR_TOOL_ALLOW",
    "PHASE_MIRROR_ENV",
    "PHASE_MIRROR_OPERATOR_KEYS",
    "PHASE_MIRROR_OPERATOR_KEYS_FILE",
    "PHASE_MIRROR_RATE_LIMIT_RPS",
    "PHASE_MIRROR_RATE_LIMIT_BURST",
    "PHASE_MIRROR_WS_AUTH",
    "PHASE_MIRROR_LOOPBACK_EXCEPTION",
    "PHASE_MIRROR_LOG_FORMAT",
    "PHASE_MIRROR_LOG_LEVEL",
    "PHASE_MIRROR_NO_TLS",
    "PHASE_MIRROR_MTLS_CA",
];

/// Load and validate `.env` if present. Unknown keys abort startup; already-set
/// environment variables take precedence (real env wins over the file).
pub fn load_validated(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;

    for (line_no, raw_line) in content.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = line.split_once('=').with_context(|| {
            format!(
                "{}:{}: malformed line (expected KEY=VALUE)",
                path.display(),
                line_no + 1
            )
        })?;
        let key = key.trim();
        if !KNOWN_ENV_KEYS.contains(&key) {
            bail!(
                "{}:{}: unrecognized environment key '{key}'",
                path.display(),
                line_no + 1
            );
        }
        if std::env::var_os(key).is_none() {
            std::env::set_var(key, value.trim());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_env(dir: &Path, content: &str) -> PathBuf {
        let path = dir.join(".env");
        std::fs::write(&path, content).unwrap();
        path
    }

    use std::path::PathBuf;

    #[test]
    fn missing_file_is_ok() {
        assert!(load_validated(Path::new("/nonexistent/.env")).is_ok());
    }

    #[test]
    fn known_keys_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_env(dir.path(), "PHASE_MIRROR_BIND=127.0.0.1:9999\n# comment\n");
        assert!(load_validated(&path).is_ok());
    }

    #[test]
    fn unknown_key_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_env(dir.path(), "DATABASE_URL=secret\n");
        let err = load_validated(&path).unwrap_err().to_string();
        assert!(
            err.contains("unrecognized environment key 'DATABASE_URL'"),
            "{err}"
        );
    }

    #[test]
    fn malformed_line_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_env(dir.path(), "PHASE_MIRROR_BIND\n");
        assert!(load_validated(&path).is_err());
    }
}
