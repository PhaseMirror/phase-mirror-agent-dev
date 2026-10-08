# Contributing to Phase Mirror Agent

Thank you for your interest in contributing to the Phase Mirror Agent! 

## Architectural Boundaries

This repository strictly follows the **Option 3 Tool Boundary** established for the Phase Mirror integration:
- **Strict Lockfile Dependence:** The MCP server and agent do not blindly trust `PATH`. They require a strict `prismpm.lock` to consume the PrismPM SDK.
- **No Reimplemented Methodology:** This repository serves as a *consumer* and mediator of PrismPM, using standard OS commands to interact with the tooling. The core methodologies are not dual-sourced or reimplemented here.
- **Evidence Isolation:** Evidence reads are strictly isolated to methodology-scoped files and prevent path traversal.

## Getting Started

1. Ensure you have the proper Rust toolchain installed.
2. Initialize the environment using the provided development containers.
3. Follow `just vv` for acceptance testing. The boundary relies on the SDK image matching `prismpm.lock`.

## Pull Requests

1. Open issues to discuss proposed changes before starting major work.
2. Ensure that any added capabilities correspond to registered claims and pass all strict gating checks.
3. Run formatting and linting (`cargo fmt`, `cargo clippy`).
4. Ensure `just vv` passes locally.
