use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use axum::{
    extract::State,
    http::{header, Method, StatusCode},
    middleware,
    response::Json,
    routing::{get, post},
    Router,
};
use axum_server::tls_rustls::RustlsConfig;
use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use tower_http::cors::{AllowMethods, AllowOrigin, CorsLayer};
use tracing::{error, info, warn};

// pirtm-apps is outside this workspace. CNL compilation is not a Phase Mirror binary.
fn compile_command(_text: &str) -> Result<std::convert::Infallible, String> {
    Err("pirtm-apps is not in this workspace".into())
}

mod audit;
mod audit_api;
mod cnl_bridge;
mod executor;
mod health;
mod observability;
mod security;
mod ws_broadcast;

use audit::store::AuditStore;
use audit_api::{audit_router, AuditApiState};
use cnl_bridge::plan_execution;
use executor::adapters::build_registry;
use executor::config::ToolConfig;
use executor::{ExecError, Receipt, ToolRegistry};
use health::{health_check, ready_check};
use observability::metrics::{handle_metrics, Metrics};
use observability::{init_log, observe_request, LogFormat};
use security::auth::{AuthState, Authenticated, Keyring};
use security::ratelimit::RateLimitState;
use ws_broadcast::{OperatorEvent, OperatorState, WsOptions, WsServer};

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Env {
    Dev,
    Production,
}

impl Env {
    fn is_production(&self) -> bool {
        matches!(self, Env::Production)
    }
}

#[derive(Parser)]
#[clap(
    name = "phase-mirror-agent",
    about = "High-integrity governance gateway for agentic AI workflows"
)]
struct Args {
    /// Address to bind the HTTP server
    #[clap(long, env = "PHASE_MIRROR_BIND", default_value = "0.0.0.0:8080")]
    bind: SocketAddr,

    /// Port for the operator WebSocket broadcast server
    #[clap(long, env = "PHASE_MIRROR_WS_PORT", default_value = "3030")]
    ws_port: u16,

    /// Comma-separated list of allowed CORS origins ("*" allows any)
    #[clap(
        long,
        env = "PHASE_MIRROR_CORS_ORIGINS",
        default_value = "http://localhost:5173,http://localhost:3000"
    )]
    cors_origins: String,

    /// PEM-encoded TLS certificate (implies TLS; requires --tls-key)
    #[clap(long, env = "PHASE_MIRROR_TLS_CERT", value_name = "CERT")]
    tls_cert: Option<PathBuf>,

    /// PEM-encoded TLS private key (implies TLS; requires --tls-cert)
    #[clap(long, env = "PHASE_MIRROR_TLS_KEY", value_name = "KEY")]
    tls_key: Option<PathBuf>,

    /// Directory for durable runtime state (audit WAL, archives)
    #[clap(long, env = "PHASE_MIRROR_STATE_DIR", default_value = "./state")]
    state_dir: PathBuf,

    /// Path to tools.toml governing adapter allow-lists and bounds
    #[clap(long, env = "PHASE_MIRROR_TOOL_CONFIG", value_name = "TOOLS_TOML")]
    tools_config: Option<PathBuf>,

    /// Comma-separated list of real adapter families to enable ("compose,systemd")
    #[clap(long, env = "PHASE_MIRROR_TOOL_ALLOW", default_value = "")]
    tool_allow: String,

    /// Deployment environment (dev | production)
    #[clap(long, env = "PHASE_MIRROR_ENV", value_enum, default_value = "dev")]
    env: Env,

    /// Inline operator keys: comma/newline-separated id:scope:secret entries
    #[clap(long, env = "PHASE_MIRROR_OPERATOR_KEYS", default_value = "")]
    operator_keys: String,

    /// Operator keys file (0600); hashed at rest (SHA-256)
    #[clap(
        long,
        env = "PHASE_MIRROR_OPERATOR_KEYS_FILE",
        default_value = "./config/keys/operator.keys"
    )]
    operator_keys_file: PathBuf,

    /// Per-client rate limit (requests per second)
    #[clap(long, env = "PHASE_MIRROR_RATE_LIMIT_RPS", default_value_t = 10)]
    rate_limit_rps: u32,

    /// Per-client rate limit burst capacity
    #[clap(long, env = "PHASE_MIRROR_RATE_LIMIT_BURST", default_value_t = 20)]
    rate_limit_burst: u32,

    /// Require a valid bearer key on WebSocket upgrades (config-gated)
    #[clap(long, env = "PHASE_MIRROR_WS_AUTH", default_value_t = false)]
    ws_auth: bool,

    /// Allow unauthenticated loopback reads (dev only; ignored in production)
    #[clap(long, env = "PHASE_MIRROR_LOOPBACK_EXCEPTION", default_value_t = true)]
    loopback_exception: bool,

    /// Log output format: "text" for local dev, "json" for machine-readable
    #[clap(long, env = "PHASE_MIRROR_LOG_FORMAT", default_value = "text")]
    log_format: String,

    /// Minimum log level: trace | debug | info | warn | error
    #[clap(long, env = "PHASE_MIRROR_LOG_LEVEL", default_value = "info")]
    log_level: String,

    /// Explicitly allow plain HTTP even in production mode
    #[clap(long, env = "PHASE_MIRROR_NO_TLS")]
    no_tls: bool,
}

