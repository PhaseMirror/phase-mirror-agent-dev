#![allow(dead_code)]
/// Log redaction for sensitive material (ADR-006 §2.5). The gateway never
/// retains raw operator keys, so this is a defensive layer: any value passed
/// through a `Redactor` has every known secret occurrence replaced.
#[derive(Debug, Clone, Default)]
pub struct Redactor {
    secrets: Vec<String>,
}

impl Redactor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, secret: &str) {
        if !secret.is_empty() && !self.secrets.iter().any(|s| s == secret) {
            self.secrets.push(secret.to_string());
        }
    }

    pub fn is_empty(&self) -> bool {
        self.secrets.is_empty()
    }

    /// Replace every occurrence of any known secret with `[REDACTED]`.
    pub fn redact(&self, input: &str) -> String {
        if self.secrets.is_empty() {
            return input.to_string();
        }
        let mut out = input.to_string();
        for secret in &self.secrets {
            out = out.replace(secret.as_str(), "[REDACTED]");
        }
        out
    }
}

/// Convenience for redacting a single sensitive string (e.g. a bearer token
/// that must never be echoed into a log line).
pub fn redact_single(secret: &str, input: &str) -> String {
    let mut r = Redactor::new();
    r.add(secret);
    r.redact(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_full_and_partial_occurrences() {
        let key = "pmr_op_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let mut r = Redactor::new();
        r.add(key);
        let line = format!("Authorization: Bearer {key} (retry with same key)");
        let out = r.redact(&line);
        assert!(!out.contains(key));
        assert!(!out.contains(KEY_FRAGMENT));
        assert_eq!(
            out,
            "Authorization: Bearer [REDACTED] (retry with same key)"
        );
    }

    #[test]
    fn empty_redactor_is_identity() {
        assert_eq!(Redactor::new().redact("pmr_op_x"), "pmr_op_x");
    }

    #[test]
    fn single_secret_helper_works() {
        let out = redact_single("sekret", "value=sekret done");
        assert_eq!(out, "value=[REDACTED] done");
    }

    #[test]
    fn deduplicates_secrets() {
        let mut r = Redactor::new();
        r.add("x");
        r.add("x");
        assert_eq!(r.secrets.len(), 1);
    }

    const KEY_FRAGMENT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
}
