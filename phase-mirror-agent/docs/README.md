# phase-mirror-agent — Documentation

Operational and architectural documentation for the phase-mirror-agent governance
gateway. Start with the [package README](../README.md) for the overview, then use
this index to find the right guide.

## Operator guides

| Guide | When to use |
| :--- | :--- |
| [Deployment guide](deployment.md) | Standing up the agent with Docker Compose or systemd, including TLS. |
| [Runbook](runbook.md) | Day-to-day operations: start/stop/restart, upgrade, key rotation, log access. |
| [Backup & restore](backup-restore.md) | Protecting and recovering the audit WAL (ADR-005). |

## Architecture decisions

The full ADR series lives in [docs/adr](adr/README.md). The most relevant ones:

- **ADR-004** — master plan: production-grade local deployment (7-phase roadmap).
- **ADR-005** — persistent, immutable, hash-chained audit store (the WAL).
- **ADR-006** — authentication, authorization, rate limiting, secrets.
- **ADR-007** — observability (JSON logs, `/metrics`) and reliable shutdown.
- **ADR-008** — governed tool-execution backplane (no stub receipts).
- **ADR-009** — packaging & deployment (Compose + systemd) — this phase.

## Development log

A phase-by-phase account of how the package was built and verified:
[DEVELOPMENT-LOG.md](DEVELOPMENT-LOG.md).

## Repository layout

```
.
├── Cargo.toml                  # Rust server (deployable binary)
├── package.json                # TS ALP-NLP library (embedded core)
├── Dockerfile                  # multi-stage image (non-root runtime)
├── .env.example                # environment template (mirrors config/env.schema.json)
├── deploy/
│   ├── compose.yaml            # Docker Compose deployment
│   └── systemd/                # hardened systemd unit
├── config/
│   ├── env.schema.json         # PHASE_MIRROR_* key contract
│   ├── tools.toml.example      # governed tool backplane config
│   └── keys/                   # operator keys (0600, hashed at rest)
├── scripts/                    # keygen, audit backup/rotate
├── schema/                     # JSON Schema for witness/audit/receipt contracts
└── docs/
    ├── README.md               # this index
    ├── deployment.md           # compose + systemd + TLS
    ├── runbook.md              # operations runbook
    ├── backup-restore.md       # WAL backup/restore/verify
    ├── DEVELOPMENT-LOG.md      # phase-by-phase development account
    └── adr/                    # architecture decision records
```
