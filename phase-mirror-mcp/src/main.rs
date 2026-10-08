use phase_mirror_mcp::{ContractManager, CrmfStorage};
use serde_json::json;
use std::io;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{Mutex, oneshot};

#[cfg(feature = "lmstudio")]
use phase_mirror_mcp::LmStudioClient;

#[tokio::main]
async fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.contains(&"--register-mcp".to_string()) {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let mcp_path = Path::new(&home).join(".lmstudio").join("mcp.json");

        let current_exe = std::env::current_exe()?;
        let command = current_exe.to_string_lossy().to_string();

        let mut servers = std::collections::BTreeMap::new();
        if mcp_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&mcp_path) {
                if let Ok(mut map) = serde_json::from_str::<
                    std::collections::BTreeMap<String, serde_json::Value>,
                >(&content)
                {
                    if let Some(mcp_servers) = map.remove("mcpServers") {
                        if let Ok(s) = serde_json::from_value(mcp_servers) {
                            servers = s;
                        }
                    }
                }
            }
        }

        servers.insert(
            "phase-mirror-mcp".to_string(),
            json!({
                "command": command,
                "args": []
            }),
        );

        let updated = json!({ "mcpServers": servers });
        std::fs::write(&mcp_path, serde_json::to_string_pretty(&updated).unwrap())?;
        eprintln!("Registered phase-mirror-mcp in {}", mcp_path.display());
        return Ok(());
    }

    let is_ws = args.iter().any(|arg| arg == "--ws");

    let contract_path = Path::new("mcp-contract.json");
    let contract_manager = match ContractManager::new(contract_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Failed to initialize ContractManager: {}", e);
            std::process::exit(1);
        }
    };

    let crmf_storage = Arc::new(Mutex::new(CrmfStorage::new("crmf_audit.jsonl")?));

    #[cfg(feature = "lmstudio")]
    let lmstudio_client = {
        let base_url = std::env::var("LMSTUDIO_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:1234/v1".to_string());
        Arc::new(LmStudioClient::new(base_url))
    };

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        eprintln!("Received SIGINT, shutting down gracefully...");
        let _ = shutdown_tx.send(());
    });

    if is_ws {
        #[cfg(not(feature = "lmstudio"))]
        {
            let server =
                phase_mirror_mcp::transport::ws::run_server(3001, contract_manager, crmf_storage);
            tokio::select! {
                result = server => result,
                _ = shutdown_rx => Ok(())
            }
        }
        #[cfg(feature = "lmstudio")]
        {
            let server = phase_mirror_mcp::transport::ws::run_server(
                3001,
                contract_manager,
                crmf_storage,
                Some(lmstudio_client),
            );
            tokio::select! {
                result = server => result,
                _ = shutdown_rx => Ok(())
            }
        }
    } else {
        #[cfg(not(feature = "lmstudio"))]
        {
            let server = phase_mirror_mcp::run_stdio_server(contract_manager, crmf_storage);
            tokio::select! {
                result = server => result,
                _ = shutdown_rx => Ok(())
            }
        }
        #[cfg(feature = "lmstudio")]
        {
            let server = phase_mirror_mcp::run_stdio_server(
                contract_manager,
                crmf_storage,
                Some(lmstudio_client),
            );
            tokio::select! {
                result = server => result,
                _ = shutdown_rx => Ok(())
            }
        }
    }
}