pub struct AgentContext {
    pub tool_registry: ToolRegistry,
    pub ws_state: Arc<OperatorState>,
    pub audit_store: Arc<AuditStore>,
    pub metrics: Arc<Metrics>,
}

fn validate_tls(
    env: Env,
    no_tls: bool,
    cert: Option<&Path>,
    key: Option<&Path>,
) -> anyhow::Result<()> {
    if cert.is_some() != key.is_some() {
        anyhow::bail!("--tls-cert and --tls-key must be provided together");
    }
    if env.is_production() && !no_tls && (cert.is_none() || key.is_none()) {
        anyhow::bail!(
            "production mode requires TLS (--tls-cert/--tls-key) or an explicit --no-tls"
        );
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    security::env::load_validated(Path::new(".env"))?;

    let args = Args::parse();
    let production = args.env.is_production();

    let log_format = LogFormat::parse(&args.log_format)?;
    init_log(log_format, &args.log_level);

    validate_tls(
        args.env,
        args.no_tls,
        args.tls_cert.as_deref(),
        args.tls_key.as_deref(),
    )?;

    let loopback = if production {
        false
    } else {
        args.loopback_exception
    };

    let keyring = Arc::new(
        Keyring::load(
            &args.operator_keys,
            Some(&args.operator_keys_file),
            production,
        )
        .map_err(anyhow::Error::msg)?,
    );
    if keyring.is_empty() {
        warn!("no operator keys provisioned; protected routes reject all callers");
    } else {
        info!(actors = ?keyring.ids(), "operator keyring loaded (SHA-256 digests only)");
    }

    let ratelimit = Arc::new(RateLimitState::new(
        args.rate_limit_rps,
        args.rate_limit_burst,
    ));

    let metrics = Arc::new(Metrics::new());

    let ws = WsServer::start(
        args.ws_port,
        WsOptions {
            keyring: if args.ws_auth {
                Some(keyring.clone())
            } else {
                None
            },
            loopback,
            ratelimit: Some(ratelimit.clone()),
            metrics: metrics.clone(),
        },
    )
    .await?;
    ws.state.emit(OperatorEvent::UserInput {
        text: "Agent booting...".into(),
    });

    let tool_config = ToolConfig::load(args.tools_config.as_deref()).map_err(anyhow::Error::msg)?;
    let tool_allow: Vec<String> = args
        .tool_allow
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let tool_registry = build_registry(&tool_config, &tool_allow);
    info!(tools = ?tool_registry.tool_names(), allow = ?tool_allow, "tool registry initialized");

    let audit_store = AuditStore::open(&args.state_dir).await?;

    let ctx = Arc::new(Mutex::new(AgentContext {
        tool_registry,
        ws_state: ws.state.clone(),
        audit_store: audit_store.clone(),
        metrics: metrics.clone(),
    }));

    let cors = build_cors(&args.cors_origins, production)?;

    let auth_state = Arc::new(AuthState { keyring, loopback });
    let app = build_app(
        ctx,
        audit_store.clone(),
        metrics,
        cors,
        auth_state,
        ratelimit,
    );

    let addr = args.bind;
    let handle = axum_server::Handle::new();
    let server_handle = handle.clone();

    let server_task = match (&args.tls_cert, &args.tls_key, args.no_tls) {
        (Some(cert), Some(key), false) => {
            let tls = RustlsConfig::from_pem_file(cert, key)
                .await
                .map_err(|e| anyhow::anyhow!("failed to load TLS cert/key: {e}"))?;
            info!(%addr, "Phase Mirror Agent listening over TLS");
            tokio::spawn(async move {
                if let Err(e) = axum_server::bind_rustls(addr, tls)
                    .handle(server_handle)
                    .serve(app.into_make_service_with_connect_info::<SocketAddr>())
                    .await
                {
                    tracing::error!(error = %e, "server error");
                }
            })
        }
        _ => {
            info!(%addr, "Phase Mirror Agent listening (plain HTTP)");
            tokio::spawn(async move {
                if let Err(e) = axum_server::bind(addr)
                    .handle(server_handle)
                    .serve(app.into_make_service_with_connect_info::<SocketAddr>())
                    .await
                {
                    tracing::error!(error = %e, "server error");
                }
            })
        }
    };

    shutdown_signal().await;

    // ADR-007 §2.4: (1) stop accepting new connections, (2) drain in-flight
    // HTTP requests with a 10s grace, (3) drain WS subscribers cooperatively,
    // (4) flush + fsync the audit WAL, (5) exit 0.
    info!("stopping HTTP listener and draining in-flight requests");
    handle.graceful_shutdown(Some(Duration::from_secs(10)));

    ws.shutdown().await;

    if let Err(e) = audit_store.flush().await {
        error!(error = %e, "audit WAL flush on shutdown failed");
    }

    if let Err(e) = server_task.await {
        error!(error = %e, "server task failed");
    }

    info!("phase-mirror-agent shutdown complete");
    Ok(())
}

/// Assemble the routed application with the security middleware stack.
/// Protected surface (command + audit + metrics): rate-limit -> authn -> authz.
fn build_app(
    ctx: Arc<Mutex<AgentContext>>,
    audit_store: Arc<AuditStore>,
    metrics: Arc<Metrics>,
    cors: CorsLayer,
    auth_state: Arc<AuthState>,
    ratelimit: Arc<RateLimitState>,
) -> Router {
    let authenticate = middleware::from_fn_with_state(auth_state, security::auth::authenticate);
    let rate_limit = middleware::from_fn_with_state(ratelimit, security::ratelimit::rate_limit);
    let require_write = middleware::from_fn(security::auth::require_write);
    let observe = middleware::from_fn_with_state(metrics.clone(), observe_request);

    let command_routes: Router<Arc<Mutex<AgentContext>>> = Router::new()
        .route("/api/command", post(handle_command))
        .route_layer(require_write.clone())
        .route_layer(rate_limit.clone())
        .route_layer(authenticate.clone());

    let audit_routes: Router<Arc<AuditApiState>> = audit_router()
        .route_layer(require_write)
        .route_layer(rate_limit)
        .route_layer(authenticate.clone());

    let metrics_routes: Router<Arc<Metrics>> = Router::new()
        .route("/metrics", get(handle_metrics))
        .route_layer(authenticate.clone());

    let ready_routes: Router<Arc<AuditStore>> = Router::new().route("/ready", get(ready_check));

    Router::new()
        .route("/health", get(health_check))
        .merge(command_routes.with_state(ctx))
        .merge(audit_routes.with_state(Arc::new(AuditApiState {
            store: audit_store.clone(),
            metrics: metrics.clone(),
        })))
        .merge(metrics_routes.with_state(metrics))
        .merge(ready_routes.with_state(audit_store))
        .layer(cors)
        .layer(observe)
}

fn build_cors(s: &str, production: bool) -> anyhow::Result<CorsLayer> {
    let origins: Vec<axum::http::HeaderValue> = s
        .split(',')
        .map(|o| o.trim())
        .filter(|o| !o.is_empty())
        .map(|o| {
            o.parse::<axum::http::HeaderValue>()
                .map_err(|e| anyhow::anyhow!("invalid CORS origin '{o}': {e}"))
        })
        .collect::<anyhow::Result<_>>()?;

    let wildcard = origins.iter().any(|o| o == "*");
    if production && wildcard {
        anyhow::bail!("CORS wildcard origin '*' is not allowed in production mode");
    }

    let layer = CorsLayer::new()
        .allow_methods(AllowMethods::list([
            Method::GET,
            Method::POST,
            Method::OPTIONS,
        ]))
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]);
    if wildcard {
        Ok(layer.allow_origin(AllowOrigin::any()))
    } else {
        Ok(layer.allow_origin(AllowOrigin::list(origins)))
    }
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                warn!("SIGINT received, shutting down phase-mirror-agent...");
            }
            _ = term.recv() => {
                warn!("SIGTERM received, shutting down phase-mirror-agent...");
            }
        }
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C handler");
        warn!("SIGINT received, shutting down phase-mirror-agent...");
    }
}

