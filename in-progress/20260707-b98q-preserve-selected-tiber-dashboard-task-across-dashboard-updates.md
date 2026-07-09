---
title: Preserve selected Tiber dashboard task across dashboard updates
blocked_by: []
blocks: []
tags: []
claim:
  host: unknown
  session: unknown
---

## Summary

## Context / Why

## Acceptance criteria

## Subtasks

## Notes / Log

- 2026-07-07: Implementation intent: preserve selection across live dashboard updates using a LiveView-style model, similar to Phoenix LiveView in the Elixir ecosystem. Prefer an approach where the server can push incremental UI/state changes without forcing a full page reload that discards client interaction state. Investigate Rust web frameworks or libraries that support this kind of live-update behavior before hand-rolling the mechanism.
