# Architecture Decision Records (ADR)

This directory contains ADRs for the Phase Mirror Agent package.

## ADR Index

| Number | Title | Status |
| :-- | :-- | :-- |
| ADR-001 | ALP-Based NLP Replaces LLM for Phase Mirror Agent | accepted |
| ADR-002 | ALP-NLP Production Grade Completion | proposed |
| ADR-003 | ALP-NLP Production Grade Completion (implementation template) | template |
| ADR-004 | Master Plan — Production-Grade Local Deployment | proposed |
| ADR-005 | Persistent, Immutable Audit Store | accepted |
| ADR-006 | Authentication, Authorization & Rate Limiting | accepted |
| ADR-007 | Observability & Reliable Shutdown | accepted |
| ADR-008 | Governed Tool-Execution Backplane | accepted |
| ADR-009 | Packaging & Deployment (Docker Compose + systemd) | accepted |
| ADR-010 | Testing & CI Governance | proposed |
| ADR-011 | Unified Sovereign Governance Pipeline & Phase 7 GA Promotion | proposed |

> Note: the index previously listed ADR-003 as "ALP-NLP Production Grade Completion";
> that content lives in `ADR-002-ALP-NLP-Production-Grade.md`. ADR-004 onward form the
> production-deployment series. The master roadmap is ADR-004.

## Phase Status (ADR-004 master plan)

| Phase | Owner | Status |
| :-- | :-- | :-- |
| Phase 1 — Durable, immutable audit store | ADR-005 | ✅ |
| Phase 2 — Governance correctness | ADR-008 | ✅ |
| Phase 3 — Security: authn/authz, rate limiting, TLS | ADR-006 | ✅ |
| Phase 4 — Observability & reliability | ADR-007 | ✅ |
| Phase 5 — Packaging & deployment | ADR-009 | ✅ |
| Phase 6 — Documentation & runbooks | ADR-009 | ✅ |
| Phase 7 — Promotion & GA | all | ⬜ |

Phases 1–6 are accepted and verified (see each ADR's verification notes and
[docs/DEVELOPMENT-LOG.md](../DEVELOPMENT-LOG.md)). Phase 7 remains: final promotion
audit and the `v1.0.0` release tag.

## Usage

1. Create a new ADR: copy `ADR-TEMPLATE.md` to a new file with sequential number
2. Fill in all sections (Context, Decision, Consequences, Security)
3. Update this index table
4. Link from relevant code via `witness_hash` references