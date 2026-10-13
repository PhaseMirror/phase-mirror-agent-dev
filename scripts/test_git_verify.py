#!/usr/bin/env python3
"""
Test Suite for git verify (P2-P3 signature requirements / Lever H2)
"""

import json
import os
import shutil
import subprocess
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
GIT_VERIFY_SCRIPT = REPO_ROOT / "scripts" / "git-verify"

def run_git_verify(args, cwd=REPO_ROOT, env=None):
    cmd = [str(GIT_VERIFY_SCRIPT)] + args
    return subprocess.run(cmd, cwd=cwd, capture_output=True, text=True, env=env)

def test_p2_p3_lifecycle():
    print("Running P2-P3 signature lifecycle test...")
    with tempfile.TemporaryDirectory() as tmpdir:
        tmp_path = Path(tmpdir)
        gov_dir = tmp_path / "governance"
        test_env = os.environ.copy()
        test_env["PHASE_MIRROR_GOVERNANCE_DIR"] = str(gov_dir)

        test_doc = tmp_path / "TEST_ADR_003.md"
        test_doc.write_text("# ADR-003: Test Document\n\nStatus: Proposed\n\nCanonical text.\n", encoding="utf-8")
        orig_bytes = test_doc.read_bytes()

        # Step 1: Initial verification (0 signatures)
        res = run_git_verify([str(test_doc), "--json"], env=test_env)
        assert res.returncode == 0, f"Expected 0, got {res.returncode}: {res.stderr}"
        data = json.loads(res.stdout)
        r0 = data["results"][0]
        assert r0["status"] == "Proposed", f"Expected Proposed, got {r0['status']}"
        assert r0["distinct_verified_count"] == 0
        assert not r0["verified"]
        print("  ✓ Step 1: 0 signatures -> Status: Proposed")

        # Step 2: Generate Key 1 (P2 keyholder)
        p2_prefix = str(tmp_path / "p2_key")
        res_kg1 = run_git_verify(["--keygen", p2_prefix, "--signer", "Legal Person 1 (P2)"], env=test_env)
        assert res_kg1.returncode == 0
        p2_key = Path(f"{p2_prefix}.key")
        p2_pub = Path(f"{p2_prefix}.pub")
        assert p2_key.exists() and p2_pub.exists()
        print("  ✓ Step 2: Generated P2 governance keypair")

        # Step 3: Generate Key 2 (P3 keyholder)
        p3_prefix = str(tmp_path / "p3_key")
        res_kg2 = run_git_verify(["--keygen", p3_prefix, "--signer", "Legal Person 2 (P3)"], env=test_env)
        assert res_kg2.returncode == 0
        p3_key = Path(f"{p3_prefix}.key")
        p3_pub = Path(f"{p3_prefix}.pub")
        assert p3_key.exists() and p3_pub.exists()
        print("  ✓ Step 3: Generated P3 governance keypair")

        # Step 4: Sign with P2 only (1/2 signatures)
        res_sign1 = run_git_verify(["--sign", str(test_doc), "--key", str(p2_key), "--signer", "Legal Person 1 (P2)"], env=test_env)
        assert res_sign1.returncode == 0
        assert test_doc.read_bytes() == orig_bytes, "Detached invariant violated: Document body was modified!"

        res_check1 = run_git_verify([str(test_doc), "--json"], env=test_env)
        data1 = json.loads(res_check1.stdout)
        r1 = data1["results"][0]
        assert r1["status"] == "Proposed", f"Expected Proposed with 1 signature, got {r1['status']}"
        assert r1["distinct_verified_count"] == 1
        assert not r1["verified"]
        print("  ✓ Step 4: 1/2 signatures -> Status: Proposed (Detached invariant preserved)")

        # Step 5: Duplicate sign with P2 again (testing distinct key requirement)
        res_sign1_dup = run_git_verify(["--sign", str(test_doc), "--key", str(p2_key), "--signer", "Legal Person 1 (P2)"], env=test_env)
        assert res_sign1_dup.returncode == 0
        res_check_dup = run_git_verify([str(test_doc), "--json"], env=test_env)
        data_dup = json.loads(res_check_dup.stdout)
        r_dup = data_dup["results"][0]
        assert r_dup["distinct_verified_count"] == 1, "Duplicate key cannot count as second cryptographic identity!"
        assert r_dup["status"] == "Proposed"
        print("  ✓ Step 5: Duplicate key cannot satisfy second identity requirement")

        # Step 6: Sign with P3 (2/2 distinct signatures)
        res_sign2 = run_git_verify(["--sign", str(test_doc), "--key", str(p3_key), "--signer", "Legal Person 2 (P3)"], env=test_env)
        assert res_sign2.returncode == 0
        assert test_doc.read_bytes() == orig_bytes, "Detached invariant violated: Document body was modified!"

        res_check2 = run_git_verify([str(test_doc), "--json"], env=test_env)
        data2 = json.loads(res_check2.stdout)
        r2 = data2["results"][0]
        assert r2["status"] == "Accepted", f"Expected Accepted with 2 signatures, got {r2['status']}"
        assert r2["distinct_verified_count"] == 2
        assert r2["verified"]
        print("  ✓ Step 6: 2/2 distinct signatures -> Status: Accepted (H2 lever satisfied)")

        # Step 7: Tamper test: Alter 1 byte of the document
        test_doc.write_text("# ADR-003: Altered Document\n\nStatus: Proposed\n\nCanonical text.\n", encoding="utf-8")
        res_tamper = run_git_verify([str(test_doc), "--json"], env=test_env)
        data_tamper = json.loads(res_tamper.stdout)
        r_tamper = data_tamper["results"][0]
        assert r_tamper["distinct_verified_count"] == 0, "Altered document must not verify against old signatures!"
        assert r_tamper["status"] == "Proposed"
        print("  ✓ Step 7: Document tamper test passed (Digest mismatch invalidates signatures)")

        # Step 8: Test check-unfreeze invariant
        res_unfreeze = run_git_verify(["--check-unfreeze"], env=test_env)
        assert res_unfreeze.returncode == 0
        assert "SUSPENDED (DO NOT RUN)" in res_unfreeze.stdout
        assert "INVARIANT PRESERVED" in res_unfreeze.stdout
        print("  ✓ Step 8: Act 2 Runtime Unfreeze suspension check verified")

    print("\nALL P2-P3 SIGNATURE VERIFICATION TESTS PASSED!\n")

if __name__ == "__main__":
    test_p2_p3_lifecycle()
