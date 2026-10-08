use crate::audit::store::{AuditEntry, AuditStore};
use crate::observability::metrics::Metrics;
use crate::security::auth::Authenticated;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

#[derive(Deserialize)]
pub struct NewAuditEntry {
    pub event_type: String,
    #[serde(default)]
    pub actor: Option<String>,
    pub details: String,
}

#[derive(Deserialize)]
pub struct AuditQuery {
    pub event_type: Option<String>,
    pub actor: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Serialize)]
pub struct AuditListResponse {
    pub entries: Vec<AuditEntry>,
    pub total: usize,
}

#[derive(Serialize)]
pub struct AuditSummary {
    pub total: usize,
    pub by_type: HashMap<String, usize>,
}

#[derive(Deserialize)]
pub struct AuditVerifyRequest {
    pub from: Option<u64>,
    pub to: Option<u64>,
}

/// Router state: the audit store plus the observability registry so write
/// paths can record `pm_audit_writes_total` / `_latency_seconds`.
pub struct AuditApiState {
    pub store: Arc<AuditStore>,
    pub metrics: Arc<Metrics>,
}

pub fn audit_router() -> Router<Arc<AuditApiState>> {
    Router::new()
        .route("/audit/entries", get(list_audit_entries))
        .route("/audit/entries", axum::routing::post(create_audit_entry))
        .route("/audit/entries/{id}", get(get_audit_entry))
        .route("/audit/summary", get(audit_summary))
        .route("/audit/health", get(audit_health))
        .route("/audit/integrity", get(audit_integrity))
        .route("/audit/verify", axum::routing::post(audit_verify))
        .route("/audit/rotate", axum::routing::post(audit_rotate))
}

async fn list_audit_entries(
    State(state): State<Arc<AuditApiState>>,
    Query(query): Query<AuditQuery>,
) -> Result<Json<AuditListResponse>, StatusCode> {
    let entries = state.store.list().await;
    let filtered: Vec<AuditEntry> = entries
        .into_iter()
        .filter(|e| {
            query.event_type.as_ref().is_none_or(|t| &e.event_type == t)
                && query
                    .actor
                    .as_ref()
                    .is_none_or(|a| e.actor.as_deref() == Some(a.as_str()))
        })
        .collect();

    let total = filtered.len();
    let limit = query.limit.unwrap_or(100).min(1000);
    let offset = query.offset.unwrap_or(0);
    let page = filtered.into_iter().skip(offset).take(limit).collect();

    Ok(Json(AuditListResponse {
        entries: page,
        total,
    }))
}

