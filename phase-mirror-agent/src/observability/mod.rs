pub mod metrics;

use axum::{
    extract::{ConnectInfo, Request, State},
    middleware::Next,
    response::Response,
};
use chrono::Utc;
use serde_json::{Map, Value};
use std::fmt;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::fmt::{format::Writer, FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::EnvFilter;

use crate::observability::metrics::Metrics;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Json,
    Text,
}

impl LogFormat {
    pub fn parse(s: &str) -> anyhow::Result<LogFormat> {
        match s.trim().to_ascii_lowercase().as_str() {
            "json" => Ok(LogFormat::Json),
            "text" => Ok(LogFormat::Text),
            other => anyhow::bail!("invalid log format '{other}' (expected 'json' or 'text')"),
        }
    }
}

/// Install the process-wide tracing subscriber. Returns false when a subscriber
/// is already installed (idempotent for test harnesses).
pub fn init_log(format: LogFormat, level: &str) -> bool {
    let filter = EnvFilter::try_new(level).unwrap_or_else(|_| {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
    });
    match format {
        LogFormat::Json => tracing_subscriber::fmt()
            .event_format(JsonEventFormatter)
            .with_env_filter(filter)
            .try_init()
            .is_ok(),
        LogFormat::Text => tracing_subscriber::fmt()
            .with_env_filter(filter)
            .try_init()
            .is_ok(),
    }
}

/// Request-scoped observation (ADR-007 §2.1/§2.2): one structured log line per
/// request plus the `pm_http_requests_total{method,status}` counter.
pub async fn observe_request(
    State(metrics): State<Arc<Metrics>>,
    req: Request,
    next: Next,
) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let session_id = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0.ip().to_string())
        .unwrap_or_else(|| "anonymous".to_string());

    let started = Instant::now();
    let res = next.run(req).await;
    let latency_ms = started.elapsed();

    let status = res.status().as_u16();
    metrics.http_request(method.as_str(), status);

    tracing::info!(
        method = %method,
        path = %path,
        status = status,
        latency_ms = latency_ms.as_secs_f64() * 1000.0,
        session_id = %session_id,
        "http request handled"
    );
    res
}

/// Builds the structured JSON representation shared by the log formatter and
/// its unit tests: `ts`, `level`, `target`, `msg`, `service`, `pid`, `fields`.
pub fn json_event(level: &str, target: &str, msg: Value, fields: Map<String, Value>) -> Value {
    let mut obj = Map::new();
    obj.insert("ts".into(), Value::String(Utc::now().to_rfc3339()));
    obj.insert("level".into(), Value::String(level.to_string()));
    obj.insert("target".into(), Value::String(target.to_string()));
    obj.insert("msg".into(), msg);
    obj.insert("service".into(), Value::String("phase-mirror-agent".into()));
    obj.insert(
        "pid".into(),
        Value::Number((std::process::id() as u64).into()),
    );
    obj.insert("fields".into(), Value::Object(fields));
    Value::Object(obj)
}

/// Tracing formatter emitting the ADR-007 §2.1 JSON shape.
pub struct JsonEventFormatter;

impl<S, N> FormatEvent<S, N> for JsonEventFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        _ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let mut fields = Map::new();
        let mut visitor = JsonVisitor {
            fields: &mut fields,
        };
        event.record(&mut visitor);

        let msg = fields.remove("message").unwrap_or(Value::Null);
        let level = event.metadata().level().as_str().to_uppercase();
        let target = event.metadata().target();
        let json = json_event(&level, target, msg, fields);
        writeln!(writer, "{json}")
    }
}

struct JsonVisitor<'a> {
    fields: &'a mut Map<String, Value>,
}

impl Visit for JsonVisitor<'_> {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.fields.insert(
            field.name().to_string(),
            Value::String(format!("{value:?}")),
        );
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.fields
            .insert(field.name().to_string(), Value::String(value.to_string()));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.fields
            .insert(field.name().to_string(), Value::Number(value.into()));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.fields
            .insert(field.name().to_string(), Value::Number(value.into()));
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.fields
            .insert(field.name().to_string(), Value::Bool(value));
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.fields
            .insert(field.name().to_string(), Value::from(value));
    }

    fn record_error(&mut self, field: &Field, value: &(dyn std::error::Error + 'static)) {
        self.fields
            .insert(field.name().to_string(), Value::String(value.to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_format_parses() {
        assert_eq!(LogFormat::parse("json").unwrap(), LogFormat::Json);
        assert_eq!(LogFormat::parse(" TEXT ").unwrap(), LogFormat::Text);
        assert!(LogFormat::parse("xml").is_err());
    }

    #[test]
    fn json_event_contains_required_fields() {
        let mut fields = Map::new();
        fields.insert("status".into(), Value::Number(200u64.into()));
        fields.insert("latency_ms".into(), Value::from(3.5));
        let event = json_event(
            "INFO",
            "phase_mirror_agent",
            Value::String("ok".into()),
            fields,
        );

        let obj = event.as_object().unwrap();
        assert!(obj.contains_key("ts"));
        assert_eq!(obj["level"], "INFO");
        assert_eq!(obj["target"], "phase_mirror_agent");
        assert_eq!(obj["msg"], "ok");
        assert_eq!(obj["service"], "phase-mirror-agent");
        assert_eq!(obj["pid"].as_u64(), Some(std::process::id() as u64));
        assert_eq!(obj["fields"]["status"], 200);
        assert!(obj["ts"]
            .as_str()
            .unwrap()
            .parse::<chrono::DateTime<Utc>>()
            .is_ok());
    }
}
