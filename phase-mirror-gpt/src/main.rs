use notify::{Config, RecursiveMode, Watcher};
use phase_mirror_gpt::archivum::{ArchivumLedger, DistributedSyncOracle};
use phase_mirror_gpt::domain_invariants::SemanticPolicy;
use phase_mirror_gpt::mirror::MirrorCoordinator;
use phase_mirror_gpt::telemetry;
use phase_mirror_gpt::transport::McpTransportWrapper;
use phase_mirror_gpt::triple_lock::TripleLockSuite;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::signal;
use tokio::sync::Mutex as TokioMutex;
use tokio::sync::mpsc as tokio_mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut reader = BufReader::new(stdin).lines();

    let shutdown = Arc::new(tokio::sync::Notify::new());
    let shutdown_clone = shutdown.clone();

    tokio::spawn(async move {
        let _ = signal::ctrl_c().await;
        eprintln!("Received SIGINT, shutting down gracefully...");
        shutdown_clone.notify_one();
    });

    // Initialize Λ-Archivum with Async WAL Persistence (ADR-003)
    let mut ledger = ArchivumLedger::new();
    let log_path = PathBuf::from("archivum.log");
    ledger.init_persistence(log_path).await?;

    // Initialize Telemetry Baseline
    let telemetry = Arc::new(Mutex::new(telemetry::LiveTelemetryOracle::init_baseline()));

    // Initialize Dynamic Semantic Policy (L1)
    let policy = Arc::new(SemanticPolicy::new());
    let policy_path = PathBuf::from("config/policy.toml");
    if policy_path.exists() {
        let toml_str = std::fs::read_to_string(&policy_path)?;
        policy.load_from_toml(&toml_str)?;
    }

    // Initialize Shared Ledger State
    let ledger_arc = Arc::new(TokioMutex::new(ledger));

    // Initialize the Mirror Coordinator
    let mirror = MirrorCoordinator::new(
        Arc::clone(&telemetry),
        Arc::clone(&ledger_arc),
        Arc::clone(&policy),
    );

    // Initialize the Triple-Lock Suite (ADR-006)
    let triple_lock = TripleLockSuite::new(
        Arc::clone(&telemetry),
        Arc::clone(&ledger_arc),
        Arc::clone(&policy),
    );

    // Initialize the high-integrity Tool Transport
    let sync_oracle = DistributedSyncOracle {
        tracked_leaves: BTreeMap::new(),
    };

    // Create the transport wrapper with the shared state
    let transport =
        McpTransportWrapper::init_internal(sync_oracle, ledger_arc, mirror, triple_lock);

    // Setup Multi-Watcher for Legal Docs and Policy (ADR-003 / ADR-005)
    let (drift_tx, mut drift_rx) = tokio_mpsc::channel(1);
    let telemetry_clone = Arc::clone(&telemetry);
    let policy_clone = Arc::clone(&policy);
    let ledger_log_clone = Arc::clone(&transport.ledger);
    let shutdown_drift = shutdown.clone();

    let mut watcher = notify::RecommendedWatcher::new(
        move |res: notify::Result<notify::Event>| {
            if res.is_ok() {
                let _ = drift_tx.blocking_send(());
            }
        },
        Config::default(),
    )?;

    watcher.watch(
        PathBuf::from("docs/legal/").as_path(),
        RecursiveMode::Recursive,
    )?;
    watcher.watch(PathBuf::from("config/").as_path(), RecursiveMode::Recursive)?;

    // Background Drift & Policy Mutation Handler
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = drift_rx.recv() => {
                    // 1. Refresh Legal Telemetry
                    {
                        let mut t = telemetry_clone.lock().expect("Failed to lock telemetry");
                        t.refresh_verification_states();
                    }

                    // 2. Hot-Reload Semantic Policy
                    let p_path = PathBuf::from("config/policy.toml");
                    if p_path.exists() {
                        if let Ok(toml_str) = std::fs::read_to_string(&p_path) {
                            if let Err(e) = policy_clone.load_from_toml(&toml_str) {
                                eprintln!("\n[POLICY ERROR] Failed to reload policy.toml: {}", e);
                            } else {
                                eprintln!("\n[POLICY MUTATION] Semantic policy successfully hot-reloaded.");
                                let mut l = ledger_log_clone.lock().await;
                                let _ = l.commit_event(
                                    "policy_mutation",
                                    "policy.toml".to_string(),
                                    toml_str.as_bytes(),
                                );
                            }
                        }
                    }

                    let t = telemetry_clone.lock().unwrap();
                    let (status, blocked) = t.determine_escalation_vector();
                    eprintln!("Status: {}", status);
                    if blocked {
                        eprintln!("CAUTION: Fail-Closed Gate is ACTIVE.");
                    }
                }
                _ = shutdown_drift.notified() => {
                    eprintln!("Drift handler shutting down...");
                    break;
                }
            }
        }
    });

    // Initial status log
    {
        let t = telemetry.lock().unwrap();
        let (status, blocked) = t.determine_escalation_vector();
        eprintln!("=== Phase Mirror GPT Kernel Online ===");
        eprintln!("Status: {}", status);
        if blocked {
            eprintln!("CAUTION: Fail-Closed Gate is ACTIVE. Compliance < 100%");
        }
    }

    // Main MCP Event Loop (ADR-004)
    loop {
        tokio::select! {
            result = reader.next_line() => {
                match result {
                    Ok(Some(line)) => {
                        if line.trim().is_empty() {
                            continue;
                        }
                        let response = transport.handle_mcp_call(&line).await;
                        stdout.write_all(response.as_bytes()).await?;
                        stdout.write_all(b"\n").await?;
                        stdout.flush().await?;
                    }
                    Ok(None) => {
                        eprintln!("stdin closed, exiting...");
                        break;
                    }
                    Err(e) => {
                        eprintln!("stdin error: {}", e);
                        break;
                    }
                }
            }
            _ = shutdown.notified() => {
                eprintln!("Main loop shutting down...");
                break;
            }
        }
    }

    Ok(())
}
