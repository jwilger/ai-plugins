---
title: Prevent callers from weakening the GPT-5.6 evaluation sandbox
blocked_by: []
blocks: []
tags: [evals, codex, sandbox, isolation, provider, minor, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Ensure the GPT-5.6 evaluation wrapper rejects or removes caller settings that could override its read-only, network-disabled sandbox. The wrapper’s isolation policy must remain authoritative.

## Context / Why

Pre-existing/design MINOR found during 20260709-spx8 isolation review. Promptfoo app-server buildSandboxPolicy prefers config.sandbox_policy over sandbox_mode and sandbox_workspace_write; the wrapper currently retains that caller field while forcing the latter fields. Add a hostile-policy regression and fail closed or overwrite it in a focused follow-up.

## Acceptance criteria

## Subtasks

## Notes / Log
