pub mod compose;
pub mod simulated;
pub mod systemd;

use super::config::ToolConfig;
use super::ToolRegistry;

pub fn validate_service_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > 63 {
        return Err(format!("invalid service name '{}'", name));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.' || c == '_')
    {
        return Err(format!("invalid service name '{}'", name));
    }
    if name
        .chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(true)
    {
        return Err(format!(
            "invalid service name '{}': must not start with a digit",
            name
        ));
    }
    Ok(())
}

pub fn validate_replicas(replicas: u32) -> Result<(), String> {
    if (super::MIN_REPLICAS..=super::MAX_REPLICAS).contains(&replicas) {
        Ok(())
    } else {
        Err(format!(
            "replicas must be between {} and {}",
            super::MIN_REPLICAS,
            super::MAX_REPLICAS
        ))
    }
}

pub fn parse_replicas(s: &str) -> Result<u32, String> {
    let n: u32 = s.parse().map_err(|_| format!("invalid replicas '{}'", s))?;
    validate_replicas(n)?;
    Ok(n)
}

pub fn build_registry(config: &ToolConfig, env_allow: &[String]) -> ToolRegistry {
    let mut registry = ToolRegistry::new();

    let compose_enabled = config.tools.compose.enabled && env_allow.iter().any(|e| e == "compose");
    let systemd_enabled = config.tools.systemd.enabled && env_allow.iter().any(|e| e == "systemd");

    for tool in ["deploy", "scale", "destroy", "revoke"] {
        if compose_enabled {
            registry.register(Box::new(compose::ComposeAdapter::new(
                tool,
                config.tools.compose.compose_file.clone(),
                config.allow.services.clone(),
            )));
        } else if systemd_enabled {
            registry.register(Box::new(systemd::SystemdAdapter::new(
                tool,
                config.allow.services.clone(),
            )));
        } else {
            registry.register(Box::new(simulated::SimulatedTool::for_tool(tool)));
        }
    }

    registry
}
