# ADR-006: Authentication, Authorization & Rate Limiting for phase-mirror-agent

- Status: accepted
- Date: 2026-08-08
- Owners: Multiplicity Foundation
- Tags: #security, #authentication, #authorization, #rate-limiting, #tls, #secrets
- Phase: phase-3 (master plan ADR-004)
- Related: ADR-004 (master), ADR-008 (governed tool backplane), ADR-009 (secrets at deploy time)

## 1. Context

The HTTP and WebSocket surfaces of `phase-mirror-agent` are currently open:

- No authentication on any route; `POST /api/command` can be invoked by anyone who can
  reach the port. A malicious or accidental command can trigger tool execution.
- No authorization scopes; every caller is equivalent to every other.
- No rate limiting; an unauthenticated flood can burn CPU on CNL compilation and spam
  the audit WAL.
- CORS allows any method and only filters origins; `POST`/`DELETE` are unconstrained.
- TLS is optional and off by default; credentials are loaded via `dotenv` from an
  unvalidated `.env`.

This violates the "non-bypassable gate" requirement: an ungoverned caller bypasses the
governance layer entirely.

## 2. Decision

Adopt a **defense-in-depth** posture: bearer-token authentication for all write paths,
scoped authorization, per-client rate limiting, tightened CORS, and TLS-first defaults.

### 2.1 Authentication

- Static **API keys** (operator keys) provisioned from `PHASE_MIRROR_OPERATOR_KEYS`
  (env or a file mounted `0600`). Format: `pmr_op_<32-hex>` from a CSPRNG.
- Required on: `POST /api/command`, `POST /audit/entries`.
- Optional (config-gated) on WebSocket upgrade (`Authorization: Bearer <key>`).
- Keys are compared with constant-time equality; hashed at rest (SHA-256) in config.
- No session/cookie machinery — this is a machine-to-machine gateway.

### 2.2 Authorization scopes

Two scopes, mapped per key:
- `operator:read` — read-only endpoints (`/health`, `/ready`, `GET /audit/*`,
  `/audit/integrity`).
- `operator:write` — command execution and audit write.
- Default deny; unknown keys rejected; scopes recorded in audit entries (`actor`).

### 2.3 Rate limiting

- Token-bucket limiter (per key and per IP) on `/api/*` and WebSocket upgrades.
- Defaults: `PHASE_MIRROR_RATE_LIMIT_RPS = 10`, burst `20`, configurable.
- Exceeded limit → `429 Too Many Requests` with `Retry-After`.

### 2.4 CORS & transport

- CORS: explicit origin allow-list only; `AllowMethods` restricted to
  `GET, POST, OPTIONS`; no `*` origin in production mode (`PHASE_MIRROR_ENV=production`).
- TLS: in production mode, refuse to bind plain HTTP unless `--no-tls` is explicitly
  passed. Operator WS upgrades over TLS use the same listener.
- mTLS is supported (config-gated) for operator clients when a CA is mounted.

### 2.5 Secrets handling

- No secrets in logs or error messages (redact `Authorization` headers in trace output).
- `.env` loading validates known keys and rejects unknown ones; a missing
  `PHASE_MIRROR_OPERATOR_KEYS` in production mode aborts startup.
- Default secrets file path: `./config/keys/operator.keys` (gitignored, `0600`).

## 3. Implementation Plan

**Phase:** Phase 3 of ADR-004.

**Target Artifacts:**
- `src/security/auth.rs` — key verification (constant-time), scope parsing
- `src/security/ratelimit.rs` — token bucket + axum middleware
- `src/security/redact.rs` — log redaction for auth material
- `src/main.rs` — middleware wiring, production TLS enforcement
- `config/keys/operator.keys.example` — provisioning template
- `scripts/gen-operator-key.sh` — CSPRNG key generator

**Acceptance Criteria:**
- [x] `POST /api/command` without a valid key → `401`; with read-scope key → `403`.
- [x] Burst of >limit requests → `429` with `Retry-After`.
- [x] Production mode refuses plain-HTTP bind without explicit `--no-tls`.
- [x] No secret material appears in structured logs (grep for key prefix returns empty).
- [x] Audit entries record `actor` scope for every write.

## 4. Consequences

### Positive
- Ungoverned callers are excluded; every write is attributable and rate-bounded.
- Default-secure posture (TLS + keys) matches the local-production bar of ADR-004.
- Constant-time key compare and hashed-at-rest keys limit credential leakage impact.

### Negative / Tradeoff
- Operator must provision keys before first use (script provided).
- Token bucket adds small per-request overhead.
- WebSocket auth complicates browser-only operator UIs; a `localhost` loopback exception
  (config-gated) is provided for local dev only.

### Neutral
- mTLS is additive; not required for the base local deployment.

## 5. Security & Governance

1. **Non-Bypassability** — every write path passes authn + authz middleware; there is no
   anonymous write route.
2. **Immutable Audit** — authorization decisions are themselves audit entries (actor scope).
3. **Zero Drift** — enforcement is code-level middleware, not configuration-only.

## 6. Dependencies

- `tower-http` (already present) for middleware; `sha2` for key hashing.
- ADR-008: tool execution is gated *after* authn within the same request.
- ADR-009: secrets provisioning and `0600` mounts at deploy time.

## 7. Promotion Criteria

| Criteria | Target / Threshold | Status |
| :--- | :--- | :--- |
| Authn | 401 for missing/invalid key on writes | ✅ |
| Authz | 403 for read-only key on writes | ✅ |
| Rate limit | 429 enforced; `Retry-After` present | ✅ |
| TLS default | Production refuses plain HTTP without `--no-tls` | ✅ |
| Secret hygiene | No secrets in logs | ✅ |

## 8. Verification Notes (2026-08-08)

Live checks against the built binary (debug, `127.0.0.1`):

- Authn: `POST /api/command` no key → `401`, unknown `pmr_op_*` key → `401`; health stays open.
- Authz: read-scope key on `POST /api/command` → `403`; write-scope key → `200` with a `simulated`
  receipt and a 64-hex `witness_id`.
- Loopback exemption (dev): unauthenticated `GET /audit/entries` from loopback → `200`, but
  unauthenticated `POST /audit/entries` from loopback → `401` (writes are never exempt); with
  `PHASE_MIRROR_LOOPBACK_EXCEPTION=false` unauthenticated loopback reads → `401`; in production
  the exemption is forced off.
- Rate limit: with `--rate-limit-rps 10 --rate-limit-burst 2`, a rapid burst yields `429` with a
  `Retry-After: 1` header after the burst capacity is consumed.
- TLS: `--env production` without certs → startup error; `--tls-cert` alone → error; production
  + `--no-tls` starts but the loopback exemption is disabled; CORS wildcard origin is rejected
  in production.
- `.env`: an unknown `PHASE_MIRROR_*` key aborts startup with the offending line; known keys pass.
- WebSocket: with `--ws-auth` and loopback exemption off, upgrades without a key or with an unknown
  key → `401`; with a valid key → `101`.
- Audit attribution: write-path audit entries record the authenticated `actor` (e.g. `alice`),
  and the caller-supplied `actor` field is ignored on the write path.
- Secret hygiene: grep of the source for logging of key material shows only actor ids and rate-limit
  bucket names (never raw `pmr_op_*` values or digests).
- Test battery: agent `cargo test` 80 passed; clippy `--no-deps --all-targets -- -D warnings` clean;
  `cargo fmt --check` clean; `tsc --noEmit` clean; `vitest run` 34 passed.

No commit made; repository staging left untouched.