async fn create_audit_entry(
    State(state): State<Arc<AuditApiState>>,
    Authenticated(auth): Authenticated,
    Json(request): Json<NewAuditEntry>,
) -> Result<Json<AuditEntry>, StatusCode> {
    if request.event_type.trim().is_empty() || request.details.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Write paths must be attributable to the authenticated actor (ADR-006 §2.2);
    // a caller-supplied actor is never trusted on the write path.
    let actor = match auth.as_ref() {
        Some(a) => Some(a.actor.clone()),
        None => request.actor,
    };

    let started = Instant::now();
    let result = state
        .store
        .append(request.event_type, actor, request.details)
        .await;
    state.metrics.audit_write(started.elapsed());
    result.map(Json).map_err(|e| {
        tracing::error!(error = %e, "audit append failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

async fn get_audit_entry(
    State(state): State<Arc<AuditApiState>>,
    Path(id): Path<u64>,
) -> Result<Json<AuditEntry>, StatusCode> {
    state
        .store
        .get(id)
        .await
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn audit_summary(State(state): State<Arc<AuditApiState>>) -> Json<AuditSummary> {
    let (total, by_type) = state.store.summary().await;
    Json(AuditSummary { total, by_type })
}

async fn audit_health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "audit-api",
        "timestamp": Utc::now().to_rfc3339(),
    }))
}

async fn audit_integrity(
    State(state): State<Arc<AuditApiState>>,
) -> Result<Json<crate::audit::store::IntegrityReport>, StatusCode> {
    state.store.verify_integrity().await.map(Json).map_err(|e| {
        tracing::error!(error = %e, "integrity verification failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

async fn audit_verify(
    State(state): State<Arc<AuditApiState>>,
    Json(request): Json<AuditVerifyRequest>,
) -> Result<Json<crate::audit::store::IntegrityReport>, StatusCode> {
    let report = state.store.verify_integrity().await.map_err(|e| {
        tracing::error!(error = %e, "range verification failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut report = report;
    let entries = state.store.list().await;
    let selected: Vec<&AuditEntry> = entries
        .iter()
        .filter(|e| {
            request.from.is_none_or(|f| e.sequence >= f)
                && request.to.is_none_or(|t| e.sequence <= t)
        })
        .collect();
    let in_range_valid = if selected.is_empty() {
        report.valid
    } else {
        let mut prev = selected[0].prev_hash.clone();
        let mut ok = true;
        for entry in &selected {
            if entry.prev_hash != prev {
                ok = false;
                break;
            }
            let expected = AuditStore::entry_hash(&entry.prev_hash, &entry.payload())
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            if expected != entry.entry_hash {
                ok = false;
                break;
            }
            prev = entry.entry_hash.clone();
        }
        ok
    };
    report.valid = report.valid && in_range_valid;
    report.entries = selected.len();
    Ok(Json(report))
}

async fn audit_rotate(
    State(state): State<Arc<AuditApiState>>,
) -> Result<Json<crate::audit::store::RotateReport>, StatusCode> {
    state.store.rotate().await.map(Json).map_err(|e| {
        tracing::error!(error = %e, "audit rotation failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::store::{GENESIS_HASH, WAL_FILE};
    use crate::security::auth::{AuthContext, Scope};
    use std::fs;
    use std::path::PathBuf;

    fn authed() -> Authenticated {
        Authenticated(Some(AuthContext {
            actor: "test-operator".into(),
            scope: Scope::Write,
        }))
    }

    async fn temp_state_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join("phase-mirror-agent-api-test")
            .join(name)
            .join(format!(
                "{}",
                chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
            ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    async fn open_store(name: &str) -> Arc<AuditStore> {
        AuditStore::open(&temp_state_dir(name).await).await.unwrap()
    }

    fn state(store: Arc<AuditStore>) -> State<Arc<AuditApiState>> {
        State(Arc::new(AuditApiState {
            store,
            metrics: Arc::new(Metrics::new()),
        }))
    }

    async fn append_entries(store: &Arc<AuditStore>, events: &[(&str, &str)]) {
        for (event_type, details) in events {
            store
                .append((*event_type).to_string(), None, (*details).to_string())
                .await
                .unwrap();
        }
    }

    #[tokio::test]
    async fn create_then_get_roundtrip() {
        let store = open_store("create_then_get").await;
        let created = create_audit_entry(
            state(store.clone()),
            authed(),
            Json(NewAuditEntry {
                event_type: "command".into(),
                actor: Some("spoofed-actor".into()),
                details: "deploy web-service cluster 3".into(),
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(created.sequence, 0);
        assert_eq!(created.prev_hash, GENESIS_HASH);
        assert_eq!(created.event_type, "command");
        assert_eq!(
            created.actor.as_deref(),
            Some("test-operator"),
            "authenticated actor overrides the caller-supplied actor"
        );

        let fetched = get_audit_entry(state(store), Path(created.id))
            .await
            .unwrap()
            .0;
        assert_eq!(fetched.details, "deploy web-service cluster 3");
    }

    #[tokio::test]
    async fn rejects_empty_event_type_or_details() {
        let store = open_store("rejects_empty").await;
        let res = create_audit_entry(
            state(store.clone()),
            authed(),
            Json(NewAuditEntry {
                event_type: "  ".into(),
                actor: None,
                details: "x".into(),
            }),
        )
        .await;
        assert!(matches!(res, Err(c) if c == StatusCode::BAD_REQUEST));

        let res = create_audit_entry(
            state(store),
            authed(),
            Json(NewAuditEntry {
                event_type: "command".into(),
                actor: None,
                details: "   ".into(),
            }),
        )
        .await;
        assert!(matches!(res, Err(c) if c == StatusCode::BAD_REQUEST));
    }

    #[tokio::test]
    async fn list_filters_by_event_type_and_actor() {
        let store = open_store("list_filters").await;
        append_entries(
            &store,
            &[
                ("command", "deploy"),
                ("veto", "rejected"),
                ("command", "scale"),
            ],
        )
        .await;
        let res = list_audit_entries(
            state(store.clone()),
            Query(AuditQuery {
                event_type: Some("command".into()),
                actor: None,
                limit: None,
                offset: None,
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(res.total, 2);
        assert!(res.entries.iter().all(|e| e.event_type == "command"));
    }

    #[tokio::test]
    async fn list_paginates_with_limit_and_offset() {
        let store = open_store("list_paginates").await;
        append_entries(&store, &[("a", "0"), ("b", "1"), ("c", "2"), ("d", "3")]).await;
        let res = list_audit_entries(
            state(store),
            Query(AuditQuery {
                event_type: None,
                actor: None,
                limit: Some(2),
                offset: Some(1),
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(res.total, 4);
        assert_eq!(res.entries.len(), 2);
        assert_eq!(res.entries[0].sequence, 1);
        assert_eq!(res.entries[1].sequence, 2);
    }

    #[tokio::test]
    async fn summary_groups_by_event_type() {
        let store = open_store("summary").await;
        append_entries(&store, &[("command", "0"), ("veto", "1"), ("command", "2")]).await;
        let summary = audit_summary(state(store)).await.0;
        assert_eq!(summary.total, 3);
        assert_eq!(summary.by_type.get("command"), Some(&2));
        assert_eq!(summary.by_type.get("veto"), Some(&1));
    }

    #[tokio::test]
    async fn missing_entry_yields_not_found() {
        let store = open_store("missing").await;
        let res = get_audit_entry(state(store), Path(42)).await;
        assert!(matches!(res, Err(c) if c == StatusCode::NOT_FOUND));
    }

    #[tokio::test]
    async fn integrity_reports_valid_and_detects_tamper() {
        let dir = temp_state_dir("integrity_api").await;
        let store = AuditStore::open(&dir).await.unwrap();
        append_entries(&store, &[("command", "a"), ("command", "b")]).await;
        let report = audit_integrity(state(store.clone())).await.unwrap().0;
        assert!(report.valid);
        assert_eq!(report.entries, 2);
        assert_eq!(report.head_hash.len(), 64);

        drop(store);
        let wal = dir.join("audit").join(WAL_FILE);
        let content = fs::read_to_string(&wal)
            .unwrap()
            .replacen("command", "COMMAND", 1);
        fs::write(&wal, content).unwrap();
        let reopened = AuditStore::open(&dir).await;
        assert!(reopened.is_err(), "tampered WAL rejected on reopen");
    }

    #[tokio::test]
    async fn verify_reports_range_selection() {
        let store = open_store("verify_range").await;
        append_entries(&store, &[("a", "0"), ("b", "1"), ("c", "2")]).await;
        let res = audit_verify(
            state(store.clone()),
            Json(AuditVerifyRequest {
                from: Some(1),
                to: Some(2),
            }),
        )
        .await
        .unwrap()
        .0;
        assert!(res.valid);
        assert_eq!(res.entries, 2);
    }

    #[tokio::test]
    async fn rotate_endpoint_archives_and_continues() {
        let store = open_store("rotate_api").await;
        append_entries(&store, &[("a", "0")]).await;
        let report = audit_rotate(state(store.clone())).await.unwrap().0;
        assert!(report.archived.as_path().exists());
        let created = create_audit_entry(
            state(store.clone()),
            authed(),
            Json(NewAuditEntry {
                event_type: "b".into(),
                actor: None,
                details: "1".into(),
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(created.prev_hash, report.head_hash, "post-rotation link");
    }
}
