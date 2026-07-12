---
title: Add single-command detailed task creation to Tiber
blocked_by: [20260707-rpmy-use-a-real-cli-argument-parser-for-tiber-commands-and-help]
blocks: []
tags: [tiber, cli, mcp, developer-experience]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Add an atomic one-invocation CLI and one-call MCP workflow for creating a fully specified Tiber backlog task without sequential follow-up mutations.

## Context / Why

The current CLI/MCP create surface accepts only a title, and the new-task skill must then issue update, acceptance, and note calls. Define one detailed payload containing title, summary, context, repeated acceptance criteria, repeated notes, tags, and optional PR/MR fields. Invalid input must create no partial task, and a sync failure must identify the single created ref and a safe recovery path. Implement after the parser migration 20260707-rpmy so new flags are not parsed twice.

## Acceptance criteria

- [ ] A single CLI invocation can create a backlog task with title, summary, multiple acceptance criteria, and optional notes without requiring follow-up update commands.
- [ ] The Tiber MCP create surface supports the same detailed creation payload or exposes an equivalent single-call detailed create operation.
- [ ] The detailed create interface is scriptable and documented, with clear handling for repeated acceptance criteria and multi-line summary/note input.
- [ ] Existing simple task creation remains backward-compatible for current CLI and MCP callers.
- [ ] The detailed payload explicitly supports title, summary, context, repeated acceptance criteria, repeated notes, tags, and optional PR/MR URL and status fields.
- [ ] Payload validation is atomic: invalid input leaves no task or partial task commit, while sync failure reports the one created ref and safe recovery guidance.

## Subtasks

## Notes / Log
