import { LexicalToken, LexicalMapper } from './lexer.js';

export interface MOCWord {
  tokens: LexicalToken[];
  prime_indices: number[];
  r_sc: number;
  c: number;
  is_coherent: boolean;
}

export interface GrammarError {
  code: string;
  message: string;
  position?: number;
}

export interface ParseResult {
  word: MOCWord | null;
  errors: GrammarError[];
}

export function calculateResonanceTension(tokenIndices: number[]): number {
  if (tokenIndices.length === 0) return 0;
  if (tokenIndices.length === 1) return 1.0;
  // For coherent sentences: sum of normalized indices / max index
  const sum = tokenIndices.reduce((acc, idx) => acc + idx, 0);
  const max = Math.max(...tokenIndices);
  return sum / max;
}

export function calculateContractionBound(tokens: LexicalToken[]): number {
  if (tokens.length === 0) return 0;
  const verbCount = tokens.filter(t => t.semantic_role === 'verb').length;
  const nounCount = tokens.filter(t => t.semantic_role === 'noun').length;
  const modifierCount = tokens.filter(t => t.semantic_role === 'modifier').length;
  const adverbCount = tokens.filter(t => t.semantic_role === 'adverb').length;
  const entropy = (verbCount * nounCount) / (tokens.length * tokens.length);
  const modifierPenalty = modifierCount * 0.05;
  const adverbBonus = adverbCount * 0.02;
  return Math.min(1.5, Math.max(0, entropy + modifierPenalty + adverbBonus));
}

export class GrammarParser {
  private mapper: LexicalMapper;

  constructor(mapper: LexicalMapper) {
    this.mapper = mapper;
  }

  parse(input: string): ParseResult {
    const tokens = this.mapper.tokenize(input);
    const errors: GrammarError[] = [];

    if (tokens.length === 0 && input.trim().length > 0) {
      const words = input.toLowerCase().match(/[a-z]+/g) || [];
      for (const word of words) {
        errors.push({
          code: 'VOCAB_001',
          message: `Unknown word: "${word}". Not in controlled vocabulary.`,
        });
      }
      return { word: null, errors };
    }

    if (tokens.length === 0) {
      return { word: null, errors: [{ code: 'PARSE_001', message: 'No valid tokens found in input.' }] };
    }

    const primeIndices = tokens.map(t => t.prime_index);
    const r_sc = calculateResonanceTension(primeIndices);
    const c = calculateContractionBound(tokens);

    return {
      word: {
        tokens,
        prime_indices: primeIndices,
        r_sc,
        c,
        is_coherent: r_sc >= 1.0 && c < 1.0,
      },
      errors,
    };
  }

  validateCoherence(result: ParseResult): boolean {
    if (!result.word) return false;
    if (result.errors.length > 0) return false;
    return result.word.is_coherent;
  }
}