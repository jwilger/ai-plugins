---
title: Expand focused Tiber CLI invalid-invocation coverage
blocked_by: []
blocks: []
tags: [tiber, cli, tests, clap, minor-review-finding]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Add a focused public CLI scenario matrix for unknown commands/options and invalid nested invocations, complementing the existing missing-update-field parser test.

## Context / Why

A MINOR lightweight-review finding during the Clap parser ticket observed that current focused coverage proves missing required arguments but does not explicitly exercise broader invalid invocation shapes. Deferred under the user's review disposition policy.

## Acceptance criteria

- [ ] Focused public CLI tests cover an unknown root command or option and an invalid nested command.
- [ ] Every covered invalid invocation asserts exit code 2, empty stdout, and parser-generated usage on stderr.
- [ ] The tests remain behavior-focused and the full tiber-cli suite passes without reintroducing a manual usage source.

## Subtasks

## Notes / Log
