import { describe, it, expect, beforeEach } from 'vitest';
import { PolicyEngine, validatePrimeGates, AdmissibilityReport } from '../../src/alp-nlp/policy.js';
import { LexicalMapper } from '../../src/alp-nlp/lexer.js';
import { UnifiedWitness, createWitness, WitnessOptions } from '../../src/alp-nlp/witness.js';

describe('validatePrimeGates', () => {
  it('validates all prime indices', () => {
    const result = validatePrimeGates([2, 3, 5, 7, 11]);
    expect(result.valid).toBe(true);
    expect(result.violations).toHaveLength(0);
  });

  it('detects non-prime indices', () => {
    const result = validatePrimeGates([2, 4, 5]);
    expect(result.valid).toBe(false);
    expect(result.violations).toContain(4);
  });
});

describe('PolicyEngine', () => {
  let policy: PolicyEngine;
  let mapper: LexicalMapper;

  beforeEach(() => {
    mapper = new LexicalMapper();
    policy = new PolicyEngine(mapper, {
      state_norm: 1.0,
      drift_rate: 0.0,
      contractivity_score: 0.5,
      kill_switch_active: false,
      critique_results: Array(10).fill({ passed: true }),
      prime_gates: [{ gate_value: 2 }, { gate_value: 3 }, { gate_value: 5 }],
    });
  });

  it('validates constitution by default', () => {
    expect(policy.validateConstitution()).toBe(true);
  });

  it('reports kill switch as active when true', () => {
    const policyWithKillSwitch = new PolicyEngine(mapper, {
      state_norm: 1.0,
      drift_rate: 0.0,
      contractivity_score: 0.5,
      kill_switch_active: true,
      critique_results: Array(10).fill({ passed: true }),
      prime_gates: [{ gate_value: 2 }],
    });
    expect(policyWithKillSwitch.validateConstitution()).toBe(false);
  });

  it('generates witness for valid action', () => {
    const witness = policy.generateWitness('test-action-1', {
      tokens: [],
      prime_indices: [2, 3],
      r_sc: 1.5,
      c: 0.1,
      is_coherent: true,
    }, { allowed: true, reason: 'Admitted', violations: [] });

    expect(witness.witness_id).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/);
    expect(witness.action_id).toBe('test-action-1');
    expect(witness.veto_status).toBe('admitted');
    expect(witness.contractivity_score).toBeGreaterThan(0);
    expect(witness.compliance_evidence).toContain('R_sc');
  });

  it('generates witness for vetoed action', () => {
    const witness = policy.generateWitness('test-action-2', null, {
      allowed: false,
      reason: 'Test veto',
      violations: ['L0_02_Gate2_ContractionBound violated'],
    });

    expect(witness.veto_status).toBe('vetoed');
    expect(witness.violations).toContain('L0_02_Gate2_ContractionBound violated');
  });
});