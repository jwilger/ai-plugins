---
title: Identify all parts of a change before implementation starts
blocked_by: []
blocks: [20260708-puyh-add-development-workflow-router-skill, 20260710-jx7i-mine-session-history-for-reusable-agent-guardrails]
tags: [development-discipline, preflight, workflow, evals]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Add a preflight check that identifies which parts of a project a requested change may affect, such as behavior, tests, documentation, configuration, packaging, releases, migrations, and operations. This should catch important follow-up work early instead of during final review.

## Context / Why

Implementation notes:\n\nObvious documentation, test, schema, configuration, packaging, release, migration, startup, eval, and workflow obligations are currently discovered late during final review. The preflight should produce a concise evidence-backed classification, not a speculative implementation plan, and must honor repository-local instructions over reusable defaults. It applies proportionally: substantive changes get the full surface check, while genuinely narrow documentation or metadata work may mark irrelevant surfaces not applicable with a reason.

## Acceptance criteria

- [ ] development-discipline provides a preflight skill or equivalent guidance that classifies change type and lists affected surfaces before implementation begins.
- [ ] Guidance covers at least behavior, tests, docs, config, packaging, release artifacts, migrations, operational startup, evals, and user workflows as possible affected surfaces.
- [ ] The change includes behavior eval cases or equivalent acceptance fixtures proving agents perform the preflight before editing in representative repositories.
- [ ] Before edits, the workflow emits a concise change classification, applicable affected surfaces, supporting repository evidence, and reasoned not-applicable entries.
- [ ] Trigger and skip rules are explicit, repository-local instructions take precedence, and fixtures cover feature, bugfix, refactor, documentation/configuration, packaging/release, migration, and operational changes.

## Subtasks

## Notes / Log
