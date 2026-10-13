# PM-MCP-003 — Local stdio registration, not a cloud binding

Status: receipt. The registration flag is on the tree. The local binary and the 187 run were not re-executed here.
Date: 2026-10-12
Published head: PhaseMirror/phase-mirror-agent-dev `095fbf04c4529b2d7c06250205abca8b7d825015`

## What the tree has

`phase-mirror-mcp/src/main.rs` handles `--register-mcp`. It writes `~/.lmstudio/mcp.json` with `type: stdio`, `command` set to `current_exe()`, `cwd` set to `current_dir()`, `PHASE_MIRROR_ENV=dev`, and `tools: ["*"]`.

`mcp-contract.json` is at the repo root. The server loads it from the process cwd and exits 1 if that fails.

## What the config is

The pasted block is a local stdio client entry. A local IDE that can execute that path can spawn the binary. A GitHub Copilot Cloud Agent container cannot open `/media/citizen/…`. That partition is not in the cloud workspace.

`tools: ["*"]` is a client allowlist, not an MCP protocol field the server enforces. It tells the client to offer every tool the server advertises, including the prismpm tools. Those still resolve only when `PRISMPM_BIN` matches a lock digest.

## What is not closed

`.github/workflows/evolution.yml` and `mcp-automation.yml` still contain `<<<<<<< HEAD` markers at this head. GitHub Actions will not parse them. The local registration does not repair that.
