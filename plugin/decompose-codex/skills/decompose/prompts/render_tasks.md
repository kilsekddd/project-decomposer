Produce an ordered, checkbox-style build plan for the app.

Format: a single markdown checklist under an H1 "Build plan". Group tasks
under H2 milestones (e.g. "Milestone 1: skeleton", "Milestone 2: ...").

Each task:

- Starts with an imperative verb
- Is small enough to complete in one focused session
- Notes the file(s) it touches if known from the FILE_TREE

Order tasks so that early items unblock later ones. Output markdown only.

If finalized PRD.md and ARCHITECTURE.md are provided in the user message,
treat them as canonical:
- Use the same project/binary name they use.
- Honor every concrete decision ARCHITECTURE.md committed to — project
  shape, language, framework, toolchain, persistence, interfaces,
  deployment target, naming. File paths in task annotations must use
  concrete extensions and directories matching those choices (never `.*`
  or "X or Y" or "choose between..." phrasing). Do NOT schedule tasks
  whose purpose is to re-decide something the architecture already
  decided (e.g. "choose language", "pick a database", "decide on a
  framework"). Those decisions are done.
- Do not schedule work for features the PRD lists as non-goals or "not
  committed". Such items belong in a brief "Out of scope (not scheduled)"
  note at the end, not as planned milestones.
