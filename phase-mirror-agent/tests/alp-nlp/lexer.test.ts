import { describe, it, expect, beforeEach } from 'vitest';
import { LexicalMapper, isPrime } from '../../src/alp-nlp/lexer.js';

describe('isPrime', () => {
  it('returns true for known primes', () => {
    expect(isPrime(2)).toBe(true);
    expect(isPrime(3)).toBe(true);
    expect(isPrime(5)).toBe(true);
    expect(isPrime(7)).toBe(true);
    expect(isPrime(11)).toBe(true);
    expect(isPrime(13)).toBe(true);
    expect(isPrime(17)).toBe(true);
    expect(isPrime(19)).toBe(true);
    expect(isPrime(23)).toBe(true);
    expect(isPrime(29)).toBe(true);
  });

  it('returns false for non-primes', () => {
    expect(isPrime(0)).toBe(false);
    expect(isPrime(1)).toBe(false);
    expect(isPrime(4)).toBe(false);
    expect(isPrime(6)).toBe(false);
    expect(isPrime(8)).toBe(false);
    expect(isPrime(9)).toBe(false);
    expect(isPrime(10)).toBe(false);
    expect(isPrime(100)).toBe(false);
  });
});

describe('LexicalMapper', () => {
  let mapper: LexicalMapper;

  beforeEach(() => {
    mapper = new LexicalMapper();
  });

  it('tokenizes known words', () => {
    const tokens = mapper.tokenize('deploy service');
    expect(tokens).toHaveLength(2);
    expect(tokens[0].word).toBe('deploy');
    expect(tokens[0].prime_index).toBe(2);
    expect(tokens[1].word).toBe('service');
    expect(tokens[1].prime_index).toBe(7);
  });

  it('ignores unknown words', () => {
    const tokens = mapper.tokenize('unknown jargon');
    expect(tokens).toHaveLength(0);
  });

  it('reports vocabulary size', () => {
    expect(mapper.getVocabularySize()).toBeGreaterThanOrEqual(50);
  });

  it('all prime indices are valid primes', () => {
    const controlledTokens = mapper.tokenize(
      'deploy verify analyze service adr govern workflow witness action policy'
    );
    for (const token of controlledTokens) {
      expect(isPrime(token.prime_index)).toBe(true);
    }
  });
});