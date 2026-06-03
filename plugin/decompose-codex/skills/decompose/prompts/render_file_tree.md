Propose a directory and file layout for the app described in the interview.

Use the language/framework the developer named. If none was specified, choose
the most reasonable default for the kind of app described and say so.

Format:

- A fenced code block with a `tree`-style listing
- Below it, a bulleted list mapping each non-trivial path to a one-line
  description of its responsibility

Keep the tree to the level of detail that helps planning, not exhaustive
boilerplate (don't list every test file). Output markdown only.

If finalized PRD.md and ARCHITECTURE.md are provided in the user message,
treat them as canonical:
- Use the same project/binary name they use; the root directory name should
  match.
- Honor every concrete decision ARCHITECTURE.md committed to — project
  shape, language, framework, toolchain, persistence, interfaces,
  deployment target, naming. Do not re-open any of those choices, do not
  offer alternatives, and do not invent a tree that fits a different
  project type than the architecture describes.
- Use the same module/component boundaries ARCHITECTURE.md names.
