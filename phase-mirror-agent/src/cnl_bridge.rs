use crate::executor::ToolRequest;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerifiedAction {
    Deploy {
        service: String,
        target: String,
        replicas: u32,
    },
    Scale {
        service: String,
        replicas: u32,
    },
    Destroy {
        service: String,
    },
    Revoke {
        previous_action_id: String,
    },
}

#[derive(Debug, Clone)]
pub struct CompilationResult {
    pub moc_word: Vec<String>,
    pub c: f64,
    pub rsc: f64,
    pub diagnostic: Option<String>,
    pub verified_action: Result<VerifiedAction, String>,
}

const FILLERS: &[&str] = &["on", "with", "it", "the", "to", "at"];
pub const MIN_REPLICAS: u32 = 1;
pub const MAX_REPLICAS: u32 = 32;

fn is_filler(lower: &str) -> bool {
    FILLERS.contains(&lower)
}

fn parse_number(token: &str) -> Option<u32> {
    let n: u32 = token.parse().ok()?;
    if (MIN_REPLICAS..=MAX_REPLICAS).contains(&n) {
        Some(n)
    } else {
        None
    }
}

fn parse_replicas_at(rest: &[String], index: &mut usize) -> Result<Option<u32>, String> {
    let token = &rest[*index];
    let lower = token.to_lowercase();

    if let Some(value) = lower.strip_prefix("replicas=") {
        let n = parse_number(value).ok_or_else(|| {
            format!(
                "replicas must be between {} and {}, got '{}'",
                MIN_REPLICAS, MAX_REPLICAS, value
            )
        })?;
        *index += 1;
        return Ok(Some(n));
    }

    if lower == "replicas" {
        let next = rest.get(*index + 1).ok_or_else(|| {
            format!(
                "replicas must be between {} and {}, got end of input",
                MIN_REPLICAS, MAX_REPLICAS
            )
        })?;
        let n = parse_number(next).ok_or_else(|| {
            format!(
                "replicas must be between {} and {}, got '{}'",
                MIN_REPLICAS, MAX_REPLICAS, next
            )
        })?;
        *index += 2;
        return Ok(Some(n));
    }

    if lower.bytes().all(|b| b.is_ascii_digit()) {
        let n = parse_number(token).ok_or_else(|| {
            format!(
                "replicas must be between {} and {}, got '{}'",
                MIN_REPLICAS, MAX_REPLICAS, token
            )
        })?;
        *index += 1;
        return Ok(Some(n));
    }

    Ok(None)
}

fn parse_deploy(rest: &[String]) -> Result<VerifiedAction, String> {
    let mut service: Option<String> = None;
    let mut target: Option<String> = None;
    let mut replicas: Option<u32> = None;

    let mut i = 0;
    while i < rest.len() {
        let token = &rest[i];
        let lower = token.to_lowercase();

        if is_filler(&lower) {
            i += 1;
            continue;
        }

        if let Some(r) = parse_replicas_at(rest, &mut i)? {
            if replicas.is_some() {
                return Err(format!("Unexpected token '{}'", token));
            }
            replicas = Some(r);
            continue;
        }

        if service.is_none() {
            service = Some(token.clone());
        } else if target.is_none() {
            target = Some(token.clone());
        } else {
            return Err(format!("Unexpected token '{}'", token));
        }
        i += 1;
    }

    let service =
        service.ok_or_else(|| "deploy requires <service> <target> [replicas=N]".to_string())?;
    let target =
        target.ok_or_else(|| "deploy requires <service> <target> [replicas=N]".to_string())?;
    let replicas = replicas.unwrap_or(1);

    Ok(VerifiedAction::Deploy {
        service,
        target,
        replicas,
    })
}

