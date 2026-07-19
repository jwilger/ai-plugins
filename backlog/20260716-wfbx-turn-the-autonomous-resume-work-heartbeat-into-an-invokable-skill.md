---
title: Make automatic goal resumption available as a reusable skill
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

Create a Codex-first skill that periodically wakes an active goal and prompts work to continue, with safe start, status, stop, and restart behavior. Use each coding tool’s supported wake mechanism and clearly document when automatic resumption cannot work.

## Context / Why

The current session proved that a persistent timer can emit a custom wake event through the model-yield mechanism. Package that behavior for reuse with a 15-minute default interval. On each tick, inspect structured goal state and, only when a goal is active, prompt autonomous continuation and recommend `/goal resume` if the goal loop is paused. Treat the mechanism as best-effort across model-capacity or turn pauses; do not claim it survives a killed tool host, computer crash, or harness without a wake/yield primitive. Codex personal use is primary; other harnesses may use a documented capability-based fallback.

## Acceptance criteria

- [ ] Invoking the skill starts at most one non-blocking, session-scoped heartbeat with a configurable interval and a 15-minute default.
- [ ] Every tick reads structured goal state and emits a real model wake/yield event only when a goal is active, telling the agent to continue and to run `/goal resume` if paused.
- [ ] The skill provides observable status and safe stop/restart behavior, prevents duplicate or orphaned timers, and retries transient model-capacity interruptions without abandoning active work.
- [ ] Documentation states the exact capability and limitations, including that a terminal-only loop cannot wake the model and that the heartbeat cannot survive a killed tool host or computer crash.
- [ ] Behavior fixtures cover active-goal wakeup, inactive-goal silence, interval configuration, duplicate prevention, cleanup, and a harness-without-wake-capability fallback without exposing secrets or running arbitrary user commands.

## Subtasks

## Notes / Log

- 2026-07-16: Claude Code has a native recurring primitive: the skill should invoke `/loop 15m INSTRUCTION` (substituting the configured interval/instruction) for Claude Code instead of emulating Codex's wake/yield timer. Select the implementation by detected harness capability and document how the native loop is stopped or replaced.
