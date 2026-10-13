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
