use crate::observability::metrics::Metrics;
use crate::security::auth::{strip_bearer, Keyring};
use crate::security::ratelimit::RateLimitState;
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::{ConnectInfo, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use futures::{SinkExt, StreamExt};
use serde::Serialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, watch};
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

#[derive(Clone, Serialize, Debug)]
#[serde(tag = "type")]
pub enum OperatorEvent {
    #[serde(rename = "user_input")]
    UserInput { text: String },
    #[serde(rename = "compiled")]
    Compiled { mocword: String, c: f64, rsc: f64 },
    #[serde(rename = "invariant_pass")]
    InvariantPass { message: String },
    #[serde(rename = "invariant_fail")]
    InvariantFail {
        diagnostic: String,
        suggestion: String,
    },
    #[serde(rename = "execution_start")]
    ExecStart { action: String },
    #[serde(rename = "execution_success")]
    ExecSuccess { action_id: String, message: String },
    #[serde(rename = "execution_failure")]
    ExecFailure { error: String },
    #[serde(rename = "retraction")]
    Retraction { message: String },
    #[serde(rename = "rollback")]
    Rollback { action_id: String, result: String },
    /// Broadcast immediately before the server closes connections during a
    /// graceful shutdown (ADR-007 §2.4 step 3).
    #[serde(rename = "execution_terminated")]
    AgentTerminated { message: String },
}

pub struct OperatorState {
    tx: broadcast::Sender<OperatorEvent>,
}

impl OperatorState {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1024);
        Self { tx }
    }

    pub fn emit(&self, event: OperatorEvent) {
        if let Err(e) = self.tx.send(event) {
            debug!(error = %e, "websocket broadcast send skipped (no receivers)");
        }
    }
}

/// WebSocket surface options (ADR-006 §2.1): auth is config-gated; the
/// loopback exemption mirrors the HTTP read exemption for local dev.
pub struct WsOptions {
    pub keyring: Option<Arc<Keyring>>,
    pub loopback: bool,
    pub ratelimit: Option<Arc<RateLimitState>>,
    pub metrics: Arc<Metrics>,
}

struct WsContext {
    events: Arc<OperatorState>,
    options: Arc<WsOptions>,
    shutdown_rx: watch::Receiver<bool>,
}

/// Owns the WebSocket broadcast server so the caller can shut it down
/// gracefully alongside the HTTP server.
pub struct WsServer {
    pub state: Arc<OperatorState>,
    handle: JoinHandle<()>,
    shutdown_tx: watch::Sender<bool>,
}

impl WsServer {
    pub async fn start(port: u16, options: WsOptions) -> anyhow::Result<Self> {
        let state = Arc::new(OperatorState::new());
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let ctx = Arc::new(WsContext {
            events: state.clone(),
            options: Arc::new(options),
            shutdown_rx,
        });

        let app = Router::new().route("/ws", get(ws_handler)).with_state(ctx);

        let addr = format!("0.0.0.0:{port}");
        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|e| anyhow::anyhow!("failed to bind WebSocket listener on {addr}: {e}"))?;

        info!(%addr, "websocket server listening");
        let mut graceful_rx = shutdown_tx.clone().subscribe();
        let handle = tokio::spawn(async move {
            let result = axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .with_graceful_shutdown(async move {
                let _ = graceful_rx.changed().await;
            })
            .await;
            if let Err(e) = result {
                warn!(error = %e, "websocket server error");
            }
        });

        Ok(Self {
            state,
            handle,
            shutdown_tx,
        })
    }

    /// Cooperative shutdown (ADR-007 §2.4): notify subscribers with an
    /// `execution_terminated` event, then signal the serve task to drain
    /// in-flight connections and exit.
    pub async fn shutdown(self) {
        info!("websocket server draining subscribers");
        let _ = self.state.tx.send(OperatorEvent::AgentTerminated {
            message: "Phase Mirror Agent is shutting down".into(),
        });
        let _ = self.shutdown_tx.send(true);
        if let Err(e) = self.handle.await {
            warn!(error = %e, "websocket server task failed");
        }
    }
}

