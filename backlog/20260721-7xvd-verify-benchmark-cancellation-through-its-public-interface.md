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

The prior test sent real interrupt and termination signals, but only after copying functions out of committed runner source into a synthetic harness. That violates the standing test-quality rule and does not prove the published command path behaves correctly. Existing public eval-runner tests already cover signal forwarding and process cleanup, so first determine whether the code-quality runner has a distinct observable behavior gap. Implementation notes: do not restore source extraction. If coverage is needed, drive the public benchmark command with controlled fake dependencies and observe exit status, child termination, reaping, and cleanup ordering.

## Acceptance criteria

- [ ] The existing public runner cancellation coverage is compared with the code-quality benchmark runner's distinct behavior.
- [ ] If a coverage gap exists, a black-box test invokes the public benchmark command and verifies interrupt handling and cleanup without reading committed source.
- [ ] No test extracts or executes functions from a committed source file.

## Subtasks

## Notes / Log

- 2026-07-22: 2026-07-22 curation rejection: Part of a large symptom-level GPT-5.6/evaluation lifecycle and artifact-quality cluster. Its present pain, confidence, or value-to-cost does not outrank the five retained root-cause items; rediscover only from a current recurring eval failure, with no shadow queue.
