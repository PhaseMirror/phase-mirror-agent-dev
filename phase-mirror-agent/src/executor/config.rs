use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ToolConfig {
    #[serde(default)]
    pub allow: Allow,
    #[serde(default)]
    pub tools: Tools,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Allow {
    #[serde(default)]
    pub services: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Tools {
    #[serde(default)]
    pub compose: ComposeConfig,
    #[serde(default)]
    pub systemd: SystemdConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ComposeConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_compose_file")]
    pub compose_file: String,
}

impl Default for ComposeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            compose_file: default_compose_file(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SystemdConfig {
    #[serde(default)]
    pub enabled: bool,
}

fn default_compose_file() -> String {
    "deploy/compose.yaml".to_string()
}

impl ToolConfig {
    pub fn load(path: Option<&Path>) -> Result<Self, String> {
        match path {
            Some(p) => {
                let raw = std::fs::read_to_string(p)
                    .map_err(|e| format!("failed to read config {}: {}", p.display(), e))?;
                toml::from_str(&raw)
                    .map_err(|e| format!("invalid tools config {}: {}", p.display(), e))
            }
            None => Ok(ToolConfig::default()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_config_parses_and_enables_adapters() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("config/tools.toml.example");
        let config = ToolConfig::load(Some(&path))
            .unwrap_or_else(|e| panic!("example config must stay parseable: {}", e));
        assert!(config.tools.compose.enabled);
        assert_eq!(config.tools.compose.compose_file, "deploy/compose.yaml");
        assert!(config.tools.systemd.enabled);
        assert_eq!(
            config.allow.services,
            vec!["^web-service$", "^database-[0-9]+$"]
        );
    }

    #[test]
    fn default_config_is_simulated() {
        let config = ToolConfig::load(None).unwrap();
        assert!(!config.tools.compose.enabled);
        assert!(!config.tools.systemd.enabled);
        assert!(config.allow.services.is_empty());
    }
}
