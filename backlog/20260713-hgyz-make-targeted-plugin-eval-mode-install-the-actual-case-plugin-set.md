---
title: Make targeted plugin evals install exactly the selected case plugin set
blocked_by: []
blocks: []
tags: [evals, plugin-modes, measurement-validity, codex, claude, validation, plugin-loading, major, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Derive targeted provider composition from selected behavior cases, reject explicit empty or unknown plugin selections before writes, and keep targeted and full-marketplace rows meaningfully distinct.

## Context / Why

scripts/evals/run.sh currently defaults targeted Codex composition to every marketplace plugin, while Claude targeted and full providers are compositionally identical. Separately, prepare-codex-home.mjs accepts an explicitly empty skills-only plugin list and can prepare a zero-plugin home. Treat these as one selection-semantics boundary: omitted selection may mean the documented full set, but an explicitly supplied targeted selection must be nonempty, known, derived from the selected cases, and installed before a row may be labeled targeted.

## Acceptance criteria

- [ ] For each selected behavior suite, targeted mode derives a deterministic, deduplicated plugin set from the selected cases and installs exactly that set for every supported harness.
- [ ] An explicitly supplied empty targeted or skills-only plugin selection, or any unknown plugin name, fails with a clear validation error before replacing or writing the eval home.
- [ ] Omitting the plugin list where the documented contract means full marketplace retains that behavior; omission is not conflated with an explicitly empty selection.
- [ ] Generated config and dashboard regressions prove targeted and full-marketplace rows differ compositionally when selected cases use a proper subset. If Claude cannot express an honest targeted composition, remove the duplicate targeted label rather than misrepresenting it.

## Subtasks

## Notes / Log

- 2026-07-14: Backlog grooming 2026-07-14: Consolidated 20260713-jtq4 because fail-closed explicit selection is part of the same plugin-selection contract required to make targeted composition trustworthy.
