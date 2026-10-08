use super::super::{ExecError, Receipt, ToolExecutor, ToolRequest};
use super::validate_service_name;
use chrono::Utc;
use std::process::Command;

pub struct SystemdAdapter {
    name: &'static str,
    allowed_services: Vec<regex::Regex>,
}

impl SystemdAdapter {
    pub fn new(name: &'static str, allowed_services: Vec<String>) -> Self {
        let allowed_services = allowed_services
            .iter()
            .filter_map(|pattern| regex::Regex::new(&format!("^(?:{})$", pattern)).ok())
            .collect();
        Self {
            name,
            allowed_services,
        }
    }

    fn service_allowed(&self, service: &str) -> bool {
        if self.allowed_services.is_empty() {
            return false;
        }
        self.allowed_services.iter().any(|re| re.is_match(service))
    }
}

impl ToolExecutor for SystemdAdapter {
    fn name(&self) -> &'static str {
        self.name
    }

    fn allowed(&self, req: &ToolRequest) -> bool {
        match req.args.first() {
            Some(service) => self.service_allowed(service),
            None => false,
        }
    }

    fn validate(&self, req: &ToolRequest) -> Result<(), String> {
        match self.name {
            "deploy" => {
                if req.args.is_empty() {
                    return Err("deploy requires <service>".to_string());
                }
                validate_service_name(&req.args[0])?;
            }
            "destroy" => {
                if req.args.is_empty() {
                    return Err("destroy requires <service>".to_string());
                }
                validate_service_name(&req.args[0])?;
            }
            "revoke" => {
                if req.args.is_empty() {
                    return Err("revoke requires <action_id>".to_string());
                }
            }
            "scale" => {
                return Err("systemd adapter does not support scale: replicas only apply to compose or containers".to_string());
            }
            other => return Err(format!("unimplemented tool '{}'", other)),
        }
        Ok(())
    }

    fn execute(&self, req: &ToolRequest) -> Result<Receipt, ExecError> {
        let started_at = Utc::now().to_rfc3339();

        let unit = match self.name {
            "deploy" => req.args[0].clone(),
            "destroy" => req.args[0].clone(),
            "revoke" => req.args[0]
                .strip_prefix("deployed_")
                .ok_or_else(|| {
                    ExecError::Invalid(format!("revoke: unknown action id '{}'", req.args[0]))
                })?
                .to_string(),
            other => {
                return Err(ExecError::Invalid(format!(
                    "unimplemented tool '{}'",
                    other
                )));
            }
        };

        let subcommand = if self.name == "destroy" || self.name == "revoke" {
            "stop"
        } else {
            "start"
        };

        let output = Command::new("systemctl")
            .arg(subcommand)
            .arg(unit)
            .output()
            .map_err(|e| ExecError::Failed(format!("failed to run systemctl: {}", e)))?;

        let finished_at = Utc::now().to_rfc3339();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit = output.status.code().unwrap_or(-1);

        let status = if output.status.success() {
            "ok"
        } else {
            "failed"
        };
        let detail = if output.status.success() {
            stdout.trim().to_string()
        } else {
            format!(
                "systemctl {} exited {}: {}",
                subcommand,
                exit,
                stderr.trim()
            )
        };

        Ok(Receipt {
            tool: req.tool.clone(),
            status: status.to_string(),
            idempotency_key: req.idempotency_key.clone().unwrap_or_default(),
            started_at,
            finished_at,
            detail,
            exit,
        })
    }
}
