---
title: Verify benchmark cancellation through its public interface
blocked_by: []
blocks: []
tags: [tests, test-quality, code-quality-benchmark, low-priority]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Confirm whether the code-quality benchmark runner needs its own regression test for stopping and cleaning up a live run. If distinct coverage is needed, test the runner through the same public command path a user invokes. This preserves useful cancellation coverage without extracting functions from committed source files.

## Context / Why

## Acceptance criteria

- [ ] The existing public runner cancellation coverage is compared with the code-quality benchmark runner's distinct behavior.
- [ ] If a coverage gap exists, a black-box test invokes the public benchmark command and verifies interrupt handling and cleanup without reading committed source.
- [ ] No test extracts or executes functions from a committed source file.

## Subtasks

## Notes / Log
