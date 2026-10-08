import { createHash } from 'node:crypto';

export const GENESIS_HASH = '0000000000000000000000000000000000000000000000000000000000000000';

export interface ChainHead {
  prevHash: string;
  sequence: number;
}

export interface IntegrityResult {
  valid: boolean;
  entries: number;
  headHash: string;
}

/** Deterministic serialization shared with the Rust chain (src/audit/store.rs). */
export function canonicalJson(value: unknown): string {
  if (value === null) return 'null';
  if (typeof value === 'boolean') return value ? 'true' : 'false';
  if (typeof value === 'number') return canonicalNumber(value);
  if (typeof value === 'string') return JSON.stringify(value);
  if (Array.isArray(value)) {
    return `[${value.map(canonicalJson).join(',')}]`;
  }
  if (typeof value === 'object') {
    const record = value as Record<string, unknown>;
    const keys = Object.keys(record).sort();
    const parts = keys
      .filter(k => record[k] !== undefined)
      .map(k => `${JSON.stringify(k)}:${canonicalJson(record[k])}`);
    return `{${parts.join(',')}}`;
  }
  throw new Error(`cannot canonicalize ${typeof value}`);
}

/** Number formatting matching Rust: integer-valued numbers as integers, else shortest form. */
function canonicalNumber(n: number): string {
  if (Number.isFinite(n) && Number.isInteger(n)) {
    return String(n);
  }
  return String(n);
}

/** entry_hash = SHA-256(prev_hash || canonical_json(payload)) — must match the Rust store. */
export function entryHash(prevHash: string, payload: object): string {
  const canonical = canonicalJson(payload);
  return createHash('sha256').update(prevHash).update(canonical).digest('hex');
}

/**
 * witness_hash = SHA-256(canonical_action || idempotency_key || timestamp)
 * — must match src/cnl_bridge.rs::witness_hash.
 */
export function witnessHash(canonicalAction: string, idempotencyKey: string, timestamp: string): string {
  return createHash('sha256')
    .update(canonicalAction)
    .update(idempotencyKey)
    .update(timestamp)
    .digest('hex');
}

/** Strip chain fields so the payload matches what the Rust store hashes. */
export function payloadOf<T extends object>(entry: T): object {
  const { prev_hash: _prev, entry_hash: _hash, ...payload } = entry as Record<string, unknown>;
  return payload;
}

/** Recompute the chain over a list of records carrying prev_hash/entry_hash/sequence. */
export function verifyChain<T extends object>(entries: T[]): IntegrityResult {
  let prev = GENESIS_HASH;
  let head = GENESIS_HASH;
  let count = 0;
  for (const entry of entries) {
    const rec = entry as Record<string, unknown>;
    if (rec.prev_hash !== prev) {
      return { valid: false, entries: count, headHash: head };
    }
    const expected = entryHash(prev, payloadOf(entry));
    if (rec.entry_hash !== expected) {
      return { valid: false, entries: count, headHash: head };
    }
    prev = rec.entry_hash as string;
    head = prev;
    count += 1;
  }
  return { valid: true, entries: count, headHash: head };
}

export function headOf<T extends object>(entries: T[]): ChainHead {
  const last = entries[entries.length - 1] as Record<string, unknown> | undefined;
  if (!last) {
    return { prevHash: GENESIS_HASH, sequence: 0 };
  }
  return {
    prevHash: (last.entry_hash as string) ?? GENESIS_HASH,
    sequence: typeof last.sequence === 'number' ? last.sequence + 1 : 0,
  };
}
