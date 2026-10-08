export interface LexicalToken {
  word: string;
  prime_index: number;
  semantic_role: 'noun' | 'verb' | 'adverb' | 'modifier';
  domain: string;
}

export interface LexicalMapping {
  vocabulary: Map<string, LexicalToken>;
}

const CONTROLLED_VOCABULARY: LexicalToken[] = [
  { word: 'deploy', prime_index: 2, semantic_role: 'verb', domain: 'devops' },
  { word: 'verify', prime_index: 3, semantic_role: 'verb', domain: 'governance' },
  { word: 'analyze', prime_index: 5, semantic_role: 'verb', domain: 'research' },
  { word: 'service', prime_index: 7, semantic_role: 'noun', domain: 'devops' },
  { word: 'adr', prime_index: 11, semantic_role: 'noun', domain: 'governance' },
  { word: 'govern', prime_index: 13, semantic_role: 'verb', domain: 'orchestration' },
  { word: 'workflow', prime_index: 17, semantic_role: 'noun', domain: 'orchestration' },
  { word: 'witness', prime_index: 19, semantic_role: 'noun', domain: 'archivum' },
  { word: 'action', prime_index: 23, semantic_role: 'noun', domain: 'policy' },
  { word: 'policy', prime_index: 29, semantic_role: 'noun', domain: 'governance' },
  { word: 'execute', prime_index: 31, semantic_role: 'verb', domain: 'sigma' },
  { word: 'transition', prime_index: 37, semantic_role: 'verb', domain: 'sigma' },
  { word: 'contract', prime_index: 41, semantic_role: 'verb', domain: 'alp' },
  { word: 'drift', prime_index: 43, semantic_role: 'noun', domain: 'governance' },
  { word: 'tension', prime_index: 47, semantic_role: 'noun', domain: 'alp' },
  { word: 'state', prime_index: 53, semantic_role: 'noun', domain: 'sigma' },
  { word: 'norm', prime_index: 59, semantic_role: 'noun', domain: 'alp' },
  { word: 'bounded', prime_index: 61, semantic_role: 'adverb', domain: 'alp' },
  { word: 'kernel', prime_index: 67, semantic_role: 'noun', domain: 'sigma' },
  { word: 'operator', prime_index: 71, semantic_role: 'noun', domain: 'alp' },
  { word: 'prime', prime_index: 73, semantic_role: 'noun', domain: 'alp' },
  { word: 'index', prime_index: 79, semantic_role: 'noun', domain: 'alp' },
  { word: 'session', prime_index: 83, semantic_role: 'noun', domain: 'sigma' },
  { word: 'stratum', prime_index: 89, semantic_role: 'noun', domain: 'alp' },
  { word: 'ensemble', prime_index: 97, semantic_role: 'noun', domain: 'alp' },
  { word: 'constitute', prime_index: 101, semantic_role: 'verb', domain: 'governance' },
  { word: 'axiom', prime_index: 103, semantic_role: 'noun', domain: 'alp' },
  { word: 'formal', prime_index: 107, semantic_role: 'adverb', domain: 'alp' },
  { word: 'prove', prime_index: 109, semantic_role: 'verb', domain: 'alp' },
  { word: 'validate', prime_index: 113, semantic_role: 'verb', domain: 'governance' },
  { word: 'reject', prime_index: 127, semantic_role: 'verb', domain: 'governance' },
  { word: 'veto', prime_index: 131, semantic_role: 'verb', domain: 'alp' },
  { word: 'pass', prime_index: 137, semantic_role: 'verb', domain: 'alp' },
  { word: 'fail', prime_index: 139, semantic_role: 'verb', domain: 'alp' },
  { word: 'sat', prime_index: 149, semantic_role: 'noun', domain: 'governance' },
  { word: 'token', prime_index: 151, semantic_role: 'noun', domain: 'mcp' },
  { word: 'request', prime_index: 157, semantic_role: 'noun', domain: 'mcp' },
  { word: 'tool', prime_index: 163, semantic_role: 'noun', domain: 'mcp' },
  { word: 'call', prime_index: 167, semantic_role: 'verb', domain: 'mcp' },
  { word: 'route', prime_index: 173, semantic_role: 'verb', domain: 'mcp' },
  { word: 'server', prime_index: 179, semantic_role: 'noun', domain: 'mcp' },
  { word: 'transport', prime_index: 181, semantic_role: 'noun', domain: 'mcp' },
  { word: 'stdio', prime_index: 191, semantic_role: 'noun', domain: 'mcp' },
  { word: 'http', prime_index: 193, semantic_role: 'noun', domain: 'mcp' },
  { word: 'secure', prime_index: 197, semantic_role: 'adverb', domain: 'governance' },
  { word: 'audit', prime_index: 199, semantic_role: 'verb', domain: 'archivum' },
  { word: 'ledger', prime_index: 211, semantic_role: 'noun', domain: 'archivum' },
  { word: 'entry', prime_index: 223, semantic_role: 'noun', domain: 'archivum' },
  { word: 'immutable', prime_index: 227, semantic_role: 'adverb', domain: 'archivum' },
  { word: 'provenance', prime_index: 229, semantic_role: 'noun', domain: 'archivum' },
  { word: 'query', prime_index: 233, semantic_role: 'noun', domain: 'nlp' },
  { word: 'response', prime_index: 239, semantic_role: 'noun', domain: 'nlp' },
  { word: 'compile', prime_index: 241, semantic_role: 'verb', domain: 'nlp' },
  { word: 'parse', prime_index: 251, semantic_role: 'verb', domain: 'nlp' },
  { word: 'transform', prime_index: 257, semantic_role: 'verb', domain: 'nlp' },
  { word: 'deterministic', prime_index: 263, semantic_role: 'adverb', domain: 'alp' },
  { word: 'bounded', prime_index: 269, semantic_role: 'adverb', domain: 'alp' },
  { word: 'coherent', prime_index: 271, semantic_role: 'adverb', domain: 'alp' },
  { word: 'contradiction', prime_index: 277, semantic_role: 'noun', domain: 'alp' },
  { word: 'violation', prime_index: 281, semantic_role: 'noun', domain: 'governance' },
  { word: 'invariant', prime_index: 283, semantic_role: 'noun', domain: 'alp' },
  { word: 'check', prime_index: 293, semantic_role: 'verb', domain: 'governance' },
  { word: 'measure', prime_index: 307, semantic_role: 'verb', domain: 'alp' },
  { word: 'calculate', prime_index: 311, semantic_role: 'verb', domain: 'alp' },
  { word: 'emit', prime_index: 313, semantic_role: 'verb', domain: 'alp' },
  { word: 'report', prime_index: 317, semantic_role: 'verb', domain: 'archivum' },
  { word: 'diagnose', prime_index: 331, semantic_role: 'verb', domain: 'alp' },
  { word: 'the', prime_index: 337, semantic_role: 'modifier', domain: 'nlp' },
  { word: 'system', prime_index: 347, semantic_role: 'noun', domain: 'system' },
  { word: 'command', prime_index: 349, semantic_role: 'verb', domain: 'sigma' },
  { word: 'mirror', prime_index: 353, semantic_role: 'noun', domain: 'alp' },
  { word: 'phase', prime_index: 359, semantic_role: 'noun', domain: 'alp' },
  { word: 'run', prime_index: 367, semantic_role: 'verb', domain: 'sigma' },
  { word: 'load', prime_index: 373, semantic_role: 'verb', domain: 'sigma' },
  { word: 'save', prime_index: 379, semantic_role: 'verb', domain: 'sigma' },
  { word: 'open', prime_index: 383, semantic_role: 'verb', domain: 'governance' },
  { word: 'close', prime_index: 389, semantic_role: 'verb', domain: 'governance' },
  { word: 'start', prime_index: 397, semantic_role: 'verb', domain: 'sigma' },
  { word: 'stop', prime_index: 401, semantic_role: 'verb', domain: 'sigma' },
  { word: 'create', prime_index: 409, semantic_role: 'verb', domain: 'alp' },
  { word: 'delete', prime_index: 419, semantic_role: 'verb', domain: 'alp' },
  { word: 'update', prime_index: 421, semantic_role: 'verb', domain: 'alp' },
  { word: 'read', prime_index: 431, semantic_role: 'verb', domain: 'sigma' },
  { word: 'write', prime_index: 433, semantic_role: 'verb', domain: 'sigma' },
  { word: 'file', prime_index: 439, semantic_role: 'noun', domain: 'system' },
  { word: 'config', prime_index: 443, semantic_role: 'noun', domain: 'system' },
  { word: 'test', prime_index: 449, semantic_role: 'verb', domain: 'sigma' },
  { word: 'assert', prime_index: 457, semantic_role: 'verb', domain: 'alp' },
  { word: 'prove', prime_index: 461, semantic_role: 'verb', domain: 'alp' },
  { word: 'check', prime_index: 463, semantic_role: 'verb', domain: 'governance' },
  { word: 'record', prime_index: 467, semantic_role: 'verb', domain: 'archivum' },
  { word: 'record', prime_index: 467, semantic_role: 'noun', domain: 'archivum' },
  { word: 'compute', prime_index: 479, semantic_role: 'verb', domain: 'sigma' },
  { word: 'process', prime_index: 487, semantic_role: 'verb', domain: 'sigma' },
  { word: 'node', prime_index: 491, semantic_role: 'noun', domain: 'mcp' },
  { word: 'python', prime_index: 499, semantic_role: 'noun', domain: 'mcp' },
  { word: 'rust', prime_index: 503, semantic_role: 'noun', domain: 'rust' },
  { word: 'kernel', prime_index: 509, semantic_role: 'noun', domain: 'sigma' },
  { word: 'session', prime_index: 521, semantic_role: 'noun', domain: 'sigma' },
  { word: 'graph', prime_index: 523, semantic_role: 'noun', domain: 'sigma' },
  { word: 'matrix', prime_index: 541, semantic_role: 'noun', domain: 'sigma' },
  { word: 'coupling', prime_index: 547, semantic_role: 'noun', domain: 'sigma' },
  { word: 'spectral', prime_index: 557, semantic_role: 'noun', domain: 'alp' },
  { word: 'gain', prime_index: 563, semantic_role: 'noun', domain: 'alp' },
  { word: 'small', prime_index: 569, semantic_role: 'adverb', domain: 'alp' },
  { word: 'link', prime_index: 571, semantic_role: 'verb', domain: 'alp' },
  { word: 'transpile', prime_index: 577, semantic_role: 'verb', domain: 'compiler' },
  { word: 'verify', prime_index: 587, semantic_role: 'verb', domain: 'compiler' },
  { word: 'diagnostic', prime_index: 593, semantic_role: 'noun', domain: 'compiler' },
  { word: 'pass', prime_index: 599, semantic_role: 'noun', domain: 'compiler' },
  { word: 'fail', prime_index: 601, semantic_role: 'noun', domain: 'compiler' },
  { word: 'l0', prime_index: 607, semantic_role: 'noun', domain: 'governance' },
  { word: 'invariants', prime_index: 613, semantic_role: 'noun', domain: 'governance' },
  { word: 'l0-1', prime_index: 617, semantic_role: 'noun', domain: 'governance' },
  { word: 'l0-2', prime_index: 619, semantic_role: 'noun', domain: 'governance' },
  { word: 'l0-3', prime_index: 631, semantic_role: 'noun', domain: 'governance' },
  { word: 'l0-4', prime_index: 641, semantic_role: 'noun', domain: 'governance' },
  { word: 'l0-5', prime_index: 643, semantic_role: 'noun', domain: 'governance' },
  { word: 'l0-6', prime_index: 647, semantic_role: 'noun', domain: 'governance' },
  { word: 'l0-7', prime_index: 653, semantic_role: 'noun', domain: 'governance' },
  { word: 'l0-9', prime_index: 659, semantic_role: 'noun', domain: 'governance' },
];

export function isPrime(n: number): boolean {
  if (n < 2) return false;
  if (n === 2) return true;
  if (n % 2 === 0) return false;
  for (let i = 3; i * i <= n; i += 2) {
    if (n % i === 0) return false;
  }
  return true;
}

export class LexicalMapper {
  private vocabulary: Map<string, LexicalToken>;

  constructor() {
    this.vocabulary = new Map();
    for (const token of CONTROLLED_VOCABULARY) {
      this.vocabulary.set(token.word.toLowerCase(), token);
    }
  }

  tokenize(input: string): LexicalToken[] {
    const words = input.toLowerCase().match(/[a-z]+/g) || [];
    return words
      .map(word => this.vocabulary.get(word))
      .filter((token): token is LexicalToken => token !== undefined);
  }

  contains(word: string): boolean {
    return this.vocabulary.has(word.toLowerCase());
  }

  getVocabularySize(): number {
    return this.vocabulary.size;
  }
}