---
name: decompose
description: |
  Turn a vague app idea into a coherent starting brief — PRD, Architecture, File Tree, CLAUDE.md, and Tasks markdown — via a focused interview. Use when the user is starting a new project, scaffolding from an idea, asks to decompose or plan a project, or explicitly invokes /decompose.
---

# project-decomposer

Run a short interview, then render five canonical artifacts (PRD.md,
ARCHITECTURE.md, FILE_TREE.md, CLAUDE.md, TASKS.md) plus `manifest.json` into
`./decomposed/{slug}/`. The Rust binary `decomposer` owns the prompt templates
and artifact writing; **you** (this conversation) own the LLM work. No
separate API key is required.

## Preflight

1. `which decomposer` — must be on PATH. If missing, ask the user to install:
   `cargo install --path crates/decomposer-cli` from the project-decomposer
   repo.
2. Confirm the current working directory is where `./decomposed/{slug}/`
   should land. If the directory already contains a project, ask before
   proceeding.
3. Get the user's one-line app idea. If they haven't given one, ask for it
   in a single sentence.

## Phase 1 — Interview

1. Run `decomposer prompts interviewer` via the Bash tool and treat the
   output as your instructions for how to interview. It enumerates the
   nine categories (`problem`, `users`, `scope`, `non_goals`, `data_model`,
   `interfaces`, `stack`, `constraints`, `risks`) and explains the
   anti-drift discipline.
2. Ask **one question at a time** in the conversation. Wait for the user's
   answer before the next question. Keep a running list of
   `{category, question, answer}` triples — you will serialize it later.
   If the user revises a previous answer mid-interview, update the entry
   in place rather than appending.
3. The `stack` category is load-bearing — it's how the brief avoids drift
   when features get added later. For each major stack decision (project
   shape, language, framework, persistence, deployment, naming), get one
   of four answer shapes: **committed** ("must be Rust"), **deferred**
   ("pick something sensible"), **conditional** ("Rust if it fits, else
   whatever"), or **N/A with reason** ("no framework — it's a single-file
   script"). Silent absence is a bug.
4. **Before stopping, walk the commitment checklist deliberately** — don't
   assume any item is irrelevant just because it didn't come up:
   - For each of the nine categories, you should be able to point to
     either: (a) a transcript entry, (b) the user's original one-line idea
     if it already covers the category (e.g. "problem: covered by the
     idea — chickens turn into flaming missiles when disturbed"), or
     (c) an explicit N/A with reason ("data_model N/A — stateless CLI
     over git"). Record the resolution in the readiness summary.
   - For each major stack decision, confirm it's committed / deferred /
     conditional / N/A-with-reason. Don't assume "framework isn't needed"
     without the user actually saying so.
   - If you find a gap during this review, ask one more question.
5. Aim for 8–12 questions; the hard cap is 15. Stop when the checklist
   above passes. Produce a **3–5 sentence readiness summary** that
   explicitly lists user-committed stack decisions, decisions the architect
   must make on the user's behalf, and any N/A items with reason. **Save
   this summary in working memory verbatim — it will be passed to
   `write-artifacts --summary` in Phase 3 and the architect prompt is
   instructed to read it.**
6. **Identify the project name.** If the user committed a concrete name
   during the `stack` interview (e.g. "diffrep"), remember it for Phase 3.
   If they deferred naming, the ARCHITECTURE render in Phase 2 will commit
   one — pull it from there. The committed name (not the user's vague
   one-line idea) drives the output directory.

## Phase 2 — Render (3 stages, strict order)

The 3-stage render is load-bearing for cross-artifact consistency — do not
collapse to a single parallel batch. Each stage's prompt comes from
`decomposer prompts <kind>`; treat it as the rendering instruction.

**Stage 1.** `decomposer prompts prd` → render the PRD body from the
transcript alone. The body is plain markdown — no outer code fences.

**Stage 2.** `decomposer prompts architecture` → render the ARCHITECTURE
body, using the PRD body from Stage 1 as prior context. The architecture
prompt forbids "X or Y" hedging in the doc body; commit to choices and put
rejected alternatives in the key-decisions section.

**Stage 3 (parallel).** Render `file-tree`, `claude-md`, and `tasks`. For
each, fetch its prompt with `decomposer prompts <kind>`, then render the
body using **both** the PRD and ARCHITECTURE bodies as prior context. Each
prompt explicitly says to "honor every concrete decision ARCHITECTURE.md
committed to" — follow that. These three are independent; produce them in
parallel where possible.

## Phase 3 — Write artifacts

Build two JSON inputs in temp files (e.g. under `/tmp/`):

- `transcript.json` — a JSON array of exchanges:
  ```json
  [
    {"category": "problem", "question": "...", "answer": "..."},
    {"category": "users",   "question": "...", "answer": "..."}
  ]
  ```
  Categories must be snake_case from the set above (`problem`, `users`,
  `scope`, `non_goals`, `data_model`, `interfaces`, `stack`, `constraints`,
  `risks`).

- `bodies.json` — a JSON object mapping kind to the rendered markdown body:
  ```json
  {
    "prd":          "...",
    "architecture": "...",
    "file_tree":    "...",
    "claude_md":    "...",
    "tasks":        "..."
  }
  ```

Then run:

```
decomposer write-artifacts \
  --idea "<the user's one-line idea>" \
  --name "<the committed project name>" \
  --summary "<the readiness summary from Phase 1>" \
  --transcript /tmp/transcript.json \
  --bodies /tmp/bodies.json
```

- `--idea` is the user's original one-line description (preserved in the
  manifest for traceability).
- `--name` is the **committed project name** identified in Phase 1 step 6.
  When set, it drives the slug and output directory (e.g. `--name diffrep`
  → `./decomposed/diffrep/`). Omit only if neither the user nor the
  architect committed a name.
- `--summary` is the verbatim 3–5 sentence readiness summary from Phase 1
  step 5. Don't paraphrase — it's stored in the manifest and re-used if
  the session is later resumed for re-rendering.

The binary writes the five markdown files plus `manifest.json` under
`./decomposed/{slug}/` and prints a single JSON line:
`{"type":"done","manifest_path":"..."}`.

Show the user the manifest path and the list of files written. Offer to
read the `CLAUDE.md` so the rest of the conversation can pick up from the
brief.

## Notes

- Subcommands `prompts` and `write-artifacts` make no HTTP calls and do not
  read `ANTHROPIC_API_KEY` — they are pure local operations. Only the
  standalone interactive flow (`decomposer <idea>`) hits the Messages API.
- If the user wants to revise after seeing the output, the simplest path is
  to re-run from scratch — there is no in-place editing flow in v1.
- For very long interviews where context bloat is a concern, Stage 2 and
  Stage 3 renders can be delegated to subagents; pass the prompt + prior
  artifact bodies into the subagent and have it return the rendered body.
