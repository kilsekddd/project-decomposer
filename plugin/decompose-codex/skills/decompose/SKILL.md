---
name: decompose
description: |
  Turn a vague app idea into a coherent starting brief for Codex: PRD, Architecture, File Tree, CLAUDE.md, AGENTS.md, and Tasks markdown via a focused interview. Use when the user is starting a new project, scaffolding from an idea, asks to decompose or plan a project, or explicitly asks for project-decomposer.
---

# project-decomposer for Codex

Run a short interview, then render six canonical artifacts into
`./decomposed/{slug}/`: `PRD.md`, `ARCHITECTURE.md`, `FILE_TREE.md`,
`CLAUDE.md`, `AGENTS.md`, `TASKS.md`, plus `manifest.json`. After writing, create an
`AGENTS.md` copy of `CLAUDE.md` in the same output directory so Codex can use
the assistant guidance natively.

You (this Codex conversation) do all the work: the prompt templates ship as
files inside this skill, and you read them, run the interview, render the
bodies, and write the files yourself. No external binary and no separate API
key are required.

The prompt files live in the `prompts/` directory next to this SKILL.md. Read
them with the Read tool; treat each file's contents as your instructions for
that step, not as text to echo to the user.

## Preflight

1. Confirm the current working directory is where `./decomposed/{slug}/`
   should land. If the directory already contains a project, ask before
   proceeding.
2. Get the user's one-line app idea. If they have not given one, ask for it
   in a single sentence.

## Phase 1 - Interview

1. Read `prompts/interviewer.md` (in this skill's directory) and treat its
   contents as your instructions for how to interview. It enumerates the nine
   categories (`problem`, `users`, `scope`, `non_goals`, `data_model`,
   `interfaces`, `stack`, `constraints`, `risks`) and explains the anti-drift
   discipline.
2. Ask one question at a time in the conversation. Wait for the user's answer
   before the next question. Keep a running list of `{category, question,
   answer}` triples; you will serialize it later. If the user revises a
   previous answer mid-interview, update the entry in place rather than
   appending.
3. The `stack` category is load-bearing. For each major stack decision
   (project shape, language, framework, persistence, deployment, naming), get
   one of four answer shapes: committed, deferred, conditional, or N/A with
   reason. Silent absence is a bug.
4. Before stopping, walk the commitment checklist deliberately:
   - For each of the nine categories, point to either a transcript entry, the
     user's original one-line idea if it already covers the category, or an
     explicit N/A with reason. "The architect will address this" is not valid
     at the category level.
   - For each major stack decision, confirm it is committed, deferred,
     conditional, or N/A with reason.
   - If you find a gap, ask one more question.
5. Aim for 8-12 questions; the hard cap is 15. Stop when the checklist passes.
   Produce a 3-5 sentence readiness summary that explicitly lists
   user-committed stack decisions, decisions the architect must make on the
   user's behalf, and any N/A items with reason. Keep this summary verbatim;
   it goes into `manifest.json` (`session.summary`) and the architect prompt
   in Phase 2 is instructed to read it.
6. Identify the project name. If the user committed a concrete name during
   the interview, remember it for Phase 3. If they deferred naming, the
   ARCHITECTURE render in Phase 2 will commit one; pull it from there.

## Phase 2 - Render

The three-stage render is load-bearing for cross-artifact consistency. Do not
collapse it into one parallel batch. Each stage's prompt is a file in
`prompts/`; read it and treat it as the rendering instruction. Produce each
body as plain markdown with no outer code fences.

Stage 1: Read `prompts/render_prd.md`, then render the PRD body from the
transcript alone.

Stage 2: Read `prompts/render_architecture.md`, then render the ARCHITECTURE
body using the PRD body as prior context, plus the readiness summary from
Phase 1. The architecture prompt forbids "X or Y" hedging in the doc body;
commit to choices and put rejected alternatives in the key-decisions section.

Stage 3: Read `prompts/render_file_tree.md`,
`prompts/render_claude_md.md`, and `prompts/render_tasks.md`. Render each
body using both the PRD and ARCHITECTURE bodies as prior context. These three
are independent and may be produced in parallel.

When rendering `claude-md`, keep the canonical content compatible with the
upstream `CLAUDE.md` prompt, but write it as durable assistant guidance that
Codex can also follow once copied to `AGENTS.md`.

## Phase 3 - Write artifacts

Compute the slug from the committed project name by lowercasing it, replacing
every run of non-alphanumeric characters with a single hyphen, and trimming
leading/trailing hyphens. If no name was ever committed, fall back to
slugifying the user's one-line idea the same way. The output directory is
`./decomposed/{slug}/`.

Write the artifact files as raw markdown with the Write or apply_patch tool:

- `./decomposed/{slug}/PRD.md`
- `./decomposed/{slug}/ARCHITECTURE.md`
- `./decomposed/{slug}/FILE_TREE.md`
- `./decomposed/{slug}/CLAUDE.md`
- `./decomposed/{slug}/AGENTS.md` (copy the exact `CLAUDE.md` body)
- `./decomposed/{slug}/TASKS.md`

Then write `./decomposed/{slug}/manifest.json` with exactly this shape. Keep
the field names, snake_case category/kind values, and `version: 2`:

```json
{
  "version": 2,
  "slug": "<slug>",
  "idea": "<the user's original one-line idea>",
  "provider": "codex",
  "model": "codex",
  "created_at": "<current UTC time, RFC 3339, e.g. 2026-05-31T17:45:00Z>",
  "session": {
    "idea": "<same one-line idea>",
    "slug": "<slug>",
    "budget": { "min": 6, "max": 15 },
    "phase": "done",
    "transcript": [
      { "category": "problem", "question": "...", "answer": "..." },
      { "category": "stack", "question": "...", "answer": "..." }
    ],
    "summary": "<the verbatim readiness summary from Phase 1 step 5>"
  },
  "artifacts": [
    { "kind": "prd", "path": "decomposed/<slug>/PRD.md" },
    { "kind": "architecture", "path": "decomposed/<slug>/ARCHITECTURE.md" },
    { "kind": "file_tree", "path": "decomposed/<slug>/FILE_TREE.md" },
    { "kind": "claude_md", "path": "decomposed/<slug>/CLAUDE.md" },
    { "kind": "agents_md", "path": "decomposed/<slug>/AGENTS.md" },
    { "kind": "tasks", "path": "decomposed/<slug>/TASKS.md" }
  ]
}
```

Category values in `transcript` must be snake_case from the set: `problem`,
`users`, `scope`, `non_goals`, `data_model`, `interfaces`, `stack`,
`constraints`, `risks`. The `artifacts[].path` values are relative to the
current working directory and intentionally match the shared artifact contract.

Show the user the manifest path and the list of files written, including
`AGENTS.md`. Offer to read `AGENTS.md` so the rest of the Codex conversation
can pick up from the brief.

## Notes

- This skill is fully self-contained: it reads the bundled prompt files and
  writes the artifacts itself. There is no `decomposer` binary on the plugin
  path and no provider API-key requirement. A separate standalone Rust CLI
  exists in the same repo for BYO-API-key / non-Codex use; it is not involved
  here and shares only the prompt text.
- If the user wants to revise after seeing the output, the simplest v1 path is
  to re-run from scratch.
- For very long interviews where context bloat is a concern, Stage 2 and
  Stage 3 renders can be delegated to subagents. Pass the prompt plus prior
  artifact bodies into the subagent and have it return only the rendered body.
