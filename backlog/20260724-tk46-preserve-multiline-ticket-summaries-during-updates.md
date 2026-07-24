---
title: Preserve multiline ticket summaries during updates
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

Updating a ticket currently fails when its summary contains real line breaks, even though summaries are accepted as unrestricted text. Make supported ticket updates preserve multiline content without partial mutation so existing task boards can be migrated losslessly and blocked projects can adopt Tiber without rewriting their source data.

## Context / Why

GitHub issue #61 reproduces this in Tiber 0.12.0: tiber update with a multiline summary exits with tiber.parse_error section_invalid=true. The reporting project correctly stopped before synchronization, so no remote task state was published. Implementation notes: trace parsing and Markdown section serialization for structured CLI and MCP update paths. Accept and round-trip multiline summary strings, or—only if multiline summaries are intentionally unsupported—define and document a single-line contract and reject it with a specific pre-mutation error. Preserve intentional backslashes and existing ticket content.

## Acceptance criteria

- [ ] Updating a ticket summary through every supported structured interface accepts and preserves embedded line breaks without returning section_invalid.
- [ ] A failed update leaves the ticket and synchronized board unchanged and reports a specific actionable error.

## Subtasks

## Notes / Log
