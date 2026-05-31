Write an architecture document for the app described in the interview.

Required sections:

- High-level shape (one paragraph plus a small ASCII diagram if it helps)
- Components: each with one paragraph on responsibility and dependencies
- Data model: entities, key fields, relationships
- External surfaces: CLI, HTTP, files, integrations
- Key decisions and tradeoffs (3-5 decisions with the rejected alternative)
- Open questions

Output markdown only. Be opinionated where the interview was clear; flag
ambiguity in Open Questions rather than papering over it.

If a finalized PRD.md is provided in the user message, treat it as canonical:
use the same project/binary name it uses, do not contradict its scope or
non-goals, and do not introduce features it lists as out of scope.

When the transcript records the developer's explicit preference on a stack
decision (project shape, language, framework, persistence, deployment,
naming), treat that preference as a hard constraint — do not re-litigate it,
do not present alternatives in the doc body, and do not record it as a
rejected alternative. The developer already chose. Reflect the choice in
the relevant section (e.g. Components, External surfaces) and, only if the
choice is non-obvious, note the reason briefly in key-decisions.

You COMMIT decisions only on points the developer explicitly deferred to
you ("pick something sensible") or didn't raise at all. The interview's
readiness summary will tell you which is which — read it carefully.

Critically, this document is where ALL unresolved ambiguity gets pinned
down. Downstream artifacts (FILE_TREE, CLAUDE.md, TASKS) inherit your
decisions and must not re-open them, so you must leave nothing important
undecided. If the PRD (or the interview) leaves any choice open, COMMIT to
one concrete option here and justify it in the key-decisions section. This
includes — but is NOT limited to — choices about:

- Project shape: CLI tool, library, web service, daemon, desktop app,
  mobile app, embedded firmware, notebook, browser extension, ML
  pipeline, plugin to another system, etc.
- Language and runtime (e.g. Rust vs Python vs TypeScript).
- Frameworks, build systems, package managers, and toolchains.
- Persistence (which DB, ORM, file format) — or "no persistence".
- Interfaces exposed (CLI flags, HTTP routes, library API surface,
  message bus, GUI).
- Deployment / packaging target (single binary, Docker, native installer,
  cloud function, npm package).
- The concrete project/binary/crate/package name, if the PRD did not
  already commit to one.

Do not punt or hedge with "X or Y" phrasing, do not list parallel
alternatives in the doc body (rejected alternatives go in the
key-decisions section, with one chosen and the rest marked rejected), and
do not defer a choice to "we'll decide later" unless the open-question
section explicitly flags it as outside the scope of this architecture.
