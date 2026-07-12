---
title: Reject option tokens consumed as missing Tiber option values before writes
blocked_by: []
blocks: []
tags: [tiber, cli, major, safety, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Prevent malformed option sequences from being interpreted as literal values and reaching write-capable Tiber operations.

## Context / Why

Final review of the Clap migration confirmed this behavior already existed in the manual parser: update option pairs accepted the next recognized flag as a value, and `install-bin --target-dir --dry-run --apply` treated `--dry-run` as the directory while applying the install. This is a pre-existing MAJOR ordinary-mistake/data-or-filesystem mutation risk, not caused by ticket rpmy. Preserve legitimate hyphen-leading values through an unambiguous form such as `--field=--value` while rejecting missing values before writes.

## Acceptance criteria

- [ ] When a value-taking option is immediately followed by a recognized option token, Tiber emits parser usage on stderr, exits with the stable parser error status, and performs no task or filesystem write.

## Subtasks

## Notes / Log
