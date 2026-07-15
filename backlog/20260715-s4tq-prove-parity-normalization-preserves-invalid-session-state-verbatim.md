---
title: Prove parity normalization preserves invalid session state verbatim
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

Strengthen release-parity coverage so malformed review state is proven byte-for-byte/structurally unchanged rather than merely different across source and distribution transcripts.

## Context / Why

Formal review finding tests.missing-session-preservation-not-proven: the missing-session regression can pass after partial normalization because both contract ID and timestamp differ. Add direct preservation assertions and exercise a present-but-invalid stable session_id boundary.

## Acceptance criteria

## Subtasks

## Notes / Log
