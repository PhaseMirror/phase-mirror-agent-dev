import { LexicalMapper, LexicalToken, LexicalMapping, isPrime } from './alp-nlp/lexer.js';
import { GrammarParser, MOCWord, ParseResult, GrammarError } from './alp-nlp/parser.js';
import { PolicyEngine, Action, AdmissibilityReport, validatePrimeGates } from './alp-nlp/policy.js';
import { createWitness, generateWitnessId, UnifiedWitness, WitnessOptions, ExecutionReceipt } from './alp-nlp/witness.js';
import { ArchivumStore } from './alp-nlp/archivum.js';

export { LexicalMapper, LexicalToken, LexicalMapping, isPrime, GrammarParser, MOCWord, ParseResult, GrammarError, PolicyEngine, Action, AdmissibilityReport, validatePrimeGates, createWitness, generateWitnessId, UnifiedWitness, WitnessOptions, ExecutionReceipt, ArchivumStore };

export class PhaseMirrorAgent {
  private mapper: LexicalMapper;
  private parser: GrammarParser;
  private policy: PolicyEngine;
  private archivum: ArchivumStore;
  private isLoaded: boolean = false;
  private actionCounter: number = 0;

  constructor() {
    this.mapper = new LexicalMapper();
    this.parser = new GrammarParser(this.mapper);
    this.policy = new PolicyEngine(this.mapper, {
      state_norm: 1.0,
      drift_rate: 0.0,
      contractivity_score: 0.5,
      kill_switch_active: false,
      critique_results: Array(10).fill({ passed: true }),
      prime_gates: [{ gate_value: 2 }, { gate_value: 3 }, { gate_value: 5 }],
    });
    this.archivum = new ArchivumStore();
  }

  async load(): Promise<void> {
    this.isLoaded = true;
  }

  async analyze(query: string, systemPrompt?: string): Promise<string> {
    if (!this.isLoaded) {
      await this.load();
    }

    this.actionCounter++;
    const actionId = `alp-nlp-${this.actionCounter}`;

    const parseResult = this.parser.parse(query);
    const admissibility = parseResult.word 
      ? this.policy.validateMOCWord(parseResult.word)
      : { allowed: false, reason: parseResult.errors[0]?.message || 'Parse failed', violations: parseResult.errors.map(e => e.message) };

    const witness = this.policy.generateWitness(actionId, parseResult.word || null, admissibility);
    await this.archivum.writeWitness(witness);

    if (!admissibility.allowed) {
      return `[VETO] ALP Policy rejected: ${admissibility.reason}\nWitness: ${witness.witness_id}`;
    }

    if (!parseResult.word || !parseResult.word.is_coherent) {
      return `[VETO] L0_02_Gate2_ContractionBound violated: Logical contradiction detected. c=${parseResult.word?.c.toFixed(4)}\nWitness: ${witness.witness_id}`;
    }

    let response = '[ALP-NLP] Deterministic response:\n\n';
    response += `Query parsed into ${parseResult.word.tokens.length} tokens.\n`;
    response += `Prime indices: ${parseResult.word.prime_indices.join(', ')}\n`;
    response += `Resonance Tension (R_sc): ${parseResult.word.r_sc.toFixed(4)}\n`;
    response += `Contraction Bound (c): ${parseResult.word.c.toFixed(4)}\n`;
    response += `Contractivity Score: ${witness.contractivity_score.toFixed(4)}\n`;
    response += `Status: ${parseResult.word.is_coherent ? 'COHERENT' : 'INCOHERENT'}\n`;

    if (systemPrompt) {
      response += `\n[System prompt recognized: "${systemPrompt.substring(0, 50)}..."]\n`;
    }

    response += `\nAction processed: ${actionId}`;
    response += `\nWitness: ${witness.witness_id}`;

    return response;
  }

  getVocabularySize(): number {
    return this.mapper.getVocabularySize();
  }

  validateQuery(query: string): AdmissibilityReport {
    const parseResult = this.parser.parse(query);
    if (!parseResult.word) {
      return { allowed: false, reason: parseResult.errors[0]?.message || 'Parse failed', violations: parseResult.errors.map(e => e.message) };
    }
    return this.policy.validateMOCWord(parseResult.word);
  }

  generateWitness(actionId: string, word: MOCWord | null, admissibility: AdmissibilityReport): UnifiedWitness {
    return this.policy.generateWitness(actionId, word, admissibility);
  }
}