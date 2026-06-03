# project-decomposer

A Rust CLI that interviews a developer about an app idea via an
LLM-driven adaptive quiz, then emits a coherent set of six markdown
artifacts to feed to a coding assistant. Ships a Claude Code plugin
(`/decompose`) that runs the interview inside an existing Claude Code
session, using the host session's auth — no separate API key required.

## Goal

Turn "I have a vague app idea" into a grounded starting point for an AI
coding assistant in one short interview. The six artifacts are designed
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
- `AGENTS.md` — the same project guidance for Codex and other
  AGENTS-aware coding agents.
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

**The plugin is fully self-contained** (as of the binary-decoupling
refactor): it ships the canonical prompt templates under
`skills/decompose/prompts/` and reads them + writes the artifacts and
`manifest.json` itself via the host conversation's Read/Write tools. It
no longer shells out to the `decomposer` binary at all — there is no
native dependency on the plugin path. The standalone CLI compiles in the
same prompt files via `include_str!`, so there's exactly one copy and the
two products can't drift. **Re-validated** with a cold live run
(`water-logged`, a water-intake CLI) on 2026-05-31: the model read the
bundled prompts, the 3-stage render held cross-artifact consistency, and
the hand-written `manifest.json` deserialized cleanly into the real
`Manifest` serde type (round-trip verified).

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
- v2 plugin: self-contained — ships the prompt templates and writes
  artifacts + manifest itself via the host conversation, no binary call.
  (The CLI retains `prompts` / `write-artifacts` subcommands as a generic
  external-driver surface, now unused by the plugin.)

What's been verified live (across PRD, ARCH, FILE_TREE, CLAUDE.md, AGENTS.md, TASKS):

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
  Anthropic + OpenAI impls, render orchestration, manifest. Compiles in
  the prompt templates from the plugin tree via `include_str!`, and
  exposes `pub fn interviewer_prompt()` / `pub fn render_prompt(kind)` for
  the standalone CLI's own subcommands.
- `decomposer-cli` (binary) — thin wrapper: argv parsing, TTY/JSON I/O,
  filesystem side of artifact writing. Also carries `prompts` /
  `write-artifacts` subcommands as a generic external-driver surface (the
  Claude Code plugin no longer uses them).

The headless-lib + thin-CLI split was originally what made the v2 plugin
viable; after the binary-decoupling refactor the plugin shares only the
prompt *text* with the CLI (via `include_str!`), not the Rust logic. The
split still structures the standalone CLI cleanly.

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

### v2 plugin — self-contained (no binary)

The Claude Code plugin at `plugin/decompose/` does everything in the host
conversation, with no `decomposer` binary call:

- **Prompts** ship as files under
  `plugin/decompose/skills/decompose/prompts/` (`interviewer.md`,
  `render_prd.md`, `render_architecture.md`, `render_file_tree.md`,
  `render_claude_md.md`, `render_tasks.md`). SKILL.md instructs the model
  to Read them as instructions. These same files are the standalone CLI's
  prompts too, pulled in via `include_str!` — one source of truth.
- **Artifact + manifest writing** is done by the model with the Write
  tool. SKILL.md specifies the slug rule, the `./decomposed/{slug}/`
  layout, and the exact `manifest.json` shape (`version: 2`, snake_case
  category/kind values, and both `claude_md` and `agents_md` guidance
  artifacts). The `Manifest` shape on disk stays the contract — it's now
  mirrored in SKILL.md prose and must be kept in sync with `manifest.rs` /
  `session.rs` if those structs change.

The plugin renders bodies via the host Claude conversation (so session
auth is inherited for free) and orchestrates the 3-stage order
explicitly.

**Legacy external-driver surface:** the CLI still has `decomposer prompts
<kind>` and `decomposer write-artifacts ...` subcommands (flat JSON in,
writes artifacts + manifest). They're no longer on the plugin path but
remain as a generic non-Claude-Code driver contract. If they're not worth
maintaining, removing them + their docs is a clean separable follow-up.

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

- **~~Re-validate self-contained plugin (post binary-decoupling).~~ DONE
  2026-05-31.** Cold `/decompose` run (`water-logged`) via
  `claude --plugin-dir`: (a) the model found and read the bundled
  `skills/decompose/prompts/*.md`, (b) the 3-stage render held
  cross-artifact consistency (binary name, Rust/SQLite/clap, configurable
  units, midnight+streak rules all consistent; rejected alts correctly in
  key-decisions; no outer fences), and (c) the hand-written `manifest.json`
  deserialized into the real `Manifest` type and round-tripped. The
  marketplace form is now unblocked.
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
- **Submit to the `claude-community` marketplace.** ✅ **SUBMITTED
  2026-05-31** via the in-app form. Now awaiting Anthropic's review; on
  approval the plugin is SHA-pinned into the community catalog and the
  public `marketplace.json` syncs nightly. (Corrected 2026-05-31 from the
  official Claude Code docs — the earlier "submit to
  `claude-plugins-official`" framing was wrong on every point.) The facts:
  - **`claude-plugins-official`** is curated by Anthropic at its sole
    discretion. **There is NO application process**, and no form/PR adds a
    plugin to it. Don't target it. (The `clau.de/plugin-directory-submission`
    short-link just 302-redirects to the docs section
    `code.claude.com/docs/en/plugins#submit-your-plugin-to-the-official-marketplace`
    — it is not a form.)
  - **`claude-community`** (`anthropics/claude-plugins-community`) is the
    public community marketplace where third-party submissions land after
    review. Users add it with
    `/plugin marketplace add anthropics/claude-plugins-community` and
    install as `@claude-community`. **This is our target.**
  - **How to submit:** one of two in-app forms (login required):
    Claude.ai → `https://claude.ai/settings/plugins/submit`, or
    Console → `https://platform.claude.com/plugins/submit`.
  - **Before submitting:** `claude plugin validate --strict ./plugin/decompose`
    must pass (it does, 2026-05-31). The review pipeline runs the same
    check + automated safety screening.
  - **After approval:** the plugin is SHA-pinned into the community catalog
    (`.claude-plugin/marketplace.json`); CI bumps the pin as we push new
    commits; the public catalog syncs nightly (so expect a delay between
    approval and appearing installable).
  Details for the form:
  - **Plugin name:** `decompose`
  - **Source repo:** `https://github.com/kilsekddd/project-decomposer`
  - **Plugin path in repo:** `plugin/decompose` (manifest at
    `plugin/decompose/.claude-plugin/plugin.json`, version `0.1.0`)
  - **Homepage:** `https://github.com/kilsekddd/project-decomposer`
  - **Security profile:** self-contained — markdown skill + bundled
    prompt files, no native binary, no MCP server, no network calls on
    the plugin path. (The binary-decoupling refactor removed the former
    `cargo install decomposer-cli` dependency.)
  **Was gated on diversity testing** — that's cleared, and the post-refactor
  re-validation gate is now also cleared. All prerequisites below are done;
  the community form is ready to submit. Prerequisites:
  - [x] Re-validate the self-contained plugin flow post-refactor (cold
        `/decompose` run `water-logged`, 2026-05-31): model read the bundled
        prompts, 3-stage render held consistency, and the model-written
        `manifest.json` deserialized into the real `Manifest` type
        (round-trip verified).
  - [x] Library-shaped idea exercised end-to-end (`webvtt-parser-lib`).
  - [x] Web-service / daemon idea exercised end-to-end (`stripe-webhook-relay`).
  - [x] One-off-script idea exercised end-to-end (`topfiles`).
  - [x] Any bugs surfaced by those runs fixed. The
        "architect-addresses-this" category loophole exposed by
        `stripe-webhook-relay` was closed by tightening the
        interviewer prompt; the `brookmark` test then asked all 9
        categories explicitly and recorded the resolution in the
        readiness summary (commit `c0e21d1`). Also fixed: the `stack`
        category was missing from the v1 ask_next_question JSON
        schema, which would have broken v1 standalone for any
        stack-category question.
  - [x] v1 ↔ v2 parity audit done for the common case:
        `signal_ready` now carries an optional `project_name` field,
        the engine calls `session.rename` when present, and the
        standalone CLI gets the same slug-from-committed-name
        behavior as the plugin's `--name` flag (commit `c0e21d1`).
        **Known limitation:** when the user defers naming and the
        architect commits the name during render, the standalone CLI
        slug still derives from `idea` — there's no post-render
        rename hook. The plugin path handles this case because
        SKILL.md instructs the model to read the committed name from
        the rendered ARCH and compute the output slug from it directly.
  - [x] LICENSE files committed (MIT OR Apache-2.0).
  - [x] README is presentable on the GitHub front page.

## Install (end users)

```sh
# Add the marketplace
claude plugin marketplace add kilsekddd/project-decomposer

# Install the plugin (self-contained — no binary needed)
claude plugin install decompose@project-decomposer
```

Restart Claude Code, then run `/decompose` in an empty project
directory.

The standalone Rust CLI (BYO-API-key, non-Claude-Code use) is a separate
install and is **not** required for the plugin:

```sh
cargo install --git https://github.com/kilsekddd/project-decomposer decomposer-cli
```
