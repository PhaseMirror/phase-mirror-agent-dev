import { describe, it, expect } from 'vitest';
import { mkdir } from 'fs/promises';
import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';
import { ArchivumStore } from '../../src/alp-nlp/archivum.js';
import { UnifiedWitness } from '../../src/alp-nlp/witness.js';
import { GENESIS_HASH, verifyChain, entryHash, canonicalJson, witnessHash } from '../../src/alp-nlp/chain.js';

const SAMPLE_WITNESSES: UnifiedWitness[] = [
  {
    witness_id: '11111111-1111-1111-1111-111111111111',
    action_id: 'alp-nlp-1',
    timestamp: '2026-08-08T00:00:00.000Z',
    compliance_evidence: 'R_sc=1.2345, c=0.1000',
    execution_receipt: {
      status: 'completed',
      prime_indices: [2, 3, 5],
      r_sc: 1.2345,
      c: 0.1,
      contractivity_score: 0.9,
    },
    contractivity_score: 0.9,
    veto_status: 'admitted',
    violations: [],
  },
  {
    witness_id: '22222222-2222-2222-2222-222222222222',
    action_id: 'alp-nlp-2',
    timestamp: '2026-08-08T00:00:01.000Z',
    compliance_evidence: 'R_sc=1.5000, c=0.8400',
    execution_receipt: {
      status: 'completed',
      prime_indices: [7, 11],
      r_sc: 1.5,
      c: 0.84,
      contractivity_score: 0.16,
    },
    contractivity_score: 0.16,
    veto_status: 'admitted',
    violations: [],
  },
  {
    witness_id: '33333333-3333-3333-3333-333333333333',
    action_id: 'alp-nlp-3',
    timestamp: '2026-08-08T00:00:02.000Z',
    compliance_evidence: 'violations=1',
    execution_receipt: {
      status: 'rejected',
      prime_indices: [2, 4],
      r_sc: 0.5,
      c: 1.2,
      contractivity_score: 0,
    },
    contractivity_score: 0,
    veto_status: 'vetoed',
    violations: ['L0_04: Non-prime gate values detected: 4'],
  },
];

describe('chain (shared algorithm)', () => {
  it('first entry links from genesis', () => {
    const chained = { ...SAMPLE_WITNESSES[0], sequence: 0, prev_hash: GENESIS_HASH };
    const hash = entryHash(GENESIS_HASH, { ...chained, entry_hash: '' });
    expect(hash).toMatch(/^[0-9a-f]{64}$/);
  });

  it('canonicalJson matches the Rust byte format', () => {
    expect(canonicalJson({ b: 2, a: [true, null, { y: 1.5, x: 's' }] }))
      .toBe('{"a":[true,null,{"x":"s","y":1.5}],"b":2}');
    expect(canonicalJson({ n: 209.3, i: 2.0 })).toBe('{"i":2,"n":209.3}');
  });

  it('witnessHash matches the Rust cnl_bridge vector', () => {
    const canonical = '{"action":"deploy","replicas":3,"service":"web-service","target":"cluster"}';
    const hash = witnessHash(canonical, 'client-key-1', '2026-08-08T12:00:00Z');
    expect(hash).toBe('6c5058f4a750d92476e741738b41ea5a92c27f9be57c0e535ec2e872854bc3b1');
    expect(witnessHash(canonical, 'client-key-1', '2026-08-08T12:00:00Z')).toBe(hash);
    expect(witnessHash(canonical, 'client-key-2', '2026-08-08T12:00:00Z')).not.toBe(hash);
  });
});

describe('ArchivumStore chain', () => {
  it('writes a chained JSONL fixture and verifies it', async () => {
    const tempDir = join(tmpdir(), `chain-fixture-${Date.now()}`);
    const archivum = new ArchivumStore(tempDir);
    for (const w of SAMPLE_WITNESSES) {
      await archivum.writeWitness(w);
    }

    const witnesses = await archivum.readWitnesses();
    expect(witnesses).toHaveLength(3);
    expect(witnesses[0].sequence).toBe(0);
    expect(witnesses[0].prev_hash).toBe(GENESIS_HASH);
    expect(witnesses[1].prev_hash).toBe(witnesses[0].entry_hash);
    expect(witnesses[2].prev_hash).toBe(witnesses[1].entry_hash);

    const report = await archivum.verifyChain();
    expect(report.valid).toBe(true);
    expect(report.entries).toBe(3);
    expect(report.headHash).toMatch(/^[0-9a-f]{64}$/);
  });

  it('detects tampering in the archive file', async () => {
    const tempDir = join(tmpdir(), `chain-tamper-${Date.now()}`);
    const archivum = new ArchivumStore(tempDir);
    for (const w of SAMPLE_WITNESSES) {
      await archivum.writeWitness(w);
    }
    const path = join(tempDir, 'state', 'archivum', 'witnesses.jsonl');
    const content = await readFile(path, 'utf-8');
    const tampered = content.replace('1.2345', '9.9999');
    await writeFile(path, tampered);
    const report = await archivum.verifyChain();
    expect(report.valid).toBe(false);
  });

  it('regenerates the committed cross-language fixture', async () => {
    const fixtureDir = join(process.cwd(), 'tests', 'fixtures');
    await mkdir(fixtureDir, { recursive: true });
    const tempDir = join(tmpdir(), `chain-fixture-gen-${Date.now()}`);
    const archivum = new ArchivumStore(tempDir);
    for (const w of SAMPLE_WITNESSES) {
      await archivum.writeWitness(w);
    }
    const content = await readFile(join(tempDir, 'state', 'archivum', 'witnesses.jsonl'), 'utf-8');
    await writeFile(join(fixtureDir, 'ts-witnesses.jsonl'), content);
    expect(content.split('\n').filter(l => l.trim())).toHaveLength(3);
  });
});
