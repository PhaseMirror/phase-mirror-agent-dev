use arc_swap::ArcSwap;
use notify::{Event, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GovernanceTier {
    Experimental,
    Authoritative,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpContract {
    pub contract_metadata: ContractMetadata,
    pub governance_floor: GovernanceFloor,
    pub tool_policies: HashMap<String, ToolPolicy>,
    pub jubilee_window: JubileeWindow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractMetadata {
    pub contract_id: String,
    pub schema_version: String,
    pub sovereign_id: String,
    pub valid_from: String,
    pub nonce: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceFloor {
    pub default_tier: GovernanceTier,
    pub enforce_sealed_veto: bool,
    pub escalation_triggers: Vec<EscalationTrigger>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationTrigger {
    pub metric: String,
    pub threshold: f64,
    pub action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPolicy {
    pub mode: String, // "Advisory" or "Authoritative"
    pub required_fields: Vec<String>,
    pub restricted_values: Option<HashMap<String, Vec<String>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JubileeWindow {
    pub sync_required: bool,
    pub max_thickness_delta: f64,
    pub checkpoint_ref: String,
}

pub struct ContractManager {
    current_contract: ArcSwap<McpContract>,
}

impl ContractManager {
    pub fn new(path: &Path) -> Result<Arc<Self>, Box<dyn std::error::Error>> {
        let contract = Self::load_contract(path)?;
        let manager = Arc::new(Self {
            current_contract: ArcSwap::from_pointee(contract),
        });

        let manager_clone = Arc::clone(&manager);
        let path_clone = path.to_path_buf();

        let mut watcher =
            notify::recommended_watcher(move |res: notify::Result<Event>| match res {
                Ok(event) => {
                    if event.kind.is_modify() {
                        if let Ok(new_contract) = Self::load_contract(&path_clone) {
                            manager_clone.current_contract.store(Arc::new(new_contract));
                            eprintln!("McpContract reloaded successfully from {:?}", path_clone);
                        }
                    }
                }
                Err(e) => eprintln!("watch error: {:?}", e),
            })?;

        watcher.watch(path, RecursiveMode::NonRecursive)?;

        // We need to keep the watcher alive, so we'll leak it for this simple implementation
        // or return it. For now, let's just leak it to keep the manager simple.
        Box::leak(Box::new(watcher));

        Ok(manager)
    }

    fn load_contract(path: &Path) -> Result<McpContract, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let contract: McpContract = serde_json::from_str(&content)?;
        Ok(contract)
    }

    pub fn get_contract(&self) -> Arc<McpContract> {
        self.current_contract.load_full()
    }

    pub fn validate_action(
        &self,
        tool_name: &str,
        arguments: &serde_json::Value,
    ) -> Result<String, String> {
        let contract = self.get_contract();

        // Check tool-specific policy
        if let Some(policy) = contract.tool_policies.get(tool_name) {
            if policy.mode == "Authoritative" {
                // In Authoritative mode, we could check for restricted values or required fields
                if let Some(restricted) = &policy.restricted_values {
                    for (field, values) in restricted {
                        if let Some(arg_val) = arguments.get(field) {
                            if let Some(s) = arg_val.as_str() {
                                if values.contains(&s.to_string()) {
                                    return Err(format!(
                                        "BLOCK: Tool '{}' used with restricted value '{}' for field '{}'.",
                                        tool_name, s, field
                                    ));
                                }
                            }
                        }
                    }
                }
                return Ok(format!(
                    "PERMITTED: Action '{}' validated against Authoritative Floor.",
                    tool_name
                ));
            }
        }

        // Default to global floor
        match contract.governance_floor.default_tier {
            GovernanceTier::Experimental => Ok(format!(
                "ADVISORY: Action '{}' permitted under Experimental Tier. No binding block triggered.",
                tool_name
            )),
            GovernanceTier::Authoritative => Ok(format!(
                "PERMITTED: Action '{}' validated against Authoritative Floor.",
                tool_name
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validation_logic() {
        let contract_json = json!({
            "contract_metadata": {
                "contract_id": "TEST",
                "schema_version": "1.0.0",
                "sovereign_id": "TEST",
                "valid_from": "now",
                "nonce": "123"
            },
            "governance_floor": {
                "default_tier": "Experimental",
                "enforce_sealed_veto": true,
                "escalation_triggers": []
            },
            "tool_policies": {
                "restricted_tool": {
                    "mode": "Authoritative",
                    "required_fields": ["field1"],
                    "restricted_values": {
                        "field1": ["forbidden"]
                    }
                }
            },
            "jubilee_window": {
                "sync_required": true,
                "max_thickness_delta": 0.05,
                "checkpoint_ref": "REF"
            }
        });

        let contract: McpContract = serde_json::from_value(contract_json).unwrap();
        let manager = ContractManager {
            current_contract: ArcSwap::from_pointee(contract),
        };

        // Test permitted action
        let res = manager.validate_action("some_tool", &json!({}));
        assert!(res.unwrap().contains("ADVISORY"));

        // Test restricted action (permitted value)
        let res = manager.validate_action("restricted_tool", &json!({"field1": "allowed"}));
        assert!(res.unwrap().contains("PERMITTED"));

        // Test restricted action (forbidden value)
        let res = manager.validate_action("restricted_tool", &json!({"field1": "forbidden"}));
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("BLOCK"));
    }
}
