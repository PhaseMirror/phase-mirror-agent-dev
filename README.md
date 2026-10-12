---
title: Phase Mirror Agent Development Workspace
description: Rust workspace for the Phase Mirror governance stack, including the agent, MCP server, GPT governance surface, and API contracts.
status: Active
license: Phase Mirror License v1.0
---

# Phase Mirror Agent Development Workspace

This repository is the development workspace for the Phase Mirror stack: a governance-first set of Rust components designed to enforce integrity, verification, and policy boundaries in agentic workflows.

The project currently contains a Cargo workspace with four primary packages:

- `phase-mirror-agent` — the core governance gateway and execution runtime
- `phase-mirror-mcp` — the MCP server boundary for structured tool access
- `phase-mirror-gpt` — governance and semantic validation surface
- `phase-mirror-surface` — shared surface/API contracts

The design prioritizes fail-closed behavior, provenance logging, verification gates, and explicit policy enforcement over convenience or unrestricted automation.

## Project status

The repository is actively maintained as a Rust workspace and reflects a mature development direction rather than a placeholder prototype.

Current focus areas include:

- integrity-first command execution and policy gates
- durable, append-only audit and provenance flows
- structured MCP interfaces for governed tool use
- governance and verification surfaces for agentic operations
- deployment and operational readiness guidance within the workspace

## Workspace structure

```text
phase-mirror-agent-dev/
├── Cargo.toml
├── LICENSE
├── README.md
├── CONTRIBUTING.md
├── SECURITY.md
├── phase-mirror-agent/
├── phase-mirror-mcp/
├── phase-mirror-gpt/
├── phase-mirror-surface/
└── .github/
```

## Components

### `phase-mirror-agent`

The core runtime for governed command processing, auditability, and operational control. This crate is the main execution layer and contains the agent-side orchestration and validation flow.

### `phase-mirror-mcp`

The MCP server provides an explicitly constrained interface for tool access and governance enforcement. It acts as a structured boundary between external systems and the agent's operational surfaces.

### `phase-mirror-gpt`

The GPT-facing governance layer focuses on plan validation, semantic guards, and policy-aware execution checks. It provides the governance surface expected by an agentic workflow.

### `phase-mirror-surface`

A shared layer for surface contracts and interface definitions used across the workspace.

## Getting started

### Prerequisites

- Rust toolchain (current workspace target supports recent stable Rust)
- Cargo
- Git
- Optional: an MCP-capable client such as Claude Desktop or another compatible host

### Build

```bash
git clone https://github.com/PhaseMirror/phase-mirror-agent-dev.git
cd phase-mirror-agent-dev
cargo build --workspace
```

### Test

```bash
cargo test --workspace
```

### Run the agent

```bash
cargo run -p phase-mirror-agent -- --help
```

For specific runtime details and operational usage, refer to the package-level README files in each crate.

## Development and contribution

Contributions are welcome, but the project is intentionally governed by strict boundaries. Review the existing repository guidance before proposing changes.

- `CONTRIBUTING.md` — contribution process and expectations
- `SECURITY.md` — vulnerability reporting and security expectations

## Licensing

This repository is licensed under the Phase Mirror License v1.0.

See the repository's `LICENSE` file for the full terms.

Important: this project is not dual-licensed under MIT OR Apache-2.0. The active licensing for this repository is the `Phase Mirror License` included in the root of this project.

The license contains operational and use restrictions, including limits on hosted/managed service use and other conditions that should be reviewed before deployment, commercialization, or redistribution.

## Security

If you find a security issue, please follow the procedures in `SECURITY.md` rather than opening a public discussion for a sensitive issue.

## Repository intent

Phase Mirror is not a permissive “run anything” agent workspace. It is a structured, verification-oriented system intended to apply governance, evidence, and strict boundaries to agentic decision-making and execution.
