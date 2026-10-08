import subprocess
import json
import time
import os

BINARY_PATH = "/home/multiplicity/Multiplicity/Phase Mirror/target/release/phase-mirror-gpt"
PROJECT_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LOG_FILE = os.path.join(PROJECT_ROOT, "archivum.log")

def run_tool_call(payload):
    process = subprocess.Popen(
        [BINARY_PATH],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        cwd=PROJECT_ROOT
    )
    input_str = json.dumps(payload) + "\n"
    stdout, stderr = process.communicate(input=input_str)
    # Give background task a moment to flush if needed
    time.sleep(0.5) 
    return stdout, stderr

def test_durability():
    print("=== Testing Λ-Archivum Durability (WAL) ===")
    if os.path.exists(LOG_FILE):
        os.remove(LOG_FILE)
    
    # We don't have a direct 'commit' tool exposed yet in the MCP contract, 
    # but the ADR-003 says p=7 Data Lineage is tracked.
    # Currently, the transport dispatch_tool (in main.rs) was supposed to commit a receipt.
    # Let's check main.rs again.
    
    print("Invoking validate_l0_invariants to trigger lineage record...")
    payload = {
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "validate_l0_invariants",
            "arguments": {
                "event_type": "pull_request",
                "tier": "tier1",
                "permission_bits": 15,
                "schema_signature": 7,
                "expected_schema": 3
            }
        },
        "id": 1
    }
    
    stdout, stderr = run_tool_call(payload)
    print(f"Stdout: {stdout.strip()}")
    
    if os.path.exists(LOG_FILE):
        print(f"SUCCESS: {LOG_FILE} created.")
        with open(LOG_FILE, 'r') as f:
            print(f"Log content: {f.read().strip()}")
    else:
        print(f"FAILURE: {LOG_FILE} NOT created.")

def test_error_handling():
    print("\n=== Testing Error Handling ===")
    
    print("Testing malformed JSON...")
    stdout, stderr = run_tool_call("NOT JSON")
    # Should skip malformed lines per main.rs
    print(f"Stderr (Expect 'Parse error'): {stderr.strip()}")

    print("Testing non-existent tool...")
    payload = {
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {"name": "ghost_tool", "arguments": {}},
        "id": 2
    }
    stdout, stderr = run_tool_call(payload)
    print(f"Stdout (Expect Method not found): {stdout.strip()}")

if __name__ == "__main__":
    test_durability()
    test_error_handling()
