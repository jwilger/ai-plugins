---
title: Prove realpath-aware bidirectional GPT-5.6 workspace overlap rejection
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

Extend workspace-isolation tests across ancestor, descendant, symlink-alias, explicit-auth, and default ~/.codex overlap cases.

## Context / Why

The current overlap matrix checks exact lexical equality only. The implementation uses realpath-aware bidirectional containment, but regressions to equality-only checks or loss of default-home protection would pass. This MINOR review finding was deferred from 20260709-spx8.

## Acceptance criteria

## Subtasks

## Notes / Log
