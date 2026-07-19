---
title: Use one parser for Tiber command-line rules
blocked_by: []
blocks: []
tags: [tiber, cli, maintainability, minor]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Remove the duplicate hand-written command checks that run before Tiber’s main command-line parser. One parser should own validation, help behavior, and error messages so the two rule sets cannot drift apart.

## Context / Why

A final-review architecture lens on 20260707-rpmy found that parse_cli_arguments manually recognizes subtask add and install-bin token layouts before invoking the derived Clap parser. This duplicates part of the grammar and makes help ordering inconsistent: malformed trailing arguments can defeat an earlier --help on those paths. The issue is MINOR and is intentionally deferred under the user’s finding policy. Keep any raw-argv normalization narrowly documented, move value constraints into typed Clap parsers where feasible, and ensure Clap remains the single owner of validation and error construction.

## Acceptance criteria

- [ ] Clap owns command validation and generated error construction without a command-specific pre-parser grammar for subtask add or install-bin.
- [ ] Help actions on the affected command paths remain parser-generated and succeed consistently even when later tokens would otherwise be malformed.
- [ ] Focused regressions preserve the intentional legacy edge cases for literal --after titles, empty predecessor rejection, and mode-looking install target paths without filesystem writes on malformed invocations.

## Subtasks

## Notes / Log
