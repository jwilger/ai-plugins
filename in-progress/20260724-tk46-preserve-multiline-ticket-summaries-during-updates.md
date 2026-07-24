---
title: Preserve multiline ticket summaries during updates
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
claim:
  host: unknown
  session: unknown
---

## Summary

Updating a ticket currently fails when its summary contains real line breaks, even though summaries are accepted as unrestricted text. Make supported ticket updates preserve multiline content without partial mutation so existing task boards can be migrated losslessly and blocked projects can adopt Tiber without rewriting their source data.

## Context / Why

GitHub issue #61 reproduces this in Tiber 0.12.0: tiber update with a multiline summary exits with tiber.parse_error section_invalid=true. The reporting project correctly stopped before synchronization, so no remote task state was published. Implementation notes: trace parsing and Markdown section serialization for structured CLI and MCP update paths. Accept and round-trip multiline summary strings, or—only if multiline summaries are intentionally unsupported—define and document a single-line contract and reject it with a specific pre-mutation error. Preserve intentional backslashes and existing ticket content.

## Acceptance criteria

- [ ] Updating a ticket summary through every supported structured interface accepts and preserves embedded line breaks without returning section_invalid.
- [ ] A failed update leaves the ticket and synchronized board unchanged and reports a specific actionable error.
- [ ] Regression tests cover multiline CLI and MCP updates, serialization and read-back, intentional backslashes, and the exact migration-blocking reproduction from GitHub issue #61.

## Subtasks

## Notes / Log

- 2026-07-24: Admitted from GitHub issue #61 as the highest-priority queued item because it blocks a lossless migration in another project. It is related to abandoned candidate 20260719-pvk7 but distinct: that candidate concerned literal backslash-n rendering, while this ticket covers a supported update operation rejecting actual multiline values.
