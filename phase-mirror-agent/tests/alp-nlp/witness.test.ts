import { describe, it, expect } from 'vitest';
import { createWitness } from '../../src/alp-nlp/witness.js';

describe('createWitness hash derivation', () => {
  it('derives witness_hash from canonical action when not supplied', () => {
    const witness = createWitness({
      actionId: 'act-1',
      primeIndices: [2, 3, 5],
      r_sc: 1.2345,
      c: 0.1,
      contractivityScore: 0.9,
      violations: [],
      canonicalAction: '{"action":"deploy","replicas":3,"service":"web-service","target":"cluster"}',
      idempotencyKey: 'client-key-1',
    });
    expect(witness.witness_hash).toMatch(/^[0-9a-f]{64}$/);
    expect(witness.execution_receipt.status).toBe('completed');
    expect(witness.veto_status).toBe('admitted');
  });

  it('prefers an explicit witnessHash', () => {
    const witness = createWitness({
      actionId: 'act-2',
      primeIndices: [7],
      r_sc: 0.5,
      c: 1.2,
      contractivityScore: 0,
      violations: ['L0_04: non-prime'],
      witnessHash: 'abc123',
    });
    expect(witness.witness_hash).toBe('abc123');
    expect(witness.execution_receipt.status).toBe('rejected');
    expect(witness.veto_status).toBe('vetoed');
  });
});