fn parse_scale(rest: &[String]) -> Result<VerifiedAction, String> {
    let mut service: Option<String> = None;
    let mut replicas: Option<u32> = None;

    let mut i = 0;
    while i < rest.len() {
        let token = &rest[i];
        let lower = token.to_lowercase();

        if is_filler(&lower) {
            i += 1;
            continue;
        }

        if let Some(r) = parse_replicas_at(rest, &mut i)? {
            if replicas.is_some() {
                return Err(format!("Unexpected token '{}'", token));
            }
            replicas = Some(r);
            continue;
        }

        if service.is_none() {
            service = Some(token.clone());
        } else {
            return Err(format!("Unexpected token '{}'", token));
        }
        i += 1;
    }

    let service = service.ok_or_else(|| "scale requires <service> [replicas=N]".to_string())?;
    let replicas = replicas.unwrap_or(1);

    Ok(VerifiedAction::Scale { service, replicas })
}

fn parse_destroy(rest: &[String]) -> Result<VerifiedAction, String> {
    let mut service: Option<String> = None;

    let mut i = 0;
    while i < rest.len() {
        let token = &rest[i];
        let lower = token.to_lowercase();

        if is_filler(&lower) {
            i += 1;
            continue;
        }

        if parse_replicas_at(rest, &mut i)?.is_some() {
            return Err("destroy takes no replicas".to_string());
        }

        if service.is_none() {
            service = Some(token.clone());
        } else {
            return Err(format!("Unexpected token '{}'", token));
        }
        i += 1;
    }

    let service = service.ok_or_else(|| "destroy requires <service>".to_string())?;

    Ok(VerifiedAction::Destroy { service })
}

fn parse_revoke(rest: &[String]) -> Result<VerifiedAction, String> {
    let mut previous_action_id: Option<String> = None;

    for token in rest {
        let lower = token.to_lowercase();

        if is_filler(&lower) {
            continue;
        }

        if previous_action_id.is_none() {
            previous_action_id = Some(token.clone());
        } else {
            return Err(format!("Unexpected token '{}'", token));
        }
    }

    let previous_action_id =
        previous_action_id.ok_or_else(|| "revoke requires <action_id>".to_string())?;

    Ok(VerifiedAction::Revoke { previous_action_id })
}

impl CompilationResult {
    pub fn invariants_passed(&self) -> bool {
        self.diagnostic.is_none()
    }
}

pub fn compile_command(input: &str) -> Result<CompilationResult, String> {
    let tokens: Vec<String> = input.split_whitespace().map(|t| t.to_string()).collect();
    if tokens.is_empty() {
        return Err("Empty input".to_string());
    }

    let verb = tokens[0].to_lowercase();
    match verb.as_str() {
        "deploy" | "scale" | "destroy" | "revoke" | "cancel" => {}
        other => return Err(format!("Unknown verb '{}'", other)),
    }

    let verified_action = match verb.as_str() {
        "deploy" => parse_deploy(&tokens[1..])?,
        "scale" => parse_scale(&tokens[1..])?,
        "destroy" => parse_destroy(&tokens[1..])?,
        "revoke" | "cancel" => parse_revoke(&tokens[1..])?,
        _ => unreachable!(),
    };

    let diagnostic = if tokens.iter().any(|t| t.to_lowercase() == "all") {
        Some("L0_08_PrimeOneForbidden: Prime index 1 is strictly expansive".to_string())
    } else {
        None
    };

    Ok(CompilationResult {
        moc_word: tokens,
        c: 0.5,
        rsc: 1.5,
        diagnostic,
        verified_action: Ok(verified_action),
    })
}

pub fn action_to_tool_request(action: &VerifiedAction) -> ToolRequest {
    let (tool, args) = match action {
        VerifiedAction::Deploy {
            service,
            target,
            replicas,
        } => (
            "deploy",
            vec![service.clone(), target.clone(), replicas.to_string()],
        ),
        VerifiedAction::Scale { service, replicas } => {
            ("scale", vec![service.clone(), replicas.to_string()])
        }
        VerifiedAction::Destroy { service } => ("destroy", vec![service.clone()]),
        VerifiedAction::Revoke { previous_action_id } => {
            ("revoke", vec![previous_action_id.clone()])
        }
    };
    ToolRequest::new(tool, args)
}

