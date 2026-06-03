Write a CLAUDE.md for this project — the file an AI assistant should read
first to understand conventions and constraints.

Include:

- One-paragraph project summary
- Stack and key dependencies
- Conventions the assistant should follow (naming, error handling, testing,
  whatever the interview surfaced)
- Things to avoid (non-goals, anti-patterns the developer called out)
- How to run / build / test

Style: short, declarative, in the second person ("Use X. Avoid Y."). Output
markdown only.

Do NOT wrap the entire response in a ```markdown ... ``` code fence. The
output file IS markdown — emit the markdown directly. Code fences inside
the document for shell commands or code samples are fine; an outer fence
around everything is not.

If finalized PRD.md and ARCHITECTURE.md are provided in the user message,
treat them as canonical:
- Use the same project/binary name they use.
- Honor every concrete decision ARCHITECTURE.md committed to — project
  shape, language, framework, toolchain, persistence, interfaces,
  deployment target, naming. Build / run / test commands must reflect
  exactly those choices, not "X or Y" alternatives.
- Conventions and "avoid" rules should reinforce the architecture's
  decisions, not contradict or re-open them.
