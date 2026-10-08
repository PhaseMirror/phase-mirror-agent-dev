pub mod adapters;
pub mod config;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fmt;
use std::sync::Mutex;

pub const MIN_REPLICAS: u32 = 1;
pub const MAX_REPLICAS: u32 = 32;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolRequest {
    pub tool: String,
    pub args: Vec<String>,
    pub idempotency_key: Option<String>,
    pub dry_run: bool,
    pub principal: Option<String>,
}

impl ToolRequest {
    pub fn new(tool: &str, args: Vec<String>) -> Self {
        Self {
            tool: tool.to_string(),
            args,
            idempotency_key: None,
            dry_run: false,
            principal: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Receipt {
    pub tool: String,
    pub status: String,
    pub idempotency_key: String,
    pub started_at: String,
    pub finished_at: String,
    pub detail: String,
    pub exit: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecError {
    NotAllowed(String),
    Invalid(String),
    Failed(String),
}

impl fmt::Display for ExecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecError::NotAllowed(msg) => write!(f, "not allowed: {}", msg),
            ExecError::Invalid(msg) => write!(f, "invalid request: {}", msg),
            ExecError::Failed(msg) => write!(f, "execution failed: {}", msg),
        }
    }
}

impl std::error::Error for ExecError {}

pub trait ToolExecutor: Send + Sync {
    fn name(&self) -> &'static str;
    fn allowed(&self, req: &ToolRequest) -> bool;
    fn validate(&self, req: &ToolRequest) -> Result<(), String>;
    fn execute(&self, req: &ToolRequest) -> Result<Receipt, ExecError>;
}

pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn ToolExecutor>>,
    receipts: Mutex<HashMap<String, Receipt>>,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            receipts: Mutex::new(HashMap::new()),
        }
    }

    pub fn register(&mut self, executor: Box<dyn ToolExecutor>) {
        self.tools.insert(executor.name().to_string(), executor);
    }

    pub fn has_tool(&self, tool: &str) -> bool {
        self.tools.contains_key(tool)
    }

    pub fn tool_names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    pub fn invoke(&self, req: &ToolRequest) -> Result<Receipt, ExecError> {
        let executor = self
            .tools
            .get(&req.tool)
            .ok_or_else(|| ExecError::NotAllowed(format!("tool '{}' not allowed", req.tool)))?;

        if !executor.allowed(req) {
            return Err(ExecError::NotAllowed(format!(
                "tool '{}' not allowed",
                req.tool
            )));
        }

        executor.validate(req).map_err(ExecError::Invalid)?;

        let key = effective_idempotency_key(req);
        if let Some(stored) = self.receipts.lock().unwrap().get(&key) {
            return Ok(stored.clone());
        }

        let receipt = if req.dry_run {
            plan_receipt(req)
        } else {
            executor.execute(req)?
        };

        self.receipts.lock().unwrap().insert(key, receipt.clone());
        Ok(receipt)
    }

    pub fn receipt_count(&self) -> usize {
        self.receipts.lock().unwrap().len()
    }
}

pub fn effective_idempotency_key(req: &ToolRequest) -> String {
    let mut hasher = Sha256::new();
    if let Some(principal) = &req.principal {
        hasher.update(b"principal:");
        hasher.update(principal.as_bytes());
        hasher.update(b"|");
    }
    hasher.update(b"tool:");
    hasher.update(req.tool.as_bytes());
    hasher.update(b"|");
    for arg in &req.args {
        hasher.update(b"arg:");
        hasher.update(arg.as_bytes());
        hasher.update(b"|");
    }
    hasher.update(b"dry_run:");
    hasher.update(if req.dry_run { b"1" } else { b"0" });
    hasher.update(b"|key:");
    hasher.update(req.idempotency_key.as_deref().unwrap_or_default());
    format!("{:x}", hasher.finalize())
}

