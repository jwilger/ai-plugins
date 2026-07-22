---
title: Test how Tiber reports invalid command-line requests
blocked_by: []
blocks: []
tags: [tiber, cli, tests, clap, minor-review-finding]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Add public command-line tests for unknown commands, unsupported options, and invalid nested requests. Each case should return the standard parser’s status, usage guidance, and error-channel behavior.

## Context / Why

A MINOR lightweight-review finding during the Clap parser ticket observed that current focused coverage proves missing required arguments but does not explicitly exercise broader invalid invocation shapes. Deferred under the user's review disposition policy.

## Acceptance criteria

- [ ] Focused public CLI tests cover an unknown root command or option and an invalid nested command.
- [ ] Every covered invalid invocation asserts exit code 2, empty stdout, and parser-generated usage on stderr.
- [ ] The tests remain behavior-focused and the full tiber-cli suite passes without reintroducing a manual usage source.

## Subtasks

## Notes / Log

- 2026-07-22: 2026-07-22 curation rejection: Real Tiber enhancement or edge case, but lower pain, severity, frequency, or leverage than closure correctness and non-destructive setup. The reserved product slot covers backlog-limit enforcement; this item is rejected with no hidden queue.
