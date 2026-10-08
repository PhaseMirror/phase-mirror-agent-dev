import { MOCWord, GrammarError, ParseResult } from './parser.js';
import { LexicalMapper, isPrime } from './lexer.js';
import { createWitness, UnifiedWitness, WitnessOptions } from './witness.js';

export interface Action {
  id: string;
  payload: string;
  mutating: boolean;
  server_binding: string | null;
}

export interface AdmissibilityReport {
  allowed: boolean;
  reason: string;
  violations: string[];
}

export interface ConstitutionModel {
  state_norm: number;
  drift_rate: number;
  contractivity_score: number;
  kill_switch_active: boolean;
  critique_results: { passed: boolean }[];
  prime_gates: { gate_value: number }[];
}



export function validatePrimeGates(indices: number[]): { valid: boolean; violations: number[] } {
  const violations: number[] = [];
  for (const idx of indices) {
    if (!isPrime(idx)) {
      violations.push(idx);
    }
  }
  return { valid: violations.length === 0, violations };
}

export class PolicyEngine {
  private mapper: LexicalMapper;
  private constitution: ConstitutionModel;

  constructor(mapper: LexicalMapper, constitution: ConstitutionModel) {
    this.mapper = mapper;
    this.constitution = constitution;
  }

  validateConstitution(): boolean {
    const lambdaMThreshold = 0.1;
    const circuitBreakerThreshold = 3;

    const stateNormBounded = this.constitution.state_norm > 0 && Number.isFinite(this.constitution.state_norm);
    const driftRateBounded = this.constitution.drift_rate < lambdaMThreshold;
    const critiquePassed = this.constitution.critique_results.length === 10 &&
      this.constitution.critique_results.every(r => r.passed);
    const primeGatesValid = this.constitution.prime_gates.every(g => isPrime(g.gate_value));
    const lambdaMCompliant = this.constitution.contractivity_score > 0 && this.constitution.contractivity_score <= 1.0;
    const killSwitchInactive = !this.constitution.kill_switch_active;
    const circuitBreakerNotTripped = this.constitution.critique_results.filter(r => !r.passed).length < circuitBreakerThreshold;

    return stateNormBounded && driftRateBounded && critiquePassed && primeGatesValid &&
      lambdaMCompliant && killSwitchInactive && circuitBreakerNotTripped;
  }

  validateAction(action: Action): AdmissibilityReport {
    const violations: string[] = [];

    if (!this.validateConstitution()) {
      violations.push('Constitutional policy validation failed');
    }

    if (action.server_binding !== null) {
      violations.push('External trust level requires no server_binding');
    }

    if (action.mutating && action.server_binding === null) {
      violations.push('Mutating actions blocked at ALP gate');
    }

    return {
      allowed: violations.length === 0,
      reason: violations.length === 0 ? 'Admitted' : 'Vetoed by constitutional policy',
      violations,
    };
  }

  validateMOCWord(word: MOCWord): AdmissibilityReport {
    const violations: string[] = [];

    const { valid, violations: primeViolations } = validatePrimeGates(word.prime_indices);
    if (!valid) {
      violations.push(`L0_04: Non-prime gate values detected: ${primeViolations.join(', ')}`);
    }

    if (word.c >= 1.0) {
      violations.push(`L0_02_Gate2_ContractionBound violated: c=${word.c.toFixed(4)} >= 1.0`);
    }

    if (word.r_sc < 1.0) {
      violations.push(`L0_03_Gate3_ResonanceTension violated: R_sc=${word.r_sc.toFixed(4)} < 1.0`);
    }

    if (!this.validateConstitution()) {
      violations.push('Constitutional validation failed');
    }

    return {
      allowed: violations.length === 0,
      reason: violations.length === 0 ? 'Admitted' : violations[0],
      violations,
    };
  }

  generateWitness(actionId: string, word: MOCWord | null, admissibility: AdmissibilityReport): UnifiedWitness {
    const contractivityScore = word && word.c < 1.0 ? 1.0 - word.c : 0.0;
    const options: WitnessOptions = {
      actionId,
      primeIndices: word?.prime_indices || [],
      r_sc: word?.r_sc || 0,
      c: word?.c || 0,
      contractivityScore,
      violations: admissibility.violations,
    };
    return createWitness(options);
  }
}