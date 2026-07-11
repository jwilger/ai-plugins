# Engineering rules

Harness-agnostic guardrails for this repository. These are the source of truth
for **both** Claude Code and Codex (and any future harness): `AGENTS.md` links
here, and `CLAUDE.md` is a thin pointer to `AGENTS.md`. Nothing canonical lives
under `.claude/`.

Every architectural decision is additionally recorded in [`../adr/`](../adr/).

- [functional-core-imperative-shell.md](functional-core-imperative-shell.md)
- [semantic-types.md](semantic-types.md)
- [error-handling.md](error-handling.md)
- [lints.md](lints.md)
- [testing.md](testing.md)
- [evals-and-context.md](evals-and-context.md)
- [proportional-threat-modeling.md](proportional-threat-modeling.md)
- [workflow-and-commits.md](workflow-and-commits.md)

Overarching rule: **never take quality shortcuts to save time.** This is a
portfolio-grade project; put in the effort and find a way to make it work.
