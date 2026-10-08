import { describe, it, expect, beforeEach } from 'vitest';
import { GrammarParser, calculateResonanceTension, calculateContractionBound } from '../../src/alp-nlp/parser.js';
import { LexicalMapper } from '../../src/alp-nlp/lexer.js';

describe('calculateResonanceTension', () => {
  it('returns zero for empty input', () => {
    expect(calculateResonanceTension([])).toBe(0);
  });

  it('calculates tension for single prime', () => {
    const tension = calculateResonanceTension([2]);
    expect(tension).toBeGreaterThan(0);
  });

  it('calculates tension for multiple primes', () => {
    const tension = calculateResonanceTension([2, 3, 5]);
    expect(tension).toBeGreaterThan(0);
    expect(tension).toBeLessThan(10);
  });
});

describe('calculateContractionBound', () => {
  it('returns zero for empty tokens', () => {
    expect(calculateContractionBound([])).toBe(0);
  });

  it('calculates bound for verb+noun pattern', () => {
    const mapper = new LexicalMapper();
    const tokens = mapper.tokenize('deploy service');
    const c = calculateContractionBound(tokens);
    expect(c).toBeGreaterThan(0);
    expect(c).toBeLessThan(1.0);
  });
});

describe('GrammarParser', () => {
  let parser: GrammarParser;
  let mapper: LexicalMapper;

  beforeEach(() => {
    mapper = new LexicalMapper();
    parser = new GrammarParser(mapper);
  });

  it('parses valid controlled natural language', () => {
    const result = parser.parse('deploy service');
    expect(result.word).not.toBeNull();
    expect(result.errors).toHaveLength(0);
  });

  it('flags unknown words', () => {
    const result = parser.parse('unknown word');
    expect(result.errors.length).toBeGreaterThan(0);
    expect(result.errors[0].code).toBe('VOCAB_001');
  });

  it('rejects empty input', () => {
    const result = parser.parse('');
    expect(result.word).toBeNull();
    expect(result.errors[0].code).toBe('PARSE_001');
  });

  it('produces coherent word for valid input', () => {
    const result = parser.parse('verify adr');
    expect(result.word).not.toBeNull();
    expect(result.word!.is_coherent).toBe(true);
    expect(result.word!.c).toBeLessThan(1.0);
  });
});