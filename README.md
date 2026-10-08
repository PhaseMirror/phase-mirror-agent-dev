---
title: Phase Mirror Agent Development Workspace
description: Autonomous evolution and development repository for the Phase Mirror Agent, integrated with the Oracle MCP Server and PrismPM SDK boundary.
status: Active
license: MIT OR Apache-2.0
---

# Phase Mirror Agent Dev Environment

An exclusive, self-contained development repository for the continuous evolution of the **Phase Mirror Agent**. This workspace unites the autonomous agent, its Oracle MCP Server, and the PrismPM SDK integration under a cohesive architectural boundary driven by webhooks and GitOps automation.

## 🎯 Purpose

This repository operationalizes the Phase Mirror methodology for agentic systems:
- **Autonomous Evolution:** The agent evolves its own capabilities in response to GitHub webhook events.
- **Strict Methodological Boundary:** The MCP Server provides a closed, structural interface to the `prismpm` SDK (Model Check, Verify, Generate, Evidence Read), preventing the agent from leaking compiler implementations into its own logic.
- **Continuous Verification:** Git workflows automatically validate the agent's structural compliance and run the verification matrices against newly evolved policies.

## 🏗️ Architecture

The workspace contains four primary architectural pillars structured as a unified Cargo workspace:

1. **`phase-mirror-agent`**: The core autonomous system capable of multi-turn reasoning and capability evolution.
2. **`phase-mirror-mcp`**: The Model Context Protocol (MCP) server. Acts as the rigorous Oracle Boundary, exposing the PrismPM SDK strictly via constrained tools.
3. **`phase-mirror-surface` & `phase-mirror-gpt`**: Sub-components handling the agentic surface interfaces, structured logging, and integration mappings.

### The Oracle Joint (Option 3 Implementation)
The MCP server enforces the Option 3 methodology:
- Cannot execute arbitrary commands.
- Binds strictly to the `prismpm.lock` environment.
- Validates structural shapes for SDK primitives before relaying actions.

## 🚀 Getting Started

### Prerequisites
- [Rust Toolchain](https://rustup.rs/) (v1.89+)
- Devcontainer runtime (to guarantee identical execution environments)
- `gh` CLI (for automated PR and issue handling via MCP)

### Bootstrapping

1. **Clone the repository:**
   ```bash
   git clone <repository-url> phase-mirror-agent-dev
   cd phase-mirror-agent-dev
   ```

2. **Launch the MCP Server:**
   ```bash
   cargo run -p phase-mirror-mcp
   ```

3. **Start the Agent & Webhook Listener:**
   ```bash
   cargo run -p phase-mirror-agent -- --listen-webhooks
   ```

## 🔄 Automation & Webhooks

This repository orchestrates its own evolution:
- **`.github/workflows/evolution.yml`**: On every commit to `main`, this action builds the Oracle MCP, runs bounded model checks, and dispatches an evolution webhook.
- **Agent Webhook Listener (`/webhook/github`)**: Listens to repository events, validates the payload, and dynamically executes `prismpm_verify` through the MCP context to formalize its new behavior.

## 📜 Contributing

When contributing to this workspace, respect the Oracle bounds:
1. Do not vendor or implement PrismPM logic directly in the agent or the server.
2. Extend capabilities by exposing explicit schema-bound tools in the MCP.
3. Validate boundaries using local plants before merging.

## ⚖️ License

Dual-licensed under the terms of both the MIT License and the Apache License (Version 2.0).
