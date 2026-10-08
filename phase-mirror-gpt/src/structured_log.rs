use serde::Serialize;
// use slog::{Logger, JSON_FORMAT};
// use slog_scope::{scope, logger};

#[derive(Serialize)]
pub struct LogEvent {
    pub timestamp: u64,
    pub event_type: &'static str,
    pub compliance_rate: f64,
    pub ledger_seq: usize,
    pub witness_hash: Option<String>,
}
