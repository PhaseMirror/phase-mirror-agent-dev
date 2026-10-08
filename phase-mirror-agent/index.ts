// DEPRECATED: LLM-based implementation moved to archive/llm-index.ts
// Active implementation: src/index.ts (ALP-based Semantic Compiler)

export { PhaseMirrorAgent } from './src/index.js';
export { LexicalMapper, LexicalToken } from './src/alp-nlp/lexer.js';
export { GrammarParser, MOCWord } from './src/alp-nlp/parser.js';
export { PolicyEngine, Action, AdmissibilityReport, UnifiedWitness } from './src/alp-nlp/policy.js';
