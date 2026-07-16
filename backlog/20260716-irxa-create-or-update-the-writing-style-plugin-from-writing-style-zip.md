---
title: Create or update the writing-style plugin from writing-style.zip
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

Use the supplied writing-style.zip archive to create a writing-style plugin, or update the existing plugin if the repository already contains one.

## Context / Why

Source material is now at the main checkout's repository root as writing-style.zip (moved there by the user from ~/Downloads/writing-style.zip). Treat Codex personal use as the primary target, Claude Code as the secondary target, and human-facing reuse as tertiary. Inspect the archive before deciding whether to add a new plugin or reconcile it with an existing marketplace plugin.

## Acceptance criteria

- [ ] The archive is inspected safely and its useful writing-style guidance is incorporated into the appropriate new or existing plugin without importing irrelevant or unsafe artifacts.
- [ ] The resulting skill is concise, executable by Codex, and gives clear guidance for producing and revising prose while preserving user intent.
- [ ] Codex marketplace metadata is complete; Claude Code metadata is added or updated when the skill is compatible with that harness.
- [ ] Plugin manifests, versions, marketplace entries, catalog documentation, and plugin README stay synchronized with repository conventions.
- [ ] Relevant deterministic validation and behavior evals pass, with evidence recorded before the task is closed.

## Subtasks

## Notes / Log
