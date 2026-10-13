<<<<<<< HEAD
# MCP automation

Status: workflow contract.
Published anchor: PhaseMirror/phase-mirror-agent-dev, branch `master`.

## Triggers

`.github/workflows/evolution.yml` runs on push to `master`, on a pull request targeting `master`, and on manual dispatch. It does not target `main`. The tree's branch is `master`.

`.github/workflows/mcp-automation.yml` runs only on `workflow_dispatch`.

## Validation gates

`evolution.yml` is hermetic. It runs, in order:

1. `cargo check --workspace`
2. `cargo test --workspace`
3. `cargo build -p phase-mirror-mcp --release`
4. `cargo test -p phase-mirror-mcp`

There is no `--check-model` flag. The MCP binary does not accept one. There is no webhook, no `webhook.site` post, and no simulated payload. A step that posts to an external host and ignores failure is a simulation, and it is not in this workflow.

## Dispatch commands

`mcp-automation.yml` accepts one input, `command`, as a choice. The choices are `build`, `test`, `validate`, and `check-contract`.

| Command | What runs |
|---|---|
| `build` | `cargo build -p phase-mirror-mcp --release` |
| `test` | `cargo test -p phase-mirror-mcp` |
| `validate` | `cargo check --workspace`, then `cargo test --workspace` |
| `check-contract` | `cargo test -p lexlean-stratum` |

Anything else is refused. Arbitrary cargo from the dispatch input is not accepted. A free-text command would be shell injection, and it is not a gate.

## Boundaries

SPEC.md §34.0: the compiler's `seal_hash` preimage is not recoverable. These workflows do not call `pirtm_verify_ensemble` and do not treat a PIRTM seal as a crate fixture.

ADR-015 and the zero-simulation rule: a CI step that cannot fail, or that reports a result it did not compute, is not a gate. The previous `evolution.yml` targeted `main`, ran a flag the binary does not have, and posted a simulated payload to `webhook.site`. That file is replaced.
=======
# MCP automation workflow setup

This repository now includes GitHub Actions automation for the Rust workspace and for the Phase Mirror MCP server.

## Included workflows

### 1. Phase Mirror CI & MCP validation (`.github/workflows/evolution.yml`)

The primary continuous integration workflow gates all pushes and pull requests against `master`:
- **Triggers**:
  - `push` to `master`
  - `pull_request` targeting `master`
  - Manual execution via `workflow_dispatch`
- **Validation Gates**:
  - Full workspace syntax and type check: `cargo check --workspace`
  - Workspace test suite execution: `cargo test --workspace` (validates 187 tests, including `lexlean-stratum` contractivity and stratification witness)
  - Release build of MCP server: `cargo build -p phase-mirror-mcp --release`
  - MCP integration test verification: `cargo test -p phase-mirror-mcp`
- **Architectural Policy**:
  - Replaces legacy simulated webhook dispatches and mock `--check-model` invocations with concrete, hermetic on-tree compilation and test execution (ADR-015, SPEC.md §34.0).

### 2. Phase Mirror MCP Automation (`.github/workflows/mcp-automation.yml`)

On-demand operational automation for the Phase Mirror Model Context Protocol server:
- **Triggers**:
  - `workflow_dispatch` with parameter `command`
- **Supported Commands**:
  - `build`: Compiles `phase-mirror-mcp` in release mode.
  - `test`: Executes the full 17-test MCP suite (`governance`, `prismpm_actions`, `stability_witness`, `runtime_harness`, `stonehenge_simulation`).
  - `validate`: Runs full release build followed by MCP and `lexlean-stratum` contractivity validation.
  - `check-contract`: Runs MCP contract governance harness.
  - Custom cargo subcommands passed to `phase-mirror-mcp`.

## Architectural Alignment

- **Ground Truth Invariant (ADR-015)**: Workflows execute physical Rust crate binaries directly on the host toolchain without synthetic mocks.
- **Process Boundary Quarantine (SPEC.md §34.0)**: CI treats `phase-mirror-mcp` as an integration boundary. Test suites preserve exact rational norm bounds without introducing float literals or fake receipts into `lexlean-stratum`.
>>>>>>> 0d2abf2 (Refactor CI workflows: update evolution.yml for master branch and add mcp-automation.yml for on-demand automation commands)
