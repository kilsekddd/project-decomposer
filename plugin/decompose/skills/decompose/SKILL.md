---
name: decompose
description: |
  Turn a vague app idea into a coherent starting brief — PRD, Architecture, File Tree, CLAUDE.md, and Tasks markdown — via a focused interview. Use when the user is starting a new project, scaffolding from an idea, asks to decompose or plan a project, or explicitly invokes /decompose.
---

# project-decomposer

Run a short interview, then render five canonical artifacts (PRD.md,
ARCHITECTURE.md, FILE_TREE.md, CLAUDE.md, TASKS.md) plus `manifest.json` into
`./decomposed/{slug}/`. **You** (this conversation) do all the work: the
prompt templates ship as files inside this skill, and you read them, run the
interview, render the bodies, and write the files yourself. No external
binary and no separate API key are required.

The prompt files live in the `prompts/` directory next to this SKILL.md.
Read them with the Read tool; treat each file's contents as your
instructions for that step, not as text to echo to the user.

## Preflight

1. Confirm the current working directory is where `./decomposed/{slug}/`
   should land. If the directory already contains a project, ask before
   proceeding.
2. Get the user's one-line app idea. If they haven't given one, ask for it
   in a single sentence.

## Phase 1 — Interview

1. Read `prompts/interviewer.md` (in this skill's directory) and treat its
   contents as your instructions for how to interview. It enumerates the
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
     over git"). These are the **only three** valid resolutions —
     "the architect will address this" is **not** valid at the category
     level. If a category (e.g. `risks`) is unresolved, ask one more
     question or get the user to mark it N/A with reason. Record the
     resolution in the readiness summary.
   - For each major stack decision, confirm it's committed / deferred /
     conditional / N/A-with-reason. Don't assume "framework isn't needed"
     without the user actually saying so.
   - If you find a gap during this review, ask one more question.
5. Aim for 8–12 questions; the hard cap is 15. Stop when the checklist
   above passes. Produce a **3–5 sentence readiness summary** that
   explicitly lists user-committed stack decisions, decisions the architect
   must make on the user's behalf, and any N/A items with reason. **Keep
   this summary verbatim — it goes into `manifest.json` (`session.summary`)
   and the architect prompt in Phase 2 is instructed to read it.**
6. **Identify the project name.** If the user committed a concrete name
   during the `stack` interview (e.g. "diffrep"), remember it for the slug.
   If they deferred naming, the ARCHITECTURE render in Phase 2 will commit
   one — pull it from there. The committed name (not the user's vague
   one-line idea) drives the output directory.

## Phase 2 — Render (3 stages, strict order)

The 3-stage render is load-bearing for cross-artifact consistency — do not
collapse to a single parallel batch. Each stage's prompt is a file in
`prompts/`; read it and treat it as the rendering instruction. Produce each
body as **plain markdown with no outer code fences** (don't wrap the whole
document in ```` ```markdown ... ``` ````).

**Stage 1 — PRD.** Read `prompts/render_prd.md`. Render the PRD body from
the transcript alone.

**Stage 2 — ARCHITECTURE.** Read `prompts/render_architecture.md`. Render
the ARCHITECTURE body using the PRD body from Stage 1 as prior context, plus
the readiness summary from Phase 1. The architecture prompt forbids "X or Y"
hedging in the doc body; commit to choices and put rejected alternatives in
the key-decisions section.

**Stage 3 (parallel) — FILE_TREE, CLAUDE.md, TASKS.** Read
`prompts/render_file_tree.md`, `prompts/render_claude_md.md`, and
`prompts/render_tasks.md`. Render each body using **both** the PRD and
ARCHITECTURE bodies as prior context. Each prompt explicitly says to "honor
every concrete decision ARCHITECTURE.md committed to" — follow that. These
three are independent; render them in parallel where possible (a subagent
per body is a good fit for long interviews where context bloat is a concern
— pass it the prompt + the PRD/ARCHITECTURE bodies and have it return the
rendered body).

## Phase 3 — Write artifacts

Compute the **slug** from the committed project name (Phase 1 step 6) by
lowercasing it, replacing every run of non-alphanumeric characters with a
single hyphen, and trimming leading/trailing hyphens (e.g. `diffrep` →
`diffrep`, `Stripe Webhook Relay` → `stripe-webhook-relay`). If no name was
ever committed, fall back to slugifying the user's one-line idea the same
way. The output directory is `./decomposed/{slug}/`.

Write the five artifact files, each as raw markdown (no outer fences), with
the Write tool:

- `./decomposed/{slug}/PRD.md`
- `./decomposed/{slug}/ARCHITECTURE.md`
- `./decomposed/{slug}/FILE_TREE.md`
- `./decomposed/{slug}/CLAUDE.md`
- `./decomposed/{slug}/TASKS.md`

Then write `./decomposed/{slug}/manifest.json` with exactly this shape (it is
the on-disk contract — keep the field names, snake_case category/kind values,
and `version: 1`):

```json
{
  "version": 1,
  "slug": "<slug>",
  "idea": "<the user's original one-line idea>",
  "provider": "claude-code",
  "model": "claude-code",
  "created_at": "<current UTC time, RFC 3339, e.g. 2026-05-31T17:45:00Z>",
  "session": {
    "idea": "<same one-line idea>",
    "slug": "<slug>",
    "budget": { "min": 6, "max": 15 },
    "phase": "done",
    "transcript": [
      { "category": "problem", "question": "...", "answer": "..." },
      { "category": "stack",   "question": "...", "answer": "..." }
    ],
    "summary": "<the verbatim readiness summary from Phase 1 step 5>"
  },
  "artifacts": [
    { "kind": "prd",          "path": "decomposed/<slug>/PRD.md" },
    { "kind": "architecture", "path": "decomposed/<slug>/ARCHITECTURE.md" },
    { "kind": "file_tree",    "path": "decomposed/<slug>/FILE_TREE.md" },
    { "kind": "claude_md",    "path": "decomposed/<slug>/CLAUDE.md" },
    { "kind": "tasks",        "path": "decomposed/<slug>/TASKS.md" }
  ]
}
```

Category values in `transcript` must be snake_case from the set: `problem`,
`users`, `scope`, `non_goals`, `data_model`, `interfaces`, `stack`,
`constraints`, `risks`. The `artifacts[].path` values are relative to the
current working directory (matching what the standalone CLI writes).

Show the user the manifest path and the list of files written. Offer to
read the `CLAUDE.md` so the rest of the conversation can pick up from the
brief.

## Notes

- This skill is fully self-contained: it reads the bundled prompt files and
  writes the artifacts itself. There is no `decomposer` binary on the plugin
  path and no `ANTHROPIC_API_KEY` requirement. (A separate standalone Rust
  CLI exists in the same repo for BYO-API-key / non-Claude-Code use; it is
  not involved here and shares only the prompt text.)
- If the user wants to revise after seeing the output, the simplest path is
  to re-run from scratch — there is no in-place editing flow.
- The `manifest.json` exists for traceability and so a future tool could
  resume or re-render from a completed session; keep its shape stable.
