# project-decomposer

A Rust CLI that interviews a developer about an app idea via an
LLM-driven adaptive quiz, then emits a coherent set of five markdown
artifacts to feed to a coding assistant. Ships a Claude Code plugin
(`/decompose`) that runs the interview inside an existing Claude Code
session, using the host session's auth — no separate API key required.

## Goal

Turn "I have a vague app idea" into a grounded starting point for an AI
coding assistant in one short interview. The five artifacts are designed
to be dropped into an empty project directory and read by an assistant
(e.g. Claude Code) as the canonical brief:

- `PRD.md` — problem, users, goals, non-goals, user journeys, success
  criteria.
- `ARCHITECTURE.md` — high-level shape, components, data model,
  external surfaces, key decisions + rejected alternatives, open
  questions. The canonical resolver for every concrete decision
  (project shape, language, stack, persistence, interfaces,
  deployment, naming).
- `FILE_TREE.md` — directory layout with one-line per-path responsibilities.
- `CLAUDE.md` — short, declarative guidance for an AI assistant: stack,
  conventions, things to avoid, run/build/test.
- `TASKS.md` — ordered checkbox build plan grouped by milestones, with
  file-touch annotations.

Plus `manifest.json` capturing the session, model, transcript, readiness
summary, and artifact paths.

## Status

**v1 standalone CLI** is functional and end-to-end smoke-tested on Opus 4.7.

**v2 Claude Code plugin** ships at `plugin/decompose/`, validated on two
live runs (a Rust CLI tool, a NeoForge Minecraft mod). The plugin uses
the host Claude conversation as the model — no `ANTHROPIC_API_KEY`
required on the plugin path. Slug, summary, and artifact rendering all
hold cross-artifact consistency.

What works:

- Two providers in v1 standalone: Anthropic (Messages API + tool_use)
  and OpenAI (Chat Completions + function calling). Provider-agnostic
  `LlmClient` trait.
- Interactive TTY mode (`rustyline`) and machine-readable `--json` mode
  on the same code path.
- Adaptive interview driven by two tools: `ask_next_question` and
  `signal_ready`. Budget enforced on `transcript.len()`, so `/back`
  rewinds without expanding the budget.
- Resume from a prior `manifest.json`: continues a half-done interview
  or re-renders artifacts from a completed session.
- 9 unit/integration tests pass; `MockClient` covers the engine loop
  without API access.
- v2 plugin: `decomposer prompts <kind>` and `decomposer write-artifacts`
  subcommands expose the prompt templates and artifact writer to the
  plugin without it constructing any HTTP client.

What's been verified live (across PRD, ARCH, FILE_TREE, CLAUDE.md, TASKS):

- Cross-artifact name consistency — slug derives from the committed
  project name via `Session::rename`, not from the user's vague idea
  string.
- No outer code-fence wrapping on any artifact (defensive fence-strip
  in `write-artifacts` for the plugin path; explicit prompt directive
  in `render_claude_md.md` for v1).
- `TASKS.md` respects PRD non-goals and ends with an explicit
  "Out of scope (not scheduled)" section.
- Architect commits to every concrete decision the interview surfaces;
  no "X or Y" hedging in FILE_TREE / CLAUDE.md / TASKS.
- Anti-drift discipline: interviewer probes the `stack` category
  (project shape, language, framework, persistence, deployment,
  naming); every category and every stack item lands in one of the
  named states (covered-in-transcript, covered-by-idea-string,
  committed, deferred, conditional, or N/A-with-reason). The readiness
  summary enumerates which stack decisions are user-committed vs
  architect-committed.

## Architecture

Cargo workspace, two crates:

- `decomposer-core` (library) — engine, session, provider trait,
  Anthropic + OpenAI impls, render orchestration, manifest, prompts.
  Exposes `pub fn interviewer_prompt()` and `pub fn render_prompt(kind)`
  for the plugin path.
- `decomposer-cli` (binary) — thin wrapper: argv parsing, TTY/JSON I/O,
  filesystem side of artifact writing, plus the plugin-path subcommands
  (`prompts`, `write-artifacts`).

The headless-lib + thin-CLI split is what makes the v2 plugin viable.

### Render contract — 3 stages, not parallel

`engine::render_all` (v1) and the plugin's Phase 2 (v2) both run renders
in three sequential stages. **Load-bearing for cross-artifact consistency.**
Do not collapse to all-parallel.

1. **PRD** alone. Establishes names, scope, non-goals from the
   transcript.
2. **ARCHITECTURE** with PRD as prior context. Required to pin every
   ambiguity the PRD leaves open. The prompt explicitly bans "X or Y"
   hedging in the doc body — rejected alternatives go in the
   key-decisions section. Transcript-stated user preferences are hard
   constraints; architect commits only on what the user deferred.
