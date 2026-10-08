import { describe, it, expect, beforeEach } from 'vitest';
import { PhaseMirrorAgent } from '../../src/index.js';
import { ArchivumStore } from '../../src/alp-nlp/archivum.js';
import { LexicalMapper } from '../../src/alp-nlp/lexer.js';
import { GrammarParser } from '../../src/alp-nlp/parser.js';
import { PolicyEngine } from '../../src/alp-nlp/policy.js';
import { tmpdir } from 'os';
import { join } from 'path';

describe('PhaseMirrorAgent integration', () => {
  let agent: PhaseMirrorAgent;

  beforeEach(() => {
    agent = new PhaseMirrorAgent();
  });

  it('processes valid query end-to-end', async () => {
    const response = await agent.analyze('deploy service');
    
    expect(response).toContain('ALP-NLP');
    expect(response).toContain('COHERENT');
    expect(response).toContain('Witness:');
    expect(response).toContain('Contractivity Score');
  });

  it('rejects unknown vocabulary', async () => {
    const response = await agent.analyze('unknown jargon xyz');
    
    expect(response).toContain('VETO');
    expect(response).toContain('Unknown word');
  });

  it('produces witness with all required fields', async () => {
    const mapper = new LexicalMapper();
    const parser = new GrammarParser(mapper);
    const policy = new PolicyEngine(mapper, {
      state_norm: 1.0,
      drift_rate: 0.0,
      contractivity_score: 0.5,
      kill_switch_active: false,
      critique_results: Array(10).fill({ passed: true }),
      prime_gates: [{ gate_value: 2 }, { gate_value: 3 }, { gate_value: 5 }],
    });

    const result = parser.parse('verify adr');
    expect(result.word).not.toBeNull();
    
    const witness = policy.generateWitness('test-1', result.word, { allowed: true, reason: 'Admitted', violations: [] });
    
    expect(witness.witness_id).toBeDefined();
    expect(witness.action_id).toBe('test-1');
    expect(witness.timestamp).toBeDefined();
    expect(witness.veto_status).toBe('admitted');
    expect(witness.contractivity_score).toBeDefined();
    expect(witness.compliance_evidence).toBeDefined();
    expect(witness.execution_receipt).toBeDefined();
    expect(witness.execution_receipt.contractivity_score).toBeDefined();
  });

  it('vetoed witness has negative contractivity score', async () => {
    const mapper = new LexicalMapper();
    const parser = new GrammarParser(mapper);
    const policy = new PolicyEngine(mapper, {
      state_norm: 1.0,
      drift_rate: 0.0,
      contractivity_score: 0.5,
      kill_switch_active: false,
      critique_results: Array(10).fill({ passed: true }),
      prime_gates: [{ gate_value: 2 }, { gate_value: 3 }, { gate_value: 5 }],
    });

    const witness = policy.generateWitness('test-2', null, {
      allowed: false,
      reason: 'Rejected',
      violations: ['L0_02'],
    });

    expect(witness.veto_status).toBe('vetoed');
    expect(witness.contractivity_score).toBe(0);
  });
});

describe('ArchivumStore', () => {
  it('writes and reads witnesses', async () => {
    const tempDir = join(tmpdir(), `alp-nlp-test-${Date.now()}`);
    const archivum = new ArchivumStore(tempDir);
    
    const witness = {
      witness_id: 'test-witness-1',
      action_id: 'test-action',
      timestamp: new Date().toISOString(),
      compliance_evidence: 'test',
      execution_receipt: {
        status: 'completed',
        prime_indices: [2, 3],
        r_sc: 1.5,
        c: 0.1,
        contractivity_score: 0.9,
      },
      contractivity_score: 0.9,
      veto_status: 'admitted' as const,
      violations: [],
    };
    
    await archivum.writeWitness(witness);
    const witnesses = await archivum.readWitnesses();
    
    expect(witnesses).toHaveLength(1);
    expect(witnesses[0].witness_id).toBe('test-witness-1');
  });
});