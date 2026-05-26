# project-decomposer

**Turn a vague app idea into a coherent build brief — and keep your AI
coding assistant from re-architecting the project every time you add a
feature.**

`project-decomposer` interviews you about an app you want to build, then
produces five canonical markdown documents — `PRD.md`,
`ARCHITECTURE.md`, `FILE_TREE.md`, `CLAUDE.md`, `TASKS.md` — plus a
`manifest.json`. Drop them into an empty project directory and an AI
coding assistant has the brief it needs to build the project without
drifting.

The defining property is **commitment**. By the end of the interview,
every major architectural decision — language, framework, persistence,
deployment, naming — is one of:

- **user-committed** (you chose it; the architect treats it as
  immovable),
- **user-deferred** (you said "pick something sensible"; the architect
  commits on your behalf, with reasoning),
- **conditional** (you preferred X if it fits, else whatever), or
- **N/A with reason** (you said it doesn't apply, and *why*).

There are no silent gaps for the assistant to fill in later and re-fill
in differently next session. That's the whole point.

---

## Two ways to use it

### As a Claude Code plugin (recommended)

Runs the interview inside your existing Claude Code conversation. No
separate API key required — the plugin uses your Claude Code session's
auth (works for both API key and Pro/Max OAuth).

```sh
claude plugin marketplace add kilsekddd/project-decomposer
claude plugin install decompose@project-decomposer
cargo install --git https://github.com/kilsekddd/project-decomposer decomposer-cli
```

Restart Claude Code, open it in an empty project directory, then
type `/decompose`.

### As a standalone Rust CLI

For automation, CI, or when you're not in Claude Code. Brings its own
LLM API key.

```sh
cargo install --git https://github.com/kilsekddd/project-decomposer decomposer-cli

export ANTHROPIC_API_KEY=sk-ant-...
decomposer "a CLI tool that summarizes git diffs"

# or use OpenAI
export OPENAI_API_KEY=sk-...
decomposer --provider openai "a CLI tool that summarizes git diffs"
```

Both modes use the exact same prompt templates and produce the exact
same artifact shape. The plugin path just routes the LLM calls through
your existing Claude Code session instead of making them itself.

---

## What it produces

For a single interview about *"a Minecraft mod that turns chickens into
flaming missiles when disturbed"*:

```
decomposed/chixpocalypse/
├── PRD.md             # problem, users, goals, non-goals, user journeys, success criteria
├── ARCHITECTURE.md    # components, data model, key decisions + rejected alternatives
├── FILE_TREE.md       # full directory layout with per-path responsibilities
├── CLAUDE.md          # stack, conventions, things to avoid, build/run/test
├── TASKS.md           # ordered milestone checklist with file-touch annotations
└── manifest.json      # session metadata + readiness summary
```

The output directory is named after the project name committed during
the interview (`chixpocalypse`), not the verbose idea string you typed.
Every artifact uses the same project name, same crate versions, same
module names, and same non-goals. Drift across artifacts is the failure
mode the whole pipeline is designed to prevent.

---

## What "no drift" looks like in practice

The interview surfaces architectural decisions explicitly and the
readiness summary records who decided what. A real summary from one of
the test runs:

> *User-committed:* NeoForge mod for Minecraft 1.21.1, Java, mod ID
> `chixpocalypse`, required on both client and server. Gameplay
> (committed): disturbed chickens (punched, nearby block broken, or
> any player within 5 blocks 360°) become flaming ballistic missiles
> that arc, explode on impact with feather particles, hurt the player,
> and do minor terrain damage (≤2 blocks); strictly chickens; held
> white banner disengages nearby chickens; 100-missile-per-chunk cap.
> Interfaces (committed): TOML config plus in-game commands.
> *Architect must decide:* package/namespace layout, config-key names,
> blacklist key shape (NBT tag vs name-tag match), exact command
> surface, missile entity registration, and trigger sound/particle
> details. *N/A:* app-level persistence beyond TOML — no data model;
> surrender state is derived per-tick from the held item.

`ARCHITECTURE.md` is then instructed to:

- Treat user-committed decisions as **hard constraints** — no
  alternatives presented in the doc body, no re-litigation.
- **Commit** on architect-deferred items with explicit reasoning in a
  key-decisions section.
- **Leave N/A items unbuilt** — no speculative scaffolding for things
  the user said don't apply.

The remaining three artifacts (`FILE_TREE`, `CLAUDE.md`, `TASKS`) then
inherit from `ARCHITECTURE` without re-opening any of its commitments.

---

## How it works

A three-stage render pipeline, not a single all-parallel call. This is
load-bearing.

1. **PRD** alone — establishes names, scope, non-goals from the
   transcript.
2. **ARCHITECTURE** with the PRD as prior context — commits every
   concrete decision the PRD leaves open. The prompt explicitly bans
   "X or Y" hedging in the doc body; rejected alternatives go in a
   key-decisions section, not inline.
3. **FILE_TREE, CLAUDE.md, TASKS** in parallel, each with `PRD` and
   `ARCHITECTURE` as prior context, instructed to *"honor every
   concrete decision ARCHITECTURE.md committed to."*

Early experiments rendered all five artifacts in one parallel batch.
That produced inconsistencies: binary names drifted, dependency choices
contradicted each other, and TASKS would hedge with `.*` file
extensions while FILE_TREE committed to `.rs`. The three-stage pipeline
fixes this by forcing `ARCHITECTURE` to be the canonical resolver.

---

## Cost & latency

**Plugin path** (Claude Code session does the LLM work): billed to your
existing Claude Code subscription. Pro/Max subscribers pay nothing
extra; API-key Claude Code users see it on the same key.

**Standalone path** (Rust CLI makes API calls directly):

| Stage      | Anthropic Opus 4.7 | Anthropic Haiku 4.5 |
| ---------- | ------------------ | ------------------- |
| Interview  | ~$0.05–0.20        | ~$0.01–0.02         |
| Render     | ~$0.30–0.60        | ~$0.03–0.06         |
| Total      | **~$0.50–1.00**    | **~$0.05–0.10**     |

Wall clock is roughly 3× a single parallel render because of the
3-stage pipeline. The cross-artifact consistency is worth the latency.

---

## Architecture

Rust workspace, two crates plus the plugin:

- **`decomposer-core`** — engine, session, providers (Anthropic +
  OpenAI), render orchestration, prompt templates, manifest schema.
- **`decomposer-cli`** — `decomposer` binary. The interactive standalone
  flow plus two no-HTTP subcommands (`prompts`, `write-artifacts`) used
  by the plugin.
- **`plugin/decompose/`** — Claude Code plugin manifest and skill body.
  The plugin path routes LLM calls through the host conversation; the
  binary only handles prompt templates and disk I/O on that path.

See [PROJECT.md](PROJECT.md) for the full design rationale, the v1 vs
v2 protocol details, follow-ups, and known limitations.

---

## License

Dual-licensed under your choice of:

- [MIT License](LICENSE-MIT), or
- [Apache License 2.0](LICENSE-APACHE)

SPDX identifier: `MIT OR Apache-2.0`. Contributions are accepted under
the same terms.