3. **FILE_TREE, CLAUDE.md, TASKS** in parallel, each given PRD +
   ARCHITECTURE as prior context, instructed to "honor every concrete
   decision ARCHITECTURE.md committed to."

### v1 `--json` protocol

Used by external drivers other than the Claude Code plugin. One JSON
object per line both directions:

- Out: `{"type":"question", "turn":N, "of_max":M, "category":"...", "question":"..."}`
- In: `{"type":"answer", "text":"..."}`
- Out (terminal): `{"type":"done", "manifest_path":"..."}`

### v2 plugin contract

The Claude Code plugin at `plugin/decompose/` calls two subcommands:

- `decomposer prompts <kind>` — prints the canonical prompt for one of
  `interviewer | prd | architecture | file-tree | claude-md | tasks`.
  No HTTP. No env var requirements.
- `decomposer write-artifacts --idea <str> [--name <str>] [--summary <str>]
  --transcript <file> --bodies <file>` — writes the five artifacts +
  `manifest.json`. Takes flat JSON inputs (idea + transcript array +
  bodies map); the binary constructs the `Session`, re-slugs from `--name`
  if given, and emits `{"type":"done","manifest_path":"..."}`.

The plugin renders bodies via the host Claude conversation (so session
auth is inherited for free) and orchestrates the 3-stage order
explicitly. The `Manifest` shape on disk is part of the contract.

## Cost & latency shape

**v1 standalone** per session, on Opus 4.7 (≈$15/Mtok input, $75/Mtok output):

- Interview: one call per question. Min 6, max 15 questions default
  → typically ~$0.05–0.20 depending on transcript depth.
- Render: 5 artifact calls (1 PRD, 1 ARCH, 3 parallel). PRD + ARCH are
  injected into the final 3 as ~6–10 kB of prior context. Rendering
  cost typically ~$0.30–0.60 on Opus.
- Wall clock: roughly 3× the original parallel render (3 sequential
  stages instead of 1). Acceptable price for cross-artifact consistency.

On Haiku 4.5 (≈$1/Mtok input, $5/Mtok output), the same session is
roughly 1/10th the cost.

**v2 plugin path** uses the host Claude Code session's billing — no
direct API calls from the binary. For Pro/Max subscribers the marginal
session cost is effectively zero; for API-key Claude Code users it bills
to the same key, no second key to provision.

## Follow-ups

Tracked here so they don't get lost between sessions.

- **v1 ↔ v2 parity audit.** The interactive standalone path
  (`decomposer "<idea>"`) doesn't surface a "what should the project be
  called" question, so the slug still derives from the idea string
  rather than the committed name. The new prompts (stack category,
  checklist review, idea-covers-category) propagate fine because
  they're just text, but the engine doesn't know about `--name`. Worth
  a small interactive-flow patch.
- **Prompt caching on v1 renders.** Each of the 3 leaf renders re-sends
  ~6–10 kB of PRD+ARCH context with no cache breakpoint. Adding
  `cache_control: ephemeral` on the system prompts plus the PRD/ARCH
  prior blocks would cut v1 render cost by roughly 30–50% per session
  (Anthropic only; plugin path is host-conversation so it doesn't
  benefit).
- **Diversity testing.** Run `/decompose` cold on a few more idea
  shapes — a library, a web service, a one-off script — to stress-test
  whether the checklist-review and idea-covers-category rules hold
  beyond CLI tools and game mods. Each run is ~$1.
- **Resume / re-render in the plugin path.** v1 has resume via
  `--resume <manifest>`; the plugin path currently has no equivalent.
  If a user wants to revise after seeing the output they have to re-run
  from scratch.
- **More providers.** v1's `LlmClient` trait has Anthropic + OpenAI.
  Adding a third provider (e.g. local Ollama, Bedrock) is a contained
  task under `crates/decomposer-core/src/provider/`.
- **LICENSE.** No license is present; required if anyone other than
  the author should use the code.
- **Submit to `claude-plugins-official`.** Once the plugin has been
  exercised on a wider variety of ideas and is stable, a PR to
  `anthropics/claude-plugins-official` would make `decompose` installable
  via the default marketplace instead of requiring users to add this
  repo as a custom marketplace.

## Install (end users)

```sh
# Add the marketplace
claude plugin marketplace add kilsekddd/project-decomposer

# Install the plugin
claude plugin install decompose@project-decomposer

# Install the binary (required — the plugin shells out to it)
cargo install --git https://github.com/kilsekddd/project-decomposer decomposer-cli
```

Restart Claude Code, then run `/decompose` in an empty project
directory.