pub fn canonical_action(action: &VerifiedAction) -> Value {
    match action {
        VerifiedAction::Deploy {
            service,
            target,
            replicas,
        } => json!({
            "action": "deploy",
            "service": service,
            "target": target,
            "replicas": replicas,
        }),
        VerifiedAction::Scale { service, replicas } => json!({
            "action": "scale",
            "service": service,
            "replicas": replicas,
        }),
        VerifiedAction::Destroy { service } => json!({
            "action": "destroy",
            "service": service,
        }),
        VerifiedAction::Revoke { previous_action_id } => json!({
            "action": "revoke",
            "previous_action_id": previous_action_id,
        }),
    }
}

pub fn canonical_action_string(action: &VerifiedAction) -> String {
    serde_json::to_string(&canonical_action(action)).expect("canonical action serialization")
}

pub fn witness_hash(canonical_action: &str, idempotency_key: &str, timestamp: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(canonical_action.as_bytes());
    hasher.update(idempotency_key.as_bytes());
    hasher.update(timestamp.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub struct ExecutionPlan {
    pub request: ToolRequest,
    pub canonical_action: String,
    pub witness_hash: String,
    pub timestamp: String,
}

pub fn plan_execution(
    action: &VerifiedAction,
    client_key: Option<String>,
    dry_run: bool,
    principal: Option<String>,
) -> ExecutionPlan {
    let canonical_action = canonical_action_string(action);
    let timestamp = Utc::now().to_rfc3339();
    let idem_key = client_key.as_deref().unwrap_or_default();
    let witness_hash = witness_hash(&canonical_action, idem_key, &timestamp);

    let mut request = action_to_tool_request(action);
    request.idempotency_key = client_key;
    request.dry_run = dry_run;
    request.principal = principal;

    ExecutionPlan {
        request,
        canonical_action,
        witness_hash,
        timestamp,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deploy() -> VerifiedAction {
        VerifiedAction::Deploy {
            service: "web-service".to_string(),
            target: "cluster".to_string(),
            replicas: 3,
        }
    }

    #[test]
    fn action_maps_to_tool_request() {
        let req = action_to_tool_request(&deploy());
        assert_eq!(req.tool, "deploy");
        assert_eq!(req.args, vec!["web-service", "cluster", "3"]);
    }

    #[test]
    fn canonical_action_is_sorted_and_deterministic() {
        let a = canonical_action_string(&deploy());
        let b = canonical_action_string(&deploy());
        assert_eq!(a, b);
        assert!(
            a.starts_with(
                r#"{"action":"deploy","replicas":3,"service":"web-service","target":"cluster"}"#
            ),
            "got: {}",
            a
        );
    }

    #[test]
    fn witness_hash_is_deterministic_sha256_hex() {
        let h1 = witness_hash(r#"{"action":"deploy"}"#, "key-1", "2026-08-08T12:00:00Z");
        let h2 = witness_hash(r#"{"action":"deploy"}"#, "key-1", "2026-08-08T12:00:00Z");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
        assert!(h1.chars().all(|c| c.is_ascii_hexdigit()));
        let h3 = witness_hash(r#"{"action":"deploy"}"#, "key-2", "2026-08-08T12:00:00Z");
        assert_ne!(h1, h3);
    }

    #[test]
    fn plan_execution_builds_plan() {
        let plan = plan_execution(&deploy(), Some("client-key".to_string()), true, None);
        assert_eq!(plan.request.tool, "deploy");
        assert!(plan.request.dry_run);
        assert_eq!(plan.request.idempotency_key.as_deref(), Some("client-key"));
        assert_eq!(plan.witness_hash.len(), 64);
        assert!(plan.canonical_action.contains("web-service"));
    }
}