fn plan_receipt(req: &ToolRequest) -> Receipt {
    let now = Utc::now().to_rfc3339();
    Receipt {
        tool: req.tool.clone(),
        status: "plan".to_string(),
        idempotency_key: req.idempotency_key.clone().unwrap_or_default(),
        started_at: now.clone(),
        finished_at: now,
        detail: format!(
            "dry-run: validated only, no side effect for tool '{}'",
            req.tool
        ),
        exit: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::adapters::{build_registry, compose};
    use crate::executor::config::ToolConfig;

    fn simulated_request(tool: &str, args: Vec<&str>) -> ToolRequest {
        ToolRequest {
            tool: tool.to_string(),
            args: args.into_iter().map(String::from).collect(),
            idempotency_key: Some("test-key".to_string()),
            dry_run: false,
            principal: Some("test-principal".to_string()),
        }
    }

    #[test]
    fn simulated_receipts_are_honest() {
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(adapters::simulated::SimulatedTool::for_tool(
            "deploy",
        )));
        let receipt = registry
            .invoke(&simulated_request(
                "deploy",
                vec!["web-service", "cluster", "3"],
            ))
            .unwrap();
        assert_eq!(receipt.status, "simulated");
        assert!(receipt.detail.contains("no side effect (simulated)"));
        assert_eq!(receipt.exit, 0);
    }

    #[test]
    fn unknown_tool_is_rejected_at_admission() {
        let registry = ToolRegistry::new();
        let err = registry
            .invoke(&simulated_request("rm -rf /", vec!["x"]))
            .unwrap_err();
        assert!(matches!(err, ExecError::NotAllowed(_)));
    }

    #[test]
    fn invalid_arguments_are_rejected() {
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(adapters::simulated::SimulatedTool::for_tool(
            "deploy",
        )));
        let err = registry
            .invoke(&simulated_request("deploy", vec!["bad name!", "cluster"]))
            .unwrap_err();
        assert!(matches!(err, ExecError::Invalid(_)));
    }

    #[test]
    fn replicas_bounds_are_enforced() {
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(adapters::simulated::SimulatedTool::for_tool(
            "deploy",
        )));
        let err = registry
            .invoke(&simulated_request(
                "deploy",
                vec!["web-service", "cluster", "100"],
            ))
            .unwrap_err();
        assert!(matches!(err, ExecError::Invalid(_)));
        assert!(err.to_string().contains("replicas"));
    }

    #[test]
    fn duplicate_idempotency_key_does_not_re_execute() {
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(adapters::simulated::SimulatedTool::for_tool(
            "deploy",
        )));
        let first = registry
            .invoke(&simulated_request(
                "deploy",
                vec!["web-service", "cluster", "3"],
            ))
            .unwrap();
        let second = registry
            .invoke(&simulated_request(
                "deploy",
                vec!["web-service", "cluster", "3"],
            ))
            .unwrap();
        assert_eq!(first, second);
        assert_eq!(registry.receipt_count(), 1);
    }

    #[test]
    fn different_args_do_not_collide_on_same_client_key() {
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(adapters::simulated::SimulatedTool::for_tool(
            "deploy",
        )));
        registry
            .invoke(&simulated_request(
                "deploy",
                vec!["web-service", "cluster", "3"],
            ))
            .unwrap();
        registry
            .invoke(&simulated_request(
                "deploy",
                vec!["database", "cluster", "3"],
            ))
            .unwrap();
        assert_eq!(registry.receipt_count(), 2);
    }

    #[test]
    fn dry_run_returns_plan_receipt_with_zero_side_effects() {
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(adapters::simulated::SimulatedTool::for_tool(
            "deploy",
        )));
        let mut req = simulated_request("deploy", vec!["web-service", "cluster", "3"]);
        req.dry_run = true;
        let receipt = registry.invoke(&req).unwrap();
        assert_eq!(receipt.status, "plan");
        assert!(receipt.detail.contains("dry-run"));
        assert!(receipt.detail.contains("no side effect"));
    }

    #[test]
    fn dry_run_and_real_run_do_not_share_idempotency() {
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(adapters::simulated::SimulatedTool::for_tool(
            "deploy",
        )));
        let mut dry = simulated_request("deploy", vec!["web-service", "cluster", "3"]);
        dry.dry_run = true;
        registry.invoke(&dry).unwrap();
        registry
            .invoke(&simulated_request(
                "deploy",
                vec!["web-service", "cluster", "3"],
            ))
            .unwrap();
        assert_eq!(registry.receipt_count(), 2);
    }

    #[test]
    fn effective_key_changes_with_principal_args_and_key() {
        let base = simulated_request("deploy", vec!["web-service", "cluster", "3"]);
        let mut other_key = base.clone();
        other_key.idempotency_key = Some("other-key".to_string());
        let mut other_args = base.clone();
        other_args.args = vec![
            "database".to_string(),
            "cluster".to_string(),
            "3".to_string(),
        ];
        let mut other_principal = base.clone();
        other_principal.principal = Some("other-principal".to_string());

        assert_ne!(
            effective_idempotency_key(&base),
            effective_idempotency_key(&other_key)
        );
        assert_ne!(
            effective_idempotency_key(&base),
            effective_idempotency_key(&other_args)
        );
        assert_ne!(
            effective_idempotency_key(&base),
            effective_idempotency_key(&other_principal)
        );
    }

    #[test]
    fn build_registry_registers_simulated_default() {
        let config = ToolConfig::default();
        let registry = build_registry(&config, &[]);
        assert!(registry.has_tool("deploy"));
        assert!(registry.has_tool("scale"));
        assert!(registry.has_tool("destroy"));
        assert_eq!(registry.tool_names().len(), 4);
    }

    #[test]
    fn compose_adapter_allow_list_is_fail_closed() {
        let empty =
            compose::ComposeAdapter::new("deploy", "docker-compose.yaml".to_string(), vec![]);
        let req = simulated_request("deploy", vec!["web-service", "cluster", "3"]);
        assert!(!empty.allowed(&req));

        let with_allow = compose::ComposeAdapter::new(
            "deploy",
            "docker-compose.yaml".to_string(),
            vec!["web-service".to_string(), "frontend-*".to_string()],
        );
        assert!(with_allow.allowed(&req));
        let denied = simulated_request("deploy", vec!["database", "cluster", "3"]);
        assert!(!with_allow.allowed(&denied));
    }

    #[test]
    fn compose_adapter_validates_args_without_executing() {
        let adapter = compose::ComposeAdapter::new(
            "deploy",
            "docker-compose.yaml".to_string(),
            vec!["*".to_string()],
        );
        let ok = simulated_request("deploy", vec!["web-service", "cluster", "3"]);
        assert!(adapter.validate(&ok).is_ok());
        let bad_replicas = simulated_request("deploy", vec!["web-service", "cluster", "0"]);
        assert!(adapter.validate(&bad_replicas).is_err());
        let missing = simulated_request("deploy", vec!["web-service"]);
        assert!(adapter.validate(&missing).is_err());
    }
}
