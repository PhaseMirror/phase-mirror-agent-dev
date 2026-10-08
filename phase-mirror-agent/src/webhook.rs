use axum::{routing::post, Router, extract::Json, response::IntoResponse};
use serde::Deserialize;
use std::process::Command;

#[derive(Deserialize)]
pub struct GithubWebhookPayload {
    pub ref_name: Option<String>,
    pub commits: Option<Vec<serde_json::Value>>,
}

pub async fn handle_github_webhook(Json(payload): Json<GithubWebhookPayload>) -> impl IntoResponse {
    println!("Received webhook push event");
    
    // Trigger Phase Mirror MCP to run prismpm_verify
    let result = Command::new("cargo")
        .args(&["run", "-p", "phase-mirror-mcp", "--", "--run-tool", "prismpm_verify"])
        .output();
        
    match result {
        Ok(out) => {
            if out.status.success() {
                println!("Automated evolution step succeeded.");
                "OK"
            } else {
                println!("Evolution step failed.");
                "FAIL"
            }
        }
        Err(_) => "ERR"
    }
}

pub fn webhook_routes() -> Router {
    Router::new().route("/webhook/github", post(handle_github_webhook))
}
