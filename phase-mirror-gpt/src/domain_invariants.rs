use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::{Arc, RwLock};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PolicyConfig {
    pub semantic_policy: SemanticPolicyConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SemanticPolicyConfig {
    pub forbidden_patterns: Vec<String>,
}

/// SemanticPolicy: Implements L1 Domain Invariant enforcement with dynamic hot-reloading support.
#[derive(Default)]
pub struct SemanticPolicy {
    pub forbidden_patterns: Arc<RwLock<HashSet<String>>>,
}

impl SemanticPolicy {
    /// Initializes an empty policy.
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads the policy from a TOML string.
    pub fn load_from_toml(&self, toml_str: &str) -> Result<()> {
        let config: PolicyConfig =
            toml::from_str(toml_str).map_err(|e| anyhow!("Failed to parse policy.toml: {}", e))?;

        let mut set = self
            .forbidden_patterns
            .write()
            .map_err(|_| anyhow!("Policy RwLock poisoned"))?;

        set.clear();
        for pattern in config.semantic_policy.forbidden_patterns {
            set.insert(pattern.to_lowercase());
        }

        Ok(())
    }

    /// Performs a deterministic scan of the draft plan for policy violations.
    /// Returns (violation_found, offending_token).
    #[inline(always)]
    pub fn scan(&self, draft: &str) -> (bool, String) {
        let normalized = draft.to_lowercase();
        let set = self
            .forbidden_patterns
            .read()
            .expect("Policy RwLock poisoned");

        for token in set.iter() {
            if normalized.contains(token) {
                return (true, token.clone());
            }
        }
        (false, String::new())
    }

    pub fn get_pattern_count(&self) -> usize {
        self.forbidden_patterns
            .read()
            .expect("Policy RwLock poisoned")
            .len()
    }
}
