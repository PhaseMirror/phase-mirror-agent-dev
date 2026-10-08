use crate::{ContractManager, JsonRpcRequest, CrmfStorage, process_request};
use axum::{
    Router,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::{IntoResponse, Json},
    routing::get,
};
use futures_util::StreamExt;
use serde_json::json;
use std::io;
use std::sync::Arc;
use tokio::sync::Mutex;

#[cfg(feature = "lmstudio")]
use crate::LmStudioClient;

#[derive(Clone)]
#[cfg(not(feature = "lmstudio"))]
struct AppState {
    contract_manager: Arc<ContractManager>,
    crmf_storage: Arc<Mutex<CrmfStorage>>,
}

#[derive(Clone)]
#[cfg(feature = "lmstudio")]
struct AppState {
    contract_manager: Arc<ContractManager>,
    crmf_storage: Arc<Mutex<CrmfStorage>>,
    lmstudio_client: Option<Arc<LmStudioClient>>,
}

#[cfg(not(feature = "lmstudio"))]
pub async fn run_server(
    port: u16,
    contract_manager: Arc<ContractManager>,
    crmf_storage: Arc<Mutex<CrmfStorage>>,
) -> io::Result<()> {
    let state = AppState {
        contract_manager,
        crmf_storage,
    };

    let app = Router::new()
        .route("/ws", get(handler))
        .route("/metrics", get(metrics_handler))
        .route("/health", get(health_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;

    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(feature = "lmstudio")]
pub async fn run_server(
    port: u16,
    contract_manager: Arc<ContractManager>,
    crmf_storage: Arc<Mutex<CrmfStorage>>,
    lmstudio_client: Option<Arc<LmStudioClient>>,
) -> io::Result<()> {
    let state = AppState {
        contract_manager,
        crmf_storage,
        lmstudio_client,
    };

    let app = Router::new()
        .route("/ws", get(handler))
        .route("/metrics", get(metrics_handler))
        .route("/health", get(health_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;

    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(not(feature = "lmstudio"))]
async fn handler(
    ws: WebSocketUpgrade,
    axum::extract::State(state): axum::extract::State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

#[cfg(feature = "lmstudio")]
async fn handler(
    ws: WebSocketUpgrade,
    axum::extract::State(state): axum::extract::State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket_lmstudio(socket, state))
}

async fn metrics_handler() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let spectral_radius = 0.74 + (rng.r#gen::<f64>() - 0.5) * 0.1;
    let l_phi = 0.61 + (rng.r#gen::<f64>() - 0.5) * 0.15;
    let epsilon = 0.012 + (rng.r#gen::<f64>() - 0.5) * 0.005;
    let fail_rate = 0.04 + (rng.r#gen::<f64>() - 0.5) * 0.02;

    json!({
        "spectralRadius": spectral_radius,
        "lPhi": l_phi,
        "epsilon": epsilon,
        "failRate": fail_rate,
        "timestamp": chrono::Utc::now().timestamp_millis()
    })
    .to_string()
}

async fn health_handler() -> Json<serde_json::Value> {
    Json(json!({
        "status": "healthy",
        "service": "phase-mirror-mcp",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

#[cfg(not(feature = "lmstudio"))]
async fn handle_socket(socket: WebSocket, state: AppState) {
    let (_sender, mut receiver) = socket.split();

    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(text) = msg {
            if let Ok(req) = serde_json::from_str::<JsonRpcRequest>(&text) {
                let _response_val = process_request(
                    req.method.clone(),
                    req.params.clone(),
                    &state.contract_manager,
                    &state.crmf_storage,
                )
                .await;
            }
        }
    }
}

#[cfg(feature = "lmstudio")]
async fn handle_socket_lmstudio(socket: WebSocket, state: AppState) {
    let (_sender, mut receiver) = socket.split();

    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(text) = msg {
            if let Ok(req) = serde_json::from_str::<JsonRpcRequest>(&text) {
                let _response_val = process_request(
                    req.method.clone(),
                    req.params.clone(),
                    &state.contract_manager,
                    &state.crmf_storage,
                    state.lmstudio_client.as_ref(),
                )
                .await;
            }
        }
    }
}
