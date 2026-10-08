import { randomUUID } from 'node:crypto';
import { witnessHash } from './chain.js';

export interface ExecutionReceipt {
  status: string;
  prime_indices: number[];
  r_sc: number;
  c: number;
  contractivity_score: number;
}

export interface UnifiedWitness {
  witness_id: string;
  action_id: string;
  timestamp: string;
  compliance_evidence: string;
  execution_receipt: ExecutionReceipt;
  contractivity_score: number;
  veto_status: 'admitted' | 'vetoed';
  witness_hash?: string;
  p_lineage?: string;
  violations: string[];
  sequence?: number;
  prev_hash?: string;
  entry_hash?: string;
}

export interface WitnessOptions {
  actionId: string;
  primeIndices: number[];
  r_sc: number;
  c: number;
  contractivityScore: number;
  violations: string[];
  witnessHash?: string;
  /** Canonical action JSON used to derive witness_hash when witnessHash is absent. */
  canonicalAction?: string;
  idempotencyKey?: string;
  pLineage?: string;
}

export function generateWitnessId(): string {
  return randomUUID();
}

export function createWitness(options: WitnessOptions): UnifiedWitness {
  const complianceEvidence = options.violations.length === 0
    ? `R_sc=${options.r_sc.toFixed(4)}, c=${options.c.toFixed(4)}`
    : `violations=${options.violations.length}`;
  const timestamp = new Date().toISOString();
  const witnessHashValue = options.witnessHash
    ?? (options.canonicalAction
      ? witnessHash(options.canonicalAction, options.idempotencyKey ?? '', timestamp)
      : undefined);

  return {
    witness_id: generateWitnessId(),
    action_id: options.actionId,
    timestamp,
    compliance_evidence: complianceEvidence,
    execution_receipt: {
      status: options.violations.length === 0 ? 'completed' : 'rejected',
      prime_indices: options.primeIndices,
      r_sc: options.r_sc,
      c: options.c,
      contractivity_score: options.contractivityScore,
    },
    contractivity_score: options.contractivityScore,
    veto_status: options.violations.length === 0 ? 'admitted' : 'vetoed',
    witness_hash: witnessHashValue,
    p_lineage: options.pLineage,
    violations: options.violations,
  };
}
