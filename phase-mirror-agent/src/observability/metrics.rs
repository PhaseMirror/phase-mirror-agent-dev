use axum::{extract::State, response::IntoResponse};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Minimal in-process metrics registry (ADR-007 §2.2). Exposed as plain-text
/// Prometheus exposition on `GET /metrics` behind operator authentication.
#[derive(Debug, Default)]
pub struct Metrics {
    commands_admitted: AtomicU64,
    commands_vetoed: AtomicU64,
    commands_error: AtomicU64,
    audit_writes_total: AtomicU64,
    audit_writes_latency_nanos: AtomicU64,
    ws_clients_current: AtomicU64,
    ws_events_broadcast_total: AtomicU64,
    http_requests_total: Mutex<HashMap<(String, u16), u64>>,
}

impl Metrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn command_admitted(&self) {
        self.commands_admitted.fetch_add(1, Ordering::Relaxed);
    }

    pub fn command_vetoed(&self) {
        self.commands_vetoed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn command_error(&self) {
        self.commands_error.fetch_add(1, Ordering::Relaxed);
    }

    pub fn audit_write(&self, latency: std::time::Duration) {
        self.audit_writes_total.fetch_add(1, Ordering::Relaxed);
        let nanos = latency.as_nanos().min(u64::MAX as u128) as u64;
        self.audit_writes_latency_nanos
            .fetch_add(nanos, Ordering::Relaxed);
    }

    pub fn ws_client_inc(&self) {
        self.ws_clients_current.fetch_add(1, Ordering::Relaxed);
    }

    pub fn ws_client_dec(&self) {
        self.ws_clients_current.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn ws_event_broadcast(&self) {
        self.ws_events_broadcast_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn http_request(&self, method: &str, status: u16) {
        let mut map = self
            .http_requests_total
            .lock()
            .expect("http counter mutex poisoned");
        *map.entry((method.to_string(), status)).or_default() += 1;
    }

    /// Render all registered metrics in plain-text Prometheus exposition format.
    pub fn render(&self) -> String {
        let mut out = String::new();
        let secs = self.audit_writes_latency_nanos.load(Ordering::Relaxed) as f64 / 1e9;

        out.push_str("# TYPE pm_commands_total counter\n");
        out.push_str(&format!(
            "pm_commands_total{{outcome=\"admitted\"}} {}\n",
            self.commands_admitted.load(Ordering::Relaxed)
        ));
        out.push_str(&format!(
            "pm_commands_total{{outcome=\"vetoed\"}} {}\n",
            self.commands_vetoed.load(Ordering::Relaxed)
        ));
        out.push_str(&format!(
            "pm_commands_total{{outcome=\"error\"}} {}\n",
            self.commands_error.load(Ordering::Relaxed)
        ));

        out.push_str("# TYPE pm_audit_writes_total counter\n");
        out.push_str(&format!(
            "pm_audit_writes_total {}\n",
            self.audit_writes_total.load(Ordering::Relaxed)
        ));
        out.push_str("# TYPE pm_audit_writes_latency_seconds counter\n");
        out.push_str(&format!("pm_audit_writes_latency_seconds {secs:.9}\n"));

        out.push_str("# TYPE pm_ws_clients_current gauge\n");
        out.push_str(&format!(
            "pm_ws_clients_current {}\n",
            self.ws_clients_current.load(Ordering::Relaxed)
        ));
        out.push_str("# TYPE pm_ws_events_broadcast_total counter\n");
        out.push_str(&format!(
            "pm_ws_events_broadcast_total {}\n",
            self.ws_events_broadcast_total.load(Ordering::Relaxed)
        ));

        out.push_str("# TYPE pm_http_requests_total counter\n");
        let map = self
            .http_requests_total
            .lock()
            .expect("http counter mutex poisoned");
        for ((method, status), count) in map.iter() {
            out.push_str(&format!(
                "pm_http_requests_total{{method=\"{method}\",status=\"{status}\"}} {count}\n"
            ));
        }

        out
    }
}

pub async fn handle_metrics(State(metrics): State<Arc<Metrics>>) -> impl IntoResponse {
    metrics.render()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn counters_render_in_prometheus_format() {
        let m = Metrics::new();
        m.command_admitted();
        m.command_vetoed();
        m.audit_write(Duration::from_millis(250));
        m.ws_client_inc();
        m.ws_client_inc();
        m.ws_client_dec();
        m.ws_event_broadcast();
        m.http_request("GET", 200);
        m.http_request("GET", 200);
        m.http_request("POST", 401);

        let out = m.render();
        assert!(out.contains("pm_commands_total{outcome=\"admitted\"} 1"));
        assert!(out.contains("pm_commands_total{outcome=\"vetoed\"} 1"));
        assert!(out.contains("pm_commands_total{outcome=\"error\"} 0"));
        assert!(out.contains("pm_audit_writes_total 1"));
        assert!(out.contains("pm_audit_writes_latency_seconds 0.250000000"));
        assert!(out.contains("pm_ws_clients_current 1"));
        assert!(out.contains("pm_ws_events_broadcast_total 1"));
        assert!(out.contains("pm_http_requests_total{method=\"GET\",status=\"200\"} 2"));
        assert!(out.contains("pm_http_requests_total{method=\"POST\",status=\"401\"} 1"));
    }

    #[test]
    fn audit_latency_accumulates_as_sum_of_nanoseconds() {
        let m = Metrics::new();
        m.audit_write(Duration::from_millis(100));
        m.audit_write(Duration::from_millis(150));
        let out = m.render();
        assert!(out.contains("pm_audit_writes_total 2"));
        assert!(out.contains("pm_audit_writes_latency_seconds 0.250000000"));
    }
}
