use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

/// A single data-lineage event extracted from the Archivum ledger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageEvent {
    /// Monotonic event index within the ledger.
    pub event_id: u64,
    /// ISO-8601 timestamp of the event.
    pub timestamp: String,
    /// Type of data action: "write", "delete", "modify", "archive", "export".
    pub action: String,
    /// Identifier of the data asset affected.
    pub asset_id: String,
    /// Identity of the agent or process that performed the action.
    pub actor: String,
    /// Optional litigation-hold tag present at the time of the event.
    #[serde(default)]
    pub hold_tag: Option<String>,
}

/// A litigation hold recorded in the governance ledger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LitigationHold {
    /// Unique hold identifier.
    pub hold_id: String,
    /// When the hold was placed (ISO-8601).
    pub issued_at: String,
    /// When the hold expires, if ever.
    pub expires_at: Option<String>,
    /// Scope: list of asset IDs or patterns covered by the hold.
    pub scope: Vec<String>,
    /// Whether the hold is currently active (not expired, not released).
    pub active: bool,
}

/// Result of spoliation risk scanning.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpoliationCheckResult {
    /// Number of spoliation risk violations detected.
    pub violations: usize,
    /// Whether any high-severity violation was found.
    pub high_severity: bool,
    /// Human-readable details of each violation.
    pub details: Vec<SpoliationViolation>,
    /// Hash of the scan parameters for audit trail.
    pub scan_fingerprint: String,
}

/// A single spoliation risk violation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpoliationViolation {
    /// Event that triggered the violation.
    pub event_id: u64,
    /// The destructive action taken.
    pub action: String,
    /// Asset affected.
    pub asset_id: String,
    /// Active hold that was violated.
    pub hold_id: String,
    /// Severity: "critical" (delete/modify during hold), "high" (export during hold), "medium" (archive during hold).
    pub severity: String,
    /// Timestamp of the violating event.
    pub timestamp: String,
}

/// Classify the severity of a data action taken during an active litigation hold.
fn classify_severity(action: &str) -> &'static str {
    match action {
        "delete" | "destroy" | "purge" => "critical",
        "modify" | "overwrite" | "truncate" => "critical",
        "export" | "download" | "transfer" => "high",
        "archive" | "compress" | "move" => "medium",
        _ => "low",
    }
}

/// Check whether a hold covers a given asset ID.
fn hold_covers_asset(hold: &LitigationHold, asset_id: &str) -> bool {
    hold.scope.iter().any(|pattern| {
        if pattern == "*" {
            return true;
        }
        if pattern.ends_with('*') {
            return asset_id.starts_with(pattern.trim_end_matches('*'));
        }
        if pattern.starts_with('*') {
            return asset_id.ends_with(pattern.trim_start_matches('*'));
        }
        pattern == asset_id
    })
}

/// Check whether a hold is currently active (not expired, not explicitly released).
fn is_hold_active(hold: &LitigationHold, now_epoch: u64) -> bool {
    if !hold.active {
        return false;
    }
    if let Some(expires) = &hold.expires_at {
        if let Ok(expiry_epoch) = parse_iso8601_epoch(expires) {
            if now_epoch >= expiry_epoch {
                return false;
            }
        }
    }
    true
}

/// Parse an ISO-8601 timestamp to Unix epoch seconds. Simplified parser.
fn parse_iso8601_epoch(ts: &str) -> Result<u64, ()> {
    // Simplified: expect "YYYY-MM-DDTHH:MM:SSZ" format
    let parts: Vec<&str> = ts.split('T').collect();
    if parts.len() != 2 {
        return Err(());
    }
    let date_parts: Vec<u32> = parts[0].split('-').filter_map(|s| s.parse().ok()).collect();
    let time_parts: Vec<u32> = parts[1]
        .trim_end_matches('Z')
        .split(':')
        .filter_map(|s| s.parse().ok())
        .collect();
    if date_parts.len() != 3 || time_parts.len() != 3 {
        return Err(());
    }
    let (y, m, d) = (date_parts[0] as i64, date_parts[1], date_parts[2]);
    let (hh, mm, ss) = (time_parts[0], time_parts[1], time_parts[2]);

    // Days since epoch (simplified, no leap year correction beyond 4-year cycle)
    let days_from_years = (y - 1970) * 365 + ((y - 1972) / 4);
    let days_from_months: i64 = (1..m)
        .map(|month| match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => 28,
            _ => 0,
        })
        .sum();
    let total_days = days_from_years + days_from_months + (d as i64 - 1);
    let epoch = total_days * 86400 + (hh as i64) * 3600 + (mm as i64) * 60 + ss as i64;
    Ok(epoch as u64)
}

/// Compute a fingerprint of the scan parameters for audit purposes.
fn compute_scan_fingerprint(holds: &[LitigationHold], events: &[LineageEvent]) -> String {
    let mut hasher = Sha256::new();
    for hold in holds {
        hasher.update(hold.hold_id.as_bytes());
        hasher.update(hold.issued_at.as_bytes());
    }
    for event in events {
        hasher.update(event.event_id.to_le_bytes());
        hasher.update(event.action.as_bytes());
    }
    format!("sha256:{}", hex::encode(hasher.finalize()))
}

/// Scan data lineage for spoliation risk against active litigation holds.
///
/// This function traverses the lineage event stream and checks each destructive
/// or mutating action against active litigation holds. Any action that modifies,
/// deletes, or exports data covered by an active hold constitutes a spoliation risk.
pub fn scan_for_spoliation_risk(
    events: &[LineageEvent],
    holds: &[LitigationHold],
) -> SpoliationCheckResult {
    let now_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let active_holds: Vec<&LitigationHold> = holds
        .iter()
        .filter(|h| is_hold_active(h, now_epoch))
        .collect();

    let mut violations = Vec::new();
    let mut high_severity = false;

    for event in events {
        // Only flag mutating actions (writes are fine; deletes/modifies/exports are risky)
        let is_mutating = matches!(
            event.action.as_str(),
            "delete"
                | "destroy"
                | "purge"
                | "modify"
                | "overwrite"
                | "truncate"
                | "export"
                | "download"
                | "transfer"
                | "archive"
                | "compress"
                | "move"
        );
        if !is_mutating {
            continue;
        }

        for hold in &active_holds {
            if hold_covers_asset(hold, &event.asset_id) {
                let severity = classify_severity(&event.action);
                if severity == "critical" || severity == "high" {
                    high_severity = true;
                }
                violations.push(SpoliationViolation {
                    event_id: event.event_id,
                    action: event.action.clone(),
                    asset_id: event.asset_id.clone(),
                    hold_id: hold.hold_id.clone(),
                    severity: severity.to_string(),
                    timestamp: event.timestamp.clone(),
                });
            }
        }
    }

    let scan_fingerprint = compute_scan_fingerprint(holds, events);

    SpoliationCheckResult {
        violations: violations.len(),
        high_severity,
        details: violations,
        scan_fingerprint,
    }
}
