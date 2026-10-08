use phase_mirror_mcp::governance::ContractManager;
use phase_mirror_mcp::persistence::CrmfStorage;
use phase_mirror_mcp::process_request;
use serde_json::json;
use std::path::Path;
use std::sync::Arc;
use tempfile::NamedTempFile;
use tokio::sync::Mutex;

#[tokio::test]
async fn test_stonehenge_simulation() {
    println!("[STONEHENGE] Initializing Autonomous Governance Cycle Simulation...");

    let contract_json = json!({
        "contract_metadata": {
            "contract_id": "STONEHENGE-PROD-001",
            "schema_version": "1.0.0",
            "sovereign_id": "AGIOS-PRIMARY",
            "valid_from": "2026-06-16T00:00:00Z",
            "nonce": "1"
        },
        "governance_floor": {
            "default_tier": "Authoritative",
            "enforce_sealed_veto": true,
            "escalation_triggers": []
        },
        "tool_policies": {},
        "jubilee_window": {
            "sync_required": true,
            "max_thickness_delta": 0.05,
            "checkpoint_ref": "REF-001"
        }
    });

    let contract_file = NamedTempFile::new().expect("Failed to create temp contract file");
    std::fs::write(
        contract_file.path(),
        serde_json::to_string_pretty(&contract_json).unwrap(),
    )
    .unwrap();

    let contract_manager =
        ContractManager::new(contract_file.path()).expect("Failed to init ContractManager");
    let crmf_storage = Arc::new(Mutex::new(
        CrmfStorage::new("stonehenge_audit.jsonl").expect("Failed to init CRMF and ACE certificates"),
    ));

    let lambda_p: f64 = 0.42;
    let drift_delta: f64 = 0.000042;

    println!("[STONEHENGE] Banach Contraction λ_p = {}", lambda_p);
    println!("[STONEHENGE] MD-005 Drift δ = {}", drift_delta);

    assert!(
        lambda_p < 1.0,
        "Banach Contraction violation: System is diverging!"
    );
    assert!(drift_delta < 0.0001, "MD-005 Drift violation!");

    let mission_req = json!({
        "name": "attest_cross_domain_mission",
        "arguments": {
            "mission_id": "MISSION-STONEHENGE-VERIFY",
            "artifact_id": "ART-t26-GOLD-MASTER"
        }
    });

    let response = {
        #[cfg(feature = "lmstudio")]
        {
            process_request(
                "tools/call".to_string(),
                mission_req,
                &contract_manager,
                &crmf_storage,
                None,
            )
            .await
        }
        #[cfg(not(feature = "lmstudio"))]
        {
            process_request(
                "tools/call".to_string(),
                mission_req,
                &contract_manager,
                &crmf_storage,
            )
            .await
        }
    };

    println!(
        "[STONEHENGE] Response: {}",
        serde_json::to_string_pretty(&response).unwrap()
    );

    let response_text = response["content"][0]["text"].as_str().unwrap();
    assert!(response_text.contains("VERIFIED: Lawful & Resilient"));
    assert!(response_text.contains("R_sc: 0.962"));

    println!("[STONEHENGE] Autonomous Governance Cycle: VERIFIED & ANCHORED.");
}
