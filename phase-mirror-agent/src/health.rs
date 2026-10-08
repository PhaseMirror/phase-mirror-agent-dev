use crate::audit::store::AuditStore;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;

/// Liveness probe (ADR-007 §2.3): always 200 while the process is serving.
/// Independent of the audit store so orchestrators can restart a wedged process.
pub async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}

/// Readiness probe (ADR-007 §2.3): reflects whether the audit WAL is writable
/// and its chain head parses. 503 (with JSON detail) otherwise.
pub async fn ready_check(State(store): State<Arc<AuditStore>>) -> Response {
    match store.check_ready().await {
        Ok(()) => Json(serde_json::json!({
            "status": "ready",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }))
        .into_response(),
        Err(e) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "status": "not_ready",
                "error": e.to_string(),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::store::WAL_FILE;
    use axum::http::StatusCode;
    use std::fs;
    use std::path::PathBuf;

    async fn temp_state_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join("phase-mirror-agent-health-test")
            .join(name)
            .join(format!(
                "{}",
                chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
            ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test]
    async fn liveness_is_always_healthy() {
        let res = health_check().await;
        assert_eq!(res.0["status"], "healthy");
    }

    #[tokio::test]
    async fn readiness_is_ready_when_wal_writable() {
        let dir = temp_state_dir("ready_ok").await;
        let store = AuditStore::open(&dir).await.unwrap();
        let res = ready_check(State(store)).await;
        assert_eq!(res.status(), StatusCode::OK);
        let body = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["status"], "ready");
    }

    #[tokio::test]
    async fn readiness_503_when_chain_head_does_not_parse() {
        let dir = temp_state_dir("ready_bad_head").await;
        let store = AuditStore::open(&dir).await.unwrap();
        store
            .append("command".into(), None, "deploy x".into())
            .await
            .unwrap();

        let audit_dir = dir.join("audit");
        let wal = audit_dir.join(WAL_FILE);
        fs::write(&wal, "{\"id\": 0, \"not\": \"an entry\"}").unwrap();

        let res = ready_check(State(store)).await;
        assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["status"], "not_ready");
    }
}
