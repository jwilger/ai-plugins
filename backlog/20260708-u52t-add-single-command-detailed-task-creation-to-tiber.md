---
title: Create a complete Tiber ticket in one command
blocked_by: []
blocks: []
tags: [tiber, cli, mcp, developer-experience]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Allow the Tiber command line and integration server to create a fully detailed backlog ticket in one operation, including its description, context, acceptance criteria, notes, tags, and optional pull-request details. Invalid input must not leave a partly created ticket.

## Context / Why

Implementation notes: The current CLI/MCP create surface accepts only a title, and the new-task skill must then issue update, acceptance, and note calls. Define one detailed payload containing title, summary, context, repeated acceptance criteria, repeated notes, tags, and optional PR/MR fields. Invalid input must create no partial task, and a sync failure must identify the single created ref and a safe recovery path. Implement after the parser migration 20260707-rpmy so new flags are not parsed twice.

## Acceptance criteria

- [ ] A single CLI invocation can create a backlog task with title, summary, multiple acceptance criteria, and optional notes without requiring follow-up update commands.
- [ ] The Tiber MCP create surface supports the same detailed creation payload or exposes an equivalent single-call detailed create operation.
- [ ] The detailed create interface is scriptable and documented, with clear handling for repeated acceptance criteria and multi-line summary/note input.
- [ ] Existing simple task creation remains backward-compatible for current CLI and MCP callers.
- [ ] The detailed payload explicitly supports title, summary, context, repeated acceptance criteria, repeated notes, tags, and optional PR/MR URL and status fields.
- [ ] Payload validation is atomic: invalid input leaves no task or partial task commit, while sync failure reports the one created ref and safe recovery guidance.
- [ ] CLI repeated flags plus safe stdin/file input handle multiline values, and the MCP schema has equivalent typed fields and behavior.

## Subtasks

## Notes / Log

- 2026-07-14: Backlog grooming 2026-07-14: Removed the completed 20260707-rpmy parser-migration prerequisite. The detailed atomic-create task is now unblocked; all existing requirements remain unchanged.
- 2026-07-22: 2026-07-22 curation rejection: Real Tiber enhancement or edge case, but lower pain, severity, frequency, or leverage than closure correctness and non-destructive setup. The reserved product slot covers backlog-limit enforcement; this item is rejected with no hidden queue.