#[derive(Deserialize)]
struct CommandRequest {
    text: String,
    session_id: Option<String>,
    dry_run: Option<bool>,
    idempotency_key: Option<String>,
}

#[derive(Serialize)]
struct CommandResponse {
    reply: String,
    action_taken: Option<String>,
    c_bound: f64,
    r_sc: f64,
    witness_id: Option<String>,
    receipt: Option<Receipt>,
    dry_run: bool,
}

async fn handle_command(
    State(ctx): State<Arc<Mutex<AgentContext>>>,
    Authenticated(auth): Authenticated,
    Json(req): Json<CommandRequest>,
) -> Result<Json<CommandResponse>, StatusCode> {
    let agent = ctx.lock().await;

    if let Some(sid) = &req.session_id {
        tracing::debug!(session_id = %sid, "command received for session");
    }
    agent.ws_state.emit(OperatorEvent::UserInput {
        text: req.text.clone(),
    });

    let compilation = match compile_command(&req.text) {
        Ok(c) => c,
        Err(e) => {
            agent.metrics.command_error();
            warn!(error = %e, "CNL compilation failed");
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    agent.ws_state.emit(OperatorEvent::Compiled {
        mocword: format!("{:?}", compilation.moc_word),
        c: compilation.c,
        rsc: compilation.rsc,
    });

    if !compilation.invariants_passed() {
        let diagnostic = compilation.diagnostic.clone().unwrap_or_default();
        agent.metrics.command_vetoed();
        agent.ws_state.emit(OperatorEvent::InvariantFail {
            diagnostic: diagnostic.clone(),
            suggestion: "Use a specific service instead of an expansive token".into(),
        });
        warn!(error = %diagnostic, "invariant violation blocks execution");
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    agent.ws_state.emit(OperatorEvent::InvariantPass {
        message: "Invariants satisfied".into(),
    });

    let action = compilation.verified_action.map_err(|e| {
        agent.metrics.command_error();
        warn!(error = %e, "no actionable command parsed");
        StatusCode::BAD_REQUEST
    })?;

    let dry_run = req.dry_run.unwrap_or(false);
    let principal = auth
        .as_ref()
        .map(|a| a.actor.clone())
        .or_else(|| req.session_id.clone());
    let plan = plan_execution(
        &action,
        req.idempotency_key.clone(),
        dry_run,
        principal.clone(),
    );

    agent.ws_state.emit(OperatorEvent::ExecStart {
        action: plan.canonical_action.clone(),
    });

    let receipt = match agent.tool_registry.invoke(&plan.request) {
        Ok(receipt) => receipt,
        Err(e) => {
            agent.metrics.command_error();
            agent.ws_state.emit(OperatorEvent::ExecFailure {
                error: e.to_string(),
            });
            warn!(error = %e, "tool execution rejected");
            let details = serde_json::json!({
                "action": plan.canonical_action,
                "receipt": null,
                "witness_hash": plan.witness_hash,
                "error": e.to_string(),
                "dry_run": dry_run,
            })
            .to_string();
            let started = std::time::Instant::now();
            let _ = agent
                .audit_store
                .append("execution".into(), principal, details)
                .await;
            agent.metrics.audit_write(started.elapsed());
            return Err(match e {
                ExecError::Failed(_) => StatusCode::INTERNAL_SERVER_ERROR,
                _ => StatusCode::BAD_REQUEST,
            });
        }
    };

    agent.ws_state.emit(OperatorEvent::ExecSuccess {
        action_id: plan.witness_hash.clone(),
        message: format!("{}: {}", receipt.status, receipt.detail),
    });

    let details = serde_json::json!({
        "action": plan.canonical_action,
        "receipt": receipt,
        "witness_hash": plan.witness_hash,
        "timestamp": plan.timestamp,
        "dry_run": dry_run,
    })
    .to_string();

    let started = std::time::Instant::now();
    agent
        .audit_store
        .append("execution".into(), principal, details)
        .await
        .map_err(|e| {
            agent.metrics.command_error();
            tracing::error!(error = %e, "audit append failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    agent.metrics.audit_write(started.elapsed());

    let reply = if dry_run {
        format!("Dry-run plan: {}", receipt.detail)
    } else {
        format!("Action completed: {} ({})", receipt.status, receipt.detail)
    };

    agent.metrics.command_admitted();

    Ok(Json(CommandResponse {
        reply,
        action_taken: Some(receipt.detail.clone()),
        c_bound: compilation.c,
        r_sc: compilation.rsc,
        witness_id: Some(plan.witness_hash.clone()),
        receipt: Some(receipt),
        dry_run,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::extract::ConnectInfo;
    use axum::http::{HeaderValue, Request};
    use security::auth::{sha256_hex, AuthContext, Scope};
    use std::fs;
    use tower::ServiceExt;

    const WRITE_KEY: &str = "pmr_op_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const READ_KEY: &str = "pmr_op_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn entries() -> String {
        format!(
            "alice:operator:write:{}\ncarol:operator:read:{}",
            sha256_hex(WRITE_KEY),
            sha256_hex(READ_KEY)
        )
    }

    async fn temp_state_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join("phase-mirror-agent-main-test")
            .join(name)
            .join(format!(
                "{}",
                chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
            ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    async fn ctx() -> Arc<Mutex<AgentContext>> {
        let dir = temp_state_dir("ctx").await;
        let audit_store = AuditStore::open(&dir).await.unwrap();
        Arc::new(Mutex::new(AgentContext {
            tool_registry: build_registry(&ToolConfig::default(), &[]),
            ws_state: Arc::new(OperatorState::new()),
            audit_store,
            metrics: Arc::new(Metrics::new()),
        }))
    }

    fn authed() -> Authenticated {
        Authenticated(Some(AuthContext {
            actor: "test-actor".into(),
            scope: Scope::Write,
        }))
    }

    fn command(text: &str) -> Json<CommandRequest> {
        Json(CommandRequest {
            text: text.to_string(),
            session_id: Some("test-session".into()),
            dry_run: None,
            idempotency_key: Some("test-key".into()),
        })
    }

    async fn secured_app(loopback: bool, rps: u32, burst: u32) -> Router {
        let dir = temp_state_dir("sec").await;
        let audit_store = AuditStore::open(&dir).await.unwrap();
        let state = Arc::new(Mutex::new(AgentContext {
            tool_registry: build_registry(&ToolConfig::default(), &[]),
            ws_state: Arc::new(OperatorState::new()),
            audit_store: audit_store.clone(),
            metrics: Arc::new(Metrics::new()),
        }));
        let keyring = Arc::new(Keyring::new(Keyring::parse_entries(&entries()).unwrap()));
        let auth_state = Arc::new(AuthState { keyring, loopback });
        let cors = build_cors("http://localhost:5173", false).unwrap();
        build_app(
            state,
            audit_store,
            Arc::new(Metrics::new()),
            cors,
            auth_state,
            Arc::new(RateLimitState::new(rps, burst)),
        )
    }

    fn command_body() -> Body {
        Body::from(r#"{"text":"deploy web-service cluster 3"}"#)
    }

    #[tokio::test]
    async fn deploy_command_yields_receipt_witness_and_audit() {
        let state = ctx().await;
        let res = handle_command(
            State(state.clone()),
            authed(),
            command("deploy web-service cluster 3"),
        )
        .await
        .unwrap()
        .0;
        assert!(res.reply.contains("Action completed"));
        assert!(res.action_taken.is_some());
        let witness = res.witness_id.expect("witness id present");
        assert_eq!(witness.len(), 64, "witness id must be a real hash");
        assert!(witness.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(res.c_bound > 0.0);
        assert!(res.r_sc > 0.0);
        assert!(!res.dry_run);

        let audit_entries = {
            let agent = state.lock().await;
            agent.audit_store.list().await
        };
        assert_eq!(audit_entries.len(), 1);
        assert_eq!(audit_entries[0].event_type, "execution");
        assert!(audit_entries[0].details.contains(&witness));
    }

    #[tokio::test]
    async fn command_records_authenticated_actor_in_audit() {
        let state = ctx().await;
        let auth = Authenticated(Some(AuthContext {
            actor: "alice".into(),
            scope: Scope::Write,
        }));
        let _ = handle_command(
            State(state.clone()),
            auth,
            command("deploy web-service cluster 3"),
        )
        .await
        .unwrap();
        let audit_entries = {
            let agent = state.lock().await;
            agent.audit_store.list().await
        };
        assert_eq!(audit_entries[0].actor.as_deref(), Some("alice"));
    }

    #[tokio::test]
    async fn unknown_tokens_are_rejected() {
        let state = ctx().await;
        let res = handle_command(
            State(state),
            authed(),
            command("please frobnicate the widget"),
        )
        .await;
        assert!(matches!(res, Err(c) if c == StatusCode::BAD_REQUEST));
    }

    #[tokio::test]
    async fn scale_command_yields_simulated_receipt() {
        let state = ctx().await;
        let res = handle_command(State(state), authed(), command("scale web-service 5"))
            .await
            .unwrap()
            .0;
        assert!(res.action_taken.is_some());
        let receipt = res.receipt.expect("receipt present");
        assert_eq!(receipt.status, "simulated");
        assert!(receipt.detail.contains("no side effect"));
        assert!(res.witness_id.is_some());
    }

    #[tokio::test]
    async fn invariant_violation_is_rejected() {
        let state = ctx().await;
        let res = handle_command(State(state), authed(), command("deploy all cluster")).await;
        assert!(matches!(
            res,
            Err(c) if c == StatusCode::UNPROCESSABLE_ENTITY
        ));
    }

    #[tokio::test]
    async fn dry_run_produces_plan_with_zero_side_effects() {
        let state = ctx().await;
        let mut req = command("deploy web-service cluster 3");
        req.0.dry_run = Some(true);
        let res = handle_command(State(state.clone()), authed(), req)
            .await
            .unwrap()
            .0;
        assert!(res.dry_run);
        assert!(res.reply.contains("Dry-run"));
        assert_eq!(res.receipt.as_ref().unwrap().status, "plan");
        let audit_entries = {
            let agent = state.lock().await;
            agent.audit_store.list().await
        };
        assert_eq!(
            audit_entries.len(),
            1,
            "dry-run still writes a plan witness"
        );
    }

    #[tokio::test]
    async fn duplicate_idempotency_key_returns_stored_receipt() {
        let state = ctx().await;
        let first = handle_command(
            State(state.clone()),
            authed(),
            command("deploy web-service cluster 3"),
        )
        .await
        .unwrap()
        .0;
        let second = handle_command(
            State(state.clone()),
            authed(),
            command("deploy web-service cluster 3"),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(
            first.receipt, second.receipt,
            "duplicate key must return the stored prior receipt"
        );
        for witness in [&first.witness_id, &second.witness_id] {
            let witness = witness.as_ref().expect("witness id present");
            assert_eq!(witness.len(), 64);
        }
        let audit_entries = {
            let agent = state.lock().await;
            agent.audit_store.list().await
        };
        assert_eq!(
            audit_entries.len(),
            2,
            "both requests are witnessed; execution itself is deduped"
        );
        assert!(audit_entries.iter().all(|e| e.event_type == "execution"));
    }

    #[tokio::test]
    async fn revoke_command_parses_to_retract_action() {
        let state = ctx().await;
        let res = handle_command(State(state), authed(), command("revoke W-1234")).await;
        assert!(res.is_ok(), "revoke should be a valid governed command");
    }

    #[test]
    fn cors_wildcard_builds_ok_in_dev() {
        assert!(build_cors("*", false).is_ok());
    }

    #[test]
    fn cors_wildcard_rejected_in_production() {
        assert!(build_cors("*", true).is_err());
    }

    #[test]
    fn cors_builds_origin_allowlist() {
        assert!(build_cors("http://a.example, http://b.example", false).is_ok());
    }

    #[test]
    fn cors_rejects_invalid_origin() {
        assert!(build_cors("http://bad\norigin", false).is_err());
    }

    #[test]
    fn cors_ignores_empty_segments() {
        let layer = build_cors(", , http://ok.example,,", false).unwrap();
        let _ = layer;
    }

    #[test]
    fn cors_origin_value_parses_as_header() {
        let value: Result<HeaderValue, _> = "http://localhost:5173".parse();
        assert!(value.is_ok());
    }

    #[test]
    fn tls_validation_enforces_production() {
        assert!(validate_tls(Env::Production, false, None, None).is_err());
        assert!(validate_tls(Env::Production, true, None, None).is_ok());
        assert!(validate_tls(
            Env::Production,
            false,
            Some(Path::new("c")),
            Some(Path::new("k"))
        )
        .is_ok());
        assert!(validate_tls(Env::Dev, false, None, None).is_ok());
        assert!(validate_tls(Env::Dev, false, Some(Path::new("c")), None).is_err());
    }

    #[tokio::test]
    async fn write_without_key_is_401() {
        let app = secured_app(false, 10, 20).await;
        let res = app
            .oneshot(
                Request::post("/api/command")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(command_body())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn write_with_unknown_key_is_401() {
        let app = secured_app(false, 10, 20).await;
        let res = app
            .oneshot(
                Request::post("/api/command")
                    .header(
                        header::AUTHORIZATION,
                        "Bearer pmr_op_ffffffffffffffffffffffffffffffff",
                    )
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(command_body())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn write_with_read_scope_is_403() {
        let app = secured_app(false, 10, 20).await;
        let res = app
            .oneshot(
                Request::post("/api/command")
                    .header(header::AUTHORIZATION, format!("Bearer {READ_KEY}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(command_body())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn write_with_write_scope_is_200() {
        let app = secured_app(false, 10, 20).await;
        let res = app
            .oneshot(
                Request::post("/api/command")
                    .header(header::AUTHORIZATION, format!("Bearer {WRITE_KEY}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(command_body())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn read_with_valid_key_is_200() {
        let app = secured_app(false, 10, 20).await;
        let res = app
            .oneshot(
                Request::get("/audit/entries")
                    .header(header::AUTHORIZATION, format!("Bearer {READ_KEY}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn read_without_key_is_401() {
        let app = secured_app(false, 10, 20).await;
        let res = app
            .oneshot(Request::get("/audit/entries").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn loopback_exempts_read_but_not_write() {
        let app = secured_app(true, 10, 20).await;
        let mut read = Request::get("/audit/entries").body(Body::empty()).unwrap();
        read.extensions_mut().insert(ConnectInfo(
            "127.0.0.1:54321".parse::<SocketAddr>().unwrap(),
        ));
        let res = app.clone().oneshot(read).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK, "loopback read exemption");

        let mut write = Request::post("/api/command")
            .header(header::CONTENT_TYPE, "application/json")
            .body(command_body())
            .unwrap();
        write.extensions_mut().insert(ConnectInfo(
            "127.0.0.1:54321".parse::<SocketAddr>().unwrap(),
        ));
        let res = app.oneshot(write).await.unwrap();
        assert_eq!(
            res.status(),
            StatusCode::UNAUTHORIZED,
            "loopback never exempts writes"
        );
    }

    #[tokio::test]
    async fn burst_over_limit_is_429_with_retry_after() {
        let app = secured_app(false, 1, 2).await;
        for _ in 0..2 {
            let res = app
                .clone()
                .oneshot(
                    Request::post("/api/command")
                        .header(header::AUTHORIZATION, format!("Bearer {WRITE_KEY}"))
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(command_body())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(res.status(), StatusCode::OK);
        }
        let res = app
            .oneshot(
                Request::post("/api/command")
                    .header(header::AUTHORIZATION, format!("Bearer {WRITE_KEY}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(command_body())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(res.headers().contains_key(header::RETRY_AFTER));
    }
}
