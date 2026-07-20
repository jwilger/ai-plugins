---
title: Preserve existing repository setup when adding Tiber
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

## Context / Why

## Acceptance criteria

- [ ] Existing `.gitignore` content remains unchanged except for adding the missing Tiber task-directory rule once.
- [ ] Equivalent existing task-closing hooks or workflows are detected, and Tiber does not create duplicate automation.
- [ ] Dry-run output clearly distinguishes files that will change, files already configured, and conflicts that require an explicit choice.
- [ ] Apply mode refuses to overwrite ambiguous existing integration files without an explicit conflict-resolution choice.
- [ ] Automated regression coverage uses a repository with a populated `.gitignore` and existing task-closing automation, and proves repeated setup is safe and idempotent.

## Subtasks

## Notes / Log
