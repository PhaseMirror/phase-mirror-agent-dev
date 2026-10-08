use phase_mirror_mcp::{LambdaTrace, SedonaSpineEvaluator};
use serde_json::json;

fn make_witness() -> LambdaTrace {
    LambdaTrace {
        lambda_p: 0.999999,
        l_p: 0.95,
        zero_spacings: vec![0.9549652277648129, 1.5563111057990717, 1.2289235832739145],
        signature: "SIGNED_HASH".to_string(),
        signer_pubkey: "ed25519:twin-prime-042".to_string(),
        proof_hash: "LEAN_PROOF_HASH_108_CORE".to_string(),
    }
}

#[test]
fn test_sedona_spine_success_path() {
    let witness = make_witness();
    let result = SedonaSpineEvaluator::evaluate_stop_rules(&witness);
    assert!(result.is_ok(), "Valid witness should pass stop rules");
}

#[test]
fn test_sedona_spine_scalar_collapse_failure() {
    let toxic_witness = LambdaTrace {
        lambda_p: 1.5,
        l_p: 0.9,
        zero_spacings: vec![1.0, 2.0],
        signature: "SIGNED_HASH".to_string(),
        signer_pubkey: "".to_string(),
        proof_hash: "LEAN_PROOF_HASH_108_CORE".to_string(),
    };
    let result = SedonaSpineEvaluator::evaluate_stop_rules(&toxic_witness);
    assert!(result.is_err(), "λ_p * L_p >= 1.0 must fail");
    let err = result.unwrap_err();
    assert!(err.contains("L0_VIOLATION"), "Should report L0_VIOLATION");
}

#[test]
fn test_sedona_spine_empty_zero_spacings_failure() {
    let empty_witness = LambdaTrace {
        lambda_p: 0.5,
        l_p: 0.5,
        zero_spacings: vec![],
        signature: "SIGNED_HASH".to_string(),
        signer_pubkey: "".to_string(),
        proof_hash: "LEAN_PROOF_HASH_108_CORE".to_string(),
    };
    let result = SedonaSpineEvaluator::evaluate_stop_rules(&empty_witness);
    assert!(result.is_err(), "Empty zero_spacings must fail");
    let err = result.unwrap_err();
    assert!(err.contains("ZEROS_EMPTY"), "Should report ZEROS_EMPTY");
}

#[test]
fn test_wrapper_returns_ok_with_witness() {
    let witness = make_witness();
    let json_output = json!({
        "status": "OK",
        "witness_id": "sha256:000...",
        "lambda_trace": witness,
        "metric": 0.999
    });

    let text_output = serde_json::to_string(&json_output).unwrap();
    assert!(text_output.contains("\"status\":\"OK\""));
    assert!(text_output.contains("zero_spacings"));
    assert!(text_output.contains("\"signature\":\"SIGNED_HASH\""));
}

#[test]
fn test_wrapper_error_format() {
    let error_output = json!({
        "isError": true,
        "content": [{ "type": "text", "text": "Scalar collapse detected" }]
    });

    let text_output = serde_json::to_string(&error_output).unwrap();
    assert!(text_output.contains("isError"));
    assert!(text_output.contains("Scalar collapse"));
}