async fn ws_handler(
    State(ctx): State<Arc<WsContext>>,
    ws: WebSocketUpgrade,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Response {
    if let Some(keyring) = &ctx.options.keyring {
        let loopback_ok = ctx.options.loopback && addr.ip().is_loopback();
        let bearer = headers.get(header::AUTHORIZATION).and_then(strip_bearer);
        let auth_ok = bearer.is_some_and(|key| keyring.verify(key).is_some());
        if !(loopback_ok || auth_ok) {
            debug!(peer = %addr.ip(), "websocket upgrade rejected: unauthenticated");
            return (
                StatusCode::UNAUTHORIZED,
                [(header::WWW_AUTHENTICATE, "Bearer")],
            )
                .into_response();
        }
    }

    if let Some(limiter) = &ctx.options.ratelimit {
        let key = format!("ws:{}", addr.ip());
        if let Err(wait) = limiter.take(&key) {
            debug!(%key, "websocket upgrade rejected: rate limited");
            let mut res = StatusCode::TOO_MANY_REQUESTS.into_response();
            if let Ok(value) = wait
                .as_secs()
                .max(1)
                .to_string()
                .parse::<header::HeaderValue>()
            {
                res.headers_mut().insert(header::RETRY_AFTER, value);
            }
            return res;
        }
    }

    ws.on_upgrade(move |socket| {
        handle_socket(
            socket,
            ctx.events.clone(),
            ctx.options.metrics.clone(),
            ctx.shutdown_rx.clone(),
        )
    })
}

async fn handle_socket(
    socket: WebSocket,
    state: Arc<OperatorState>,
    metrics: Arc<Metrics>,
    mut shutdown: watch::Receiver<bool>,
) {
    metrics.ws_client_inc();
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.tx.subscribe();

    loop {
        tokio::select! {
            event = rx.recv() => {
                match event {
                    Ok(event) => {
                        let json = match serde_json::to_string(&event) {
                            Ok(j) => j,
                            Err(e) => {
                                warn!(error = %e, "failed to serialize websocket event");
                                continue;
                            }
                        };
                        metrics.ws_event_broadcast();
                        if sender.send(Message::Text(json)).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!(skipped = %n, "websocket consumer lagged behind broadcast");
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
            incoming = receiver.next() => {
                match incoming {
                    Some(Ok(Message::Close(_))) => {
                        break;
                    }
                    Some(Ok(_)) => {
                        debug!("ignoring client-originated websocket message");
                    }
                    Some(Err(e)) => {
                        warn!(error = %e, "websocket receive error");
                        break;
                    }
                    None => {
                        break;
                    }
                }
            }
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    info!("closing websocket client during shutdown");
                    let _ = sender.send(Message::Close(None)).await;
                    break;
                }
            }
        }
    }

    let _ = sender.close().await;
    metrics.ws_client_dec();
    debug!("websocket client disconnected");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn state_broadcasts_events_to_subscribers() {
        let state = Arc::new(OperatorState::new());
        let mut rx1 = state.tx.subscribe();
        let mut rx2 = state.tx.subscribe();

        state.emit(OperatorEvent::UserInput {
            text: "hello".into(),
        });

        let e1 = rx1.recv().await.unwrap();
        let e2 = rx2.recv().await.unwrap();
        match (e1, e2) {
            (OperatorEvent::UserInput { text: t1 }, OperatorEvent::UserInput { text: t2 }) => {
                assert_eq!(t1, "hello");
                assert_eq!(t2, "hello");
            }
            _ => panic!("expected user_input event"),
        }
    }

    #[tokio::test]
    async fn serializes_with_type_tag() {
        let event = OperatorEvent::ExecSuccess {
            action_id: "a1".into(),
            message: "ok".into(),
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["type"], "execution_success");
        assert_eq!(json["action_id"], "a1");
    }

    #[tokio::test]
    async fn emit_without_receivers_is_ok() {
        let state = Arc::new(OperatorState::new());
        state.emit(OperatorEvent::Retraction {
            message: "oops".into(),
        });
    }

    #[tokio::test]
    async fn agent_terminated_serializes_as_execution_terminated() {
        let event = OperatorEvent::AgentTerminated {
            message: "bye".into(),
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["type"], "execution_terminated");
    }
}
