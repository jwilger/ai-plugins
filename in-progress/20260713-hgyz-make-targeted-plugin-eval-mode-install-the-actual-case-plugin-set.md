---
title: Make targeted plugin evals install exactly the selected case plugin set
blocked_by: []
blocks: []
tags: [evals, plugin-modes, measurement-validity, codex, claude, validation, plugin-loading, major, backlog]
pr_mr_url: 
pr_mr_status: 
claim:
  host: unknown
  session: unknown
---

## Summary

Derive targeted provider composition from selected behavior cases, reject explicit empty or unknown plugin selections before writes, and keep targeted and full-marketplace rows meaningfully distinct.

## Context / Why

scripts/evals/run.sh currently defaults targeted Codex composition to every marketplace plugin, while Claude targeted and full providers are compositionally identical. Separately, prepare-codex-home.mjs accepts an explicitly empty skills-only plugin list and can prepare a zero-plugin home. Treat these as one selection-semantics boundary: omitted selection may mean the documented full set, but an explicitly supplied targeted selection must be nonempty, known, derived from the selected cases, and installed before a row may be labeled targeted.

## Acceptance criteria

- [x] For each selected behavior suite, targeted mode derives a deterministic, deduplicated plugin set from the selected cases and installs exactly that set for every supported harness.
- [x] An explicitly supplied empty targeted or skills-only plugin selection, or any unknown plugin name, fails with a clear validation error before replacing or writing the eval home.
- [x] Omitting the plugin list where the documented contract means full marketplace retains that behavior; omission is not conflated with an explicitly empty selection.
- [x] Generated config and dashboard regressions prove targeted and full-marketplace rows differ compositionally when selected cases use a proper subset. If Claude cannot express an honest targeted composition, remove the duplicate targeted label rather than misrepresenting it.
- [x] Generated behavior-eval cases preserve list-valued metadata such as plugins, skills, and coverage_kinds as atomic metadata rather than Promptfoo variable-sweep dimensions; a one-fixture, one-sample, one-provider run produces exactly one target request, with a regression test.

## Subtasks

## Notes / Log

- 2026-07-14: Backlog grooming 2026-07-14: Consolidated 20260713-jtq4 because fail-closed explicit selection is part of the same plugin-selection contract required to make targeted composition trustworthy.
- 2026-07-15: 2026-07-15: A focused Codex live run of tiber-new-task-command-backlog-capture configured one fixture, one sample, and one provider but Promptfoo executed 10 target requests. The loader returned one case; Promptfoo expanded plugins (1) × skills (2) × coverage_kinds (5) into a Cartesian sweep, collapsing those arrays to scalar result vars. This invalidates current sample accounting and inflated an 11-fixture run to 73 reported cases. Treat preservation of list metadata as part of targeted-composition measurement validity.
- 2026-07-16: Completion evidence at 351cf031939d460e88b4d6f3e37f297a6bfc01df: exact targeted/full provider-composition validation, fail-closed dashboard provenance, list-metadata atomicity, explicit empty/unknown selection rejection before writes, and Codex-home overlap/alias safety are implemented and pushed directly to main. Focused eval-runner regressions passed 84/84; fresh just ci passed all repository gates including 401 Bats and development-discipline tests/mutation checks; formal final review session final-review-886ae7c96db43dc2 had no unresolved blockers. GitHub CI run 29468495391 is green for the exact commit. Provider-backed behavior eval eval-u1J-2026-07-16T05:29:12 executed the intended 276 cases exactly once across six provider labels with 0 runtime errors and recorded exact compositions: 114 passed, 162 failed (41.3%); targeted scored 22/46 for both harnesses versus full Claude 21/46 and full Codex 20/46. Those quality thresholds remain red and are not represented as passing. Diagnosis found both broader skill-quality debt and hard-guard false positives; the detector defect is split to 20260716-ya64, while writable downstream quality measurement is prioritized in 20260715-n6bs. Full-marketplace canary eval-pxD-2026-07-16T06:55:21 passed 2/2 with 0 errors. The static dashboard rebuilt successfully from the restored behavior artifact. Promptfoo share was attempted but unavailable because this machine's CLI has no cloud login; local repo-owned artifacts were produced.
