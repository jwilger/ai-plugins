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

## Acceptance criteria

- [ ] Existing `.gitignore` content remains unchanged except for adding the missing Tiber task-directory rule once.
- [ ] Equivalent existing task-closing hooks or workflows are detected, and Tiber does not create duplicate automation.
- [ ] Dry-run output clearly distinguishes files that will change, files already configured, and conflicts that require an explicit choice.
- [ ] Apply mode refuses to overwrite ambiguous existing integration files without an explicit conflict-resolution choice.
- [ ] Automated regression coverage uses a repository with a populated `.gitignore` and existing task-closing automation, and proves repeated setup is safe and idempotent.
- [ ] A behavior fixture based on GitHub issue #53 rejects destructive or duplicate repository setup.

## Subtasks

## Notes / Log
