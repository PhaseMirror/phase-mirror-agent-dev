import { UnifiedWitness } from './witness.js';
import { canonicalJson, entryHash, headOf, payloadOf, verifyChain, IntegrityResult } from './chain.js';
import { mkdir, open } from 'fs/promises';
import { join } from 'path';

export class ArchivumStore {
  private archivumDir: string;

  constructor(basePath: string = process.cwd()) {
    this.archivumDir = join(basePath, 'state', 'archivum');
  }

  async writeWitness(witness: UnifiedWitness): Promise<void> {
    await mkdir(this.archivumDir, { recursive: true });
    const path = join(this.archivumDir, 'witnesses.jsonl');

    const witnesses = await this.readWitnesses();
    const { prevHash, sequence } = headOf(witnesses);

    const chained: UnifiedWitness = {
      ...witness,
      sequence,
      prev_hash: prevHash,
      entry_hash: '',
    };
    chained.entry_hash = entryHash(prevHash, payloadOf(chained));

    const jsonLine = canonicalJson(chained) + '\n';
    const handle = await open(path, 'a');
    try {
      await handle.writeFile(jsonLine);
      await handle.sync();
    } finally {
      await handle.close();
    }
  }

  async readWitnesses(): Promise<UnifiedWitness[]> {
    const path = join(this.archivumDir, 'witnesses.jsonl');
    try {
      const content = await import('fs').then(fs => fs.promises.readFile(path, 'utf-8'));
      return content
        .split('\n')
        .filter(line => line.trim().length > 0)
        .map(line => JSON.parse(line) as UnifiedWitness);
    } catch (err: any) {
      if (err.code === 'ENOENT') return [];
      throw err;
    }
  }

  async verifyChain(): Promise<IntegrityResult> {
    const witnesses = await this.readWitnesses();
    return verifyChain(witnesses);
  }

  async getLastWitness(actionId: string): Promise<UnifiedWitness | null> {
    const witnesses = await this.readWitnesses();
    return witnesses.find(w => w.action_id === actionId) || null;
  }
}
