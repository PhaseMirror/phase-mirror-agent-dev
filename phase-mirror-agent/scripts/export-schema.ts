import { writeFile } from 'fs/promises';
import { join } from 'path';
import { JSONSchemaType } from 'ajv/dist/2020.js';

const unifiedWitnessSchema: JSONSchemaType<any> = {
  $id: 'UnifiedWitness',
  type: 'object',
  properties: {
    witness_id: { type: 'string' },
    action_id: { type: 'string' },
    timestamp: { type: 'string', format: 'date-time' },
    compliance_evidence: { type: 'string' },
    execution_receipt: {
      type: 'object',
      properties: {
        status: { type: 'string' },
        prime_indices: {
          type: 'array',
          items: { type: 'number' },
        },
        r_sc: { type: 'number' },
        c: { type: 'number' },
        contractivity_score: { type: 'number' },
      },
      required: ['status', 'prime_indices', 'r_sc', 'c', 'contractivity_score'],
    },
    contractivity_score: { type: 'number' },
    veto_status: {
      type: 'string',
      enum: ['admitted', 'vetoed'],
    },
    witness_hash: { type: 'string', nullable: true },
    p_lineage: { type: 'string', nullable: true },
    violations: {
      type: 'array',
      items: { type: 'string' },
    },
    sequence: { type: 'integer', nullable: true },
    prev_hash: { type: 'string', pattern: '^[0-9a-f]{64}$', nullable: true },
    entry_hash: { type: 'string', pattern: '^[0-9a-f]{64}$', nullable: true },
  },
  required: ['witness_id', 'action_id', 'timestamp', 'compliance_evidence', 'execution_receipt', 'contractivity_score', 'veto_status', 'violations'],
};

const lexicalTokenSchema: JSONSchemaType<any> = {
  $id: 'LexicalToken',
  type: 'object',
  properties: {
    word: { type: 'string' },
    prime_index: { type: 'number' },
    semantic_role: {
      type: 'string',
      enum: ['noun', 'verb', 'adverb', 'modifier'],
    },
    domain: { type: 'string' },
  },
  required: ['word', 'prime_index', 'semantic_role', 'domain'],
};

const mocWordSchema: JSONSchemaType<any> = {
  $id: 'MOCWord',
  type: 'object',
  properties: {
    tokens: {
      type: 'array',
      items: { $ref: 'LexicalToken' },
    },
    prime_indices: {
      type: 'array',
      items: { type: 'number' },
    },
    r_sc: { type: 'number' },
    c: { type: 'number' },
    is_coherent: { type: 'boolean' },
  },
  required: ['tokens', 'prime_indices', 'r_sc', 'c', 'is_coherent'],
};

const toolRequestSchema: JSONSchemaType<any> = {
  $id: 'ToolRequest',
  type: 'object',
  properties: {
    tool: { type: 'string' },
    args: {
      type: 'array',
      items: { type: 'string' },
    },
    idempotency_key: { type: 'string', nullable: true },
    dry_run: { type: 'boolean' },
    principal: { type: 'string', nullable: true },
  },
  required: ['tool', 'args', 'dry_run'],
};

const receiptSchema: JSONSchemaType<any> = {
  $id: 'Receipt',
  type: 'object',
  properties: {
    tool: { type: 'string' },
    status: { type: 'string' },
    idempotency_key: { type: 'string' },
    started_at: { type: 'string', format: 'date-time' },
    finished_at: { type: 'string', format: 'date-time' },
    detail: { type: 'string' },
    exit: { type: 'integer' },
  },
  required: ['tool', 'status', 'idempotency_key', 'started_at', 'finished_at', 'detail', 'exit'],
};

async function main() {
  const outputDir = join(process.cwd(), 'dist');
  try {
    await import('fs').then(fs => fs.promises.mkdir(outputDir, { recursive: true }));
  } catch {}

  await writeFile(join(outputDir, 'unified-witness.schema.json'), JSON.stringify(unifiedWitnessSchema, null, 2));
  await writeFile(join(outputDir, 'lexical-token.schema.json'), JSON.stringify(lexicalTokenSchema, null, 2));
  await writeFile(join(outputDir, 'moc-word.schema.json'), JSON.stringify(mocWordSchema, null, 2));
  await writeFile(join(outputDir, 'tool-request.schema.json'), JSON.stringify(toolRequestSchema, null, 2));
  await writeFile(join(outputDir, 'receipt.schema.json'), JSON.stringify(receiptSchema, null, 2));

  console.log('Schemas exported to dist/');
}

main().catch(console.error);