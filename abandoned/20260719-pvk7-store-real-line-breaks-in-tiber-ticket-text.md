---
title: Store real line breaks in Tiber ticket text
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

Make Tiber preserve actual line breaks in ticket fields instead of displaying the two characters backslash and n.

## Context / Why

Ticket 20260708-puyh is a concrete example: text intended to span multiple lines contains literal \\n sequences. This makes tickets harder to read and causes formatting supplied through the CLI or MCP interface to be stored incorrectly. Implementation notes: trace argument parsing, field serialization, Markdown rendering, and round-trip updates so escaped input is interpreted exactly once without corrupting intentional backslashes.

## Acceptance criteria

- [ ] Multiline text entered through supported Tiber interfaces is stored and rendered with real line breaks.
- [ ] Existing literal backslash-n text can be distinguished from intentional backslash characters and repaired safely.
- [ ] Regression coverage includes the failure shape visible in ticket 20260708-puyh and a save-read-update round trip.

## Subtasks

## Notes / Log

- 2026-07-22: 2026-07-22 curation rejection: Real Tiber enhancement or edge case, but lower pain, severity, frequency, or leverage than closure correctness and non-destructive setup. The reserved product slot covers backlog-limit enforcement; this item is rejected with no hidden queue.
