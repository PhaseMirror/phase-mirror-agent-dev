use crate::executor::ToolRequest;
use chrono::Utc;
use pirtm_apps::cnl::VerifiedAction;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

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
