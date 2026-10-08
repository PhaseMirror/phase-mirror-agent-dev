// phaseMirrorAlpCnlBridge.ts
// Bridges the ALP Lean proofs (compiled to WASM) with the CNL dashboard.

import init, {
  evaluate_esi_risk_wasm,
} from './pkg/sedona_spine.js';

let wasmInitialized = false;

async function ensureWasm(): Promise<void> {
  if (!wasmInitialized) {
    await init('./pkg/sedona_spine_bg.wasm');
    wasmInitialized = true;
  }
}

export type Action = {
  kind: string;
  payload: string;
};

export type Constitution = {
  rules: { clause: string; prime_index: number; allowed: boolean }[];
};

export async function validateCommand(
  command: string,
  constitution: Constitution,
): Promise<boolean> {
  await ensureWasm();

  // Mock mapping from CNL command to Sedona Spine ESI Inputs
  // In a real app, the CNL parser would extract these values.
  const esiInputs = {
    spoliation_potential: 0.1,
    preservation_urgency: 0.2,
    volume_estimate_gb: 10.0,
  };
  
  // Use a prime factor derived from the constitution (e.g. first rule's prime_index)
  const p_factor = constitution.rules.length > 0 ? constitution.rules[0].prime_index : 2;
  const sigma = 2.0;

  try {
    const witness = evaluate_esi_risk_wasm(esiInputs, p_factor, sigma);
    // Veto the command if the resulting risk level is Critical
    return witness.compilation_result.risk_level !== 'Critical';
  } catch (err) {
    console.error('Validation failed:', err);
    return false; // Fail secure
  }
}
