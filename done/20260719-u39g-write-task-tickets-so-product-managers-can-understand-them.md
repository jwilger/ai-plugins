---
title: Write task tickets so product managers can understand them
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

Make task titles and main descriptions clear to a typical product manager. A reader should understand the problem, the desired outcome, and why it matters without needing specialist engineering knowledge.

## Context / Why

Implementation notes: Update the relevant ticket-writing skills and templates. Keep specialist terms out of the title and main description when plain language is accurate. When a technical term is necessary, explain it where it first appears or move the deeper detail into a clearly labeled Implementation notes section.

## Acceptance criteria

- [x] Ticket titles state the intended outcome in plain language and avoid unexplained specialist terms.
- [x] The main description explains the problem, desired outcome, and business or user value for a typical product manager.
- [x] Necessary technical terms are explained where they first appear or moved into a clearly labeled Implementation notes section.
- [x] Behavior tests cover jargon-heavy titles, unexplained technical descriptions, and compliant plain-language tickets.

## Subtasks

## Notes / Log

- 2026-07-20: Completed in signed commits b478549 and 069100d on main. Verification: focused structural and fixture tests passed 7/7; the provider-backed plain-language scenario passed 2/2 across Claude and Codex; plugin-eval analysis scored 100/100; formal final review completed clean across all nine lenses; full local just ci completed mutation testing and all 579 shell tests; exact GitHub CI run 29713808620 for commit 069100d finished successfully, including the CI gate.
