# Privacy Statement — decompose

The `decompose` Claude Code plugin collects no personal data, transmits no
data, and uses no telemetry, analytics, or third-party services.

## What the plugin does

- It runs entirely inside your existing Claude Code session. It makes no
  network calls of its own, bundles no MCP server, and contacts no external
  service.
- Your interview answers are processed by your host Claude Code
  conversation (subject to Anthropic's privacy policy) solely to generate
  the output documents. The plugin adds no separate processing or
  transmission.
- All output — `PRD.md`, `ARCHITECTURE.md`, `FILE_TREE.md`, `CLAUDE.md`,
  `TASKS.md`, and `manifest.json` — is written to your local filesystem
  under `./decomposed/{slug}/`. Nothing is uploaded or shared by the plugin.

## What the plugin does NOT do

- No data collection, profiling, or tracking.
- No telemetry, usage analytics, or crash reporting.
- No accounts, no servers, no third-party APIs on the plugin path.

## Standalone CLI (separate from the plugin)

This project also ships an optional standalone Rust CLI (`decomposer-cli`)
for non-Claude-Code use. That CLI sends your interview answers to the LLM
provider you configure (Anthropic or OpenAI) using your own API key, under
that provider's privacy terms. The CLI is not part of the plugin and is not
involved when you use `/decompose` in Claude Code.

## Contact

Questions: https://github.com/kilsekddd/project-decomposer/issues
