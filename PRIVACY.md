# Privacy Statement — project-decomposer plugins

The `decompose` Claude Code plugin and `decompose-codex` Codex plugin collect
no personal data, transmit no data, and use no telemetry, analytics, or
third-party services.

## What the plugins do

- They run entirely inside your existing host conversation: Claude Code for
  `decompose`, Codex for `decompose-codex`. They make no network calls of
  their own, bundle no MCP server, and contact no external service.
- Your interview answers are processed by the host conversation solely to
  generate the output documents. The plugins add no separate processing or
  transmission.
- All output — `PRD.md`, `ARCHITECTURE.md`, `FILE_TREE.md`, `CLAUDE.md`,
  `AGENTS.md`, `TASKS.md`, and `manifest.json` — is written to your local filesystem
  under `./decomposed/{slug}/`. Nothing is uploaded or shared by either plugin.

## What the plugins do NOT do

- No data collection, profiling, or tracking.
- No telemetry, usage analytics, or crash reporting.
- No accounts, no servers, no third-party APIs on either plugin path.

## Standalone CLI (separate from the plugins)

This project also ships an optional standalone Rust CLI (`decomposer-cli`)
for BYO-API-key use outside the host-session plugins. That CLI sends your
interview answers to the LLM provider you configure (Anthropic or OpenAI)
using your own API key, under that provider's privacy terms. The CLI is not
part of either plugin and is not involved when you use `/decompose` in Claude
Code, `$decompose`, or `/prompts:decompose` in Codex.

## Contact

Questions: https://github.com/kilsekddd/project-decomposer/issues
