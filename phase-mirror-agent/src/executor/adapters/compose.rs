use super::super::{ExecError, Receipt, ToolExecutor, ToolRequest};
use super::{parse_replicas, validate_service_name};
use chrono::Utc;
use std::process::Command;

pub struct ComposeAdapter {
    name: &'static str,
    compose_file: String,
    allowed_services: Vec<regex::Regex>,
}

impl ComposeAdapter {
    pub fn new(name: &'static str, compose_file: String, allowed_services: Vec<String>) -> Self {
        let allowed_services = allowed_services
            .iter()
            .filter_map(|pattern| regex::Regex::new(&format!("^(?:{})$", pattern)).ok())
            .collect();
        Self {
            name,
            compose_file,
            allowed_services,
        }
    }

    fn service_allowed(&self, service: &str) -> bool {
        if self.allowed_services.is_empty() {
            return false;
        }
        self.allowed_services.iter().any(|re| re.is_match(service))
    }

    fn command_args(&self, req: &ToolRequest) -> Result<Vec<String>, ExecError> {
        match self.name {
            "deploy" | "scale" => {
                let service = &req.args[0];
                let replicas = req
                    .args
                    .get(2)
                    .map(|r| r.as_str())
                    .or_else(|| req.args.get(1).map(|r| r.as_str()))
                    .unwrap_or("1");
                Ok(vec![
                    "compose".to_string(),
                    "-f".to_string(),
                    self.compose_file.clone(),
                    "up".to_string(),
                    "-d".to_string(),
                    "--scale".to_string(),
                    format!("{}={}", service, replicas),
                    service.clone(),
                ])
            }
            "destroy" => {
                let service = &req.args[0];
                Ok(vec![
                    "compose".to_string(),
                    "-f".to_string(),
                    self.compose_file.clone(),
                    "rm".to_string(),
                    "-sf".to_string(),
                    service.clone(),
                ])
            }
            "revoke" => {
                let action_id = &req.args[0];
                let service = action_id.strip_prefix("deployed_").ok_or_else(|| {
                    ExecError::Invalid(format!("revoke: unknown action id '{}'", action_id))
                })?;
                Ok(vec![
                    "compose".to_string(),
                    "-f".to_string(),
                    self.compose_file.clone(),
                    "rm".to_string(),
                    "-sf".to_string(),
                    service.to_string(),
                ])
            }
            other => Err(ExecError::Invalid(format!(
                "unimplemented tool '{}'",
                other
            ))),
        }
    }
}

impl ToolExecutor for ComposeAdapter {
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
            other => return Err(format!("unimplemented tool '{}'", other)),
        }
        Ok(())
    }

    fn execute(&self, req: &ToolRequest) -> Result<Receipt, ExecError> {
        let started_at = Utc::now().to_rfc3339();
        let kind = match self.name {
            "deploy" | "scale" => "up",
            "destroy" | "revoke" => "rm",
            other => {
                return Err(ExecError::Invalid(format!(
                    "unimplemented tool '{}'",
                    other
                )))
            }
        };
        let args = self.command_args(req)?;

        let output = Command::new("docker")
            .args(&args)
            .output()
            .map_err(|e| ExecError::Failed(format!("failed to run docker compose: {}", e)))?;

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
            format!("{} exited {}: {}", kind, exit, stderr.trim())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::ToolRequest;

    fn request(tool: &str, args: &[&str]) -> ToolRequest {
        ToolRequest::new(tool, args.iter().map(|s| s.to_string()).collect())
    }

    #[test]
    fn deploy_argv_places_compose_before_dash_f() {
        let adapter = ComposeAdapter::new("deploy", "compose.yaml".to_string(), vec![]);
        let args = adapter
            .command_args(&request("deploy", &["web-service", "cluster", "3"]))
            .unwrap();
        assert_eq!(
            args,
            vec![
                "compose",
                "-f",
                "compose.yaml",
                "up",
                "-d",
                "--scale",
                "web-service=3",
                "web-service"
            ]
        );
    }

    #[test]
    fn revoke_argv_requires_deployed_prefix() {
        let adapter = ComposeAdapter::new("revoke", "compose.yaml".to_string(), vec![]);
        assert!(adapter
            .command_args(&request("revoke", &["deployed_web-service"]))
            .is_ok());
        assert!(adapter
            .command_args(&request("revoke", &["web-service"]))
            .is_err());
    }
}
