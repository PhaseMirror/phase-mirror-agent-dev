use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// A litigation hold record from the governance ledger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoldRecord {
    /// Unique hold identifier.
    pub hold_id: String,
    /// ISO-8601 timestamp when the hold was placed.
    pub issued_at: String,
    /// ISO-8601 timestamp when the hold expires, if time-limited.
    pub expires_at: Option<String>,
    /// Whether the hold has been explicitly released.
    pub released: bool,
    /// ISO-8601 timestamp of release, if released.
    pub released_at: Option<String>,
    /// Scope of the hold: asset IDs, patterns, or "*" for global.
    pub scope: Vec<String>,
    /// Issuing authority (court, internal legal, regulator).
    pub issuer: String,
    /// Matter/case reference number.
    pub matter_ref: Option<String>,
    /// Free-text description of the hold purpose.
    pub description: Option<String>,
}

/// Result of a litigation hold scan.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LitigationScanResult {
    /// Whether any active litigation hold was found.
    pub active_hold: bool,
    /// Number of active holds.
    pub active_count: usize,
    /// Details of each active hold.
    pub active_holds: Vec<ActiveHoldSummary>,
    /// Total number of holds in the ledger (active + expired + released).
    pub total_holds: usize,
    /// Timestamp of the scan.
    pub scanned_at: String,
}

/// Summary of an active litigation hold for the scan result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveHoldSummary {
    pub hold_id: String,
    pub issued_at: String,
    pub expires_at: Option<String>,
    pub issuer: String,
    pub matter_ref: Option<String>,
    pub scope: Vec<String>,
}

/// Parse an ISO-8601 timestamp to Unix epoch seconds. Simplified parser.
fn parse_iso8601_epoch(ts: &str) -> Result<u64, ()> {
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

/// Determine whether a hold record is currently active.
///
/// A hold is active if:
/// 1. It has not been explicitly released (`released == false`)
/// 2. It has not expired (if `expires_at` is set, the current time is before expiry)
fn is_active(hold: &HoldRecord, now_epoch: u64) -> bool {
    if hold.released {
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

/// Scan the governance ledger for active litigation holds.
///
/// This function examines all hold records and identifies which ones are
/// currently active (not released, not expired). An active litigation hold
/// means that data within the hold's scope must be preserved and cannot be
/// deleted, modified, or exported without explicit legal authorization.
pub fn scan_for_litigation_hold(holds: &[HoldRecord]) -> LitigationScanResult {
    let now_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let now_str = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    let active: Vec<&HoldRecord> = holds.iter().filter(|h| is_active(h, now_epoch)).collect();

    let active_holds: Vec<ActiveHoldSummary> = active
        .iter()
        .map(|h| ActiveHoldSummary {
            hold_id: h.hold_id.clone(),
            issued_at: h.issued_at.clone(),
            expires_at: h.expires_at.clone(),
            issuer: h.issuer.clone(),
            matter_ref: h.matter_ref.clone(),
            scope: h.scope.clone(),
        })
        .collect();

    LitigationScanResult {
        active_hold: !active.is_empty(),
        active_count: active.len(),
        active_holds,
        total_holds: holds.len(),
        scanned_at: now_str,
    }
}
