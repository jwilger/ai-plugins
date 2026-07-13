---
title: Document retained subagent slot recovery in final-review guidance
blocked_by: []
blocks: []
tags: [development-discipline, final-review, subagents, workflow, guidance, minor]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Teach final-review orchestrators how to release a collaboration slot retained by a completed reviewer without reusing that reviewer or falsifying fresh-context attestations.

## Context / Why

Observed during the Ctrl-C eval-runner final review: the collaboration runtime sometimes retained a completed reviewer and rejected the next fresh spawn at the global thread limit. The successful recovery was to trigger a no-work administrative follow-up on the retained completed agent, immediately interrupt that administrative turn, then spawn a brand-new fork_turns=none reviewer. Guidance must stress that the interrupted agent is not reused for review and that the new reviewer remains genuinely fresh. Document bounded retries and safe fallback behavior without fabricating attestations.

## Acceptance criteria

## Subtasks

## Notes / Log
