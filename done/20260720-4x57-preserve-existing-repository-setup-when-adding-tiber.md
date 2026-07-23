---
title: Preserve existing repository setup when adding Tiber
blocked_by: []
blocks: []
tags: [tiber, scaffold, safety, eval-case]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Adding Tiber to an existing repository can currently erase unrelated ignore rules and create a second copy of task-closing automation. Change repository setup so it preserves what is already there, adds only missing Tiber integration, and clearly reports conflicts before making changes. This prevents setup from damaging project configuration or creating competing automation.

## Context / Why

Covers GitHub issues #52 and #53. Issue #53 is also a reusable evaluation case, meaning a saved scenario used to test future agent behavior. Implementation notes: Update tiber scaffold repo so ignore-file changes are additive and safe to repeat. Detect equivalent existing hooks and GitHub Actions workflows rather than relying only on generated filenames. The dry run must describe additions, no-ops, and conflicts accurately, and apply mode must not replace ambiguous existing files without an explicit conflict-resolution choice.

## Acceptance criteria

- [x] Existing `.gitignore` content remains unchanged except for adding the missing Tiber task-directory rule once.
- [x] Equivalent existing task-closing hooks or workflows are detected, and Tiber does not create duplicate automation.
- [x] Dry-run output clearly distinguishes files that will change, files already configured, and conflicts that require an explicit choice.
- [x] Apply mode refuses to overwrite ambiguous existing integration files without an explicit conflict-resolution choice.
- [x] Automated regression coverage uses a repository with a populated `.gitignore` and existing task-closing automation, and proves repeated setup is safe and idempotent.
- [x] A behavior fixture based on GitHub issue #53 rejects destructive or duplicate repository setup.

## Subtasks

## Notes / Log
