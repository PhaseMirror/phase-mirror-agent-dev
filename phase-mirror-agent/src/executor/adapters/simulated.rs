use super::super::{ExecError, Receipt, ToolExecutor, ToolRequest};
use super::{parse_replicas, validate_service_name};
use chrono::Utc;

pub struct SimulatedTool {
    name: &'static str,
}

impl SimulatedTool {
    pub fn for_tool(tool: &'static str) -> Self {
        Self { name: tool }
    }
}

impl ToolExecutor for SimulatedTool {
    fn name(&self) -> &'static str {
        self.name
    }

    fn allowed(&self, _req: &ToolRequest) -> bool {
        true
    }

    fn validate(&self, req: &ToolRequest) -> Result<(), String> {
        match self.name {
            "deploy" => {
                if req.args.len() < 2 {
                    return Err("deploy requires <service> <target> [replicas]".to_string());
                }
                validate_service_name(&req.args[0])?;
                if let Some(replicas) = req.args.get(2) {
                    parse_replicas(replicas)?;
                }
            }
            "scale" => {
                if req.args.is_empty() {
                    return Err("scale requires <service> [replicas]".to_string());
                }
                validate_service_name(&req.args[0])?;
                if let Some(replicas) = req.args.get(1) {
                    parse_replicas(replicas)?;
                }
            }
            "destroy" | "revoke" => {
                if req.args.is_empty() {
                    return Err(format!("{} requires <service>", self.name));
                }
                validate_service_name(&req.args[0])?;
            }
            other => return Err(format!("unimplemented tool '{}'", other)),
        }
        Ok(())
    }

    fn execute(&self, req: &ToolRequest) -> Result<Receipt, ExecError> {
        let now = Utc::now().to_rfc3339();
        Ok(Receipt {
            tool: req.tool.clone(),
            status: "simulated".to_string(),
            idempotency_key: req.idempotency_key.clone().unwrap_or_default(),
            started_at: now.clone(),
            finished_at: now,
            detail: format!("no side effect (simulated) for tool '{}'", self.name),
            exit: 0,
        })
    }
}
