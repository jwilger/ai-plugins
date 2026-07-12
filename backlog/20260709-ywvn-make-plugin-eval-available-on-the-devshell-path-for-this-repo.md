---
title: Make plugin-eval available on the devshell PATH for this repo
blocked_by: []
blocks: []
tags: [developer-experience, nix, plugin-eval, tooling]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Expose plugin-eval as a reproducible command on this repository's Nix devshell PATH so documented analysis and benchmark workflows work from a clean checkout.

## Context / Why

flake.nix does not currently provide plugin-eval, and AGENTS documents a fallback into the user's installed Codex plugin cache. Add plugin-eval from an explicit pinned source recorded by the repository rather than depending on ~/.codex, a global install, or other user state. The normal documented command should work immediately after nix develop.

## Acceptance criteria

## Subtasks

## Notes / Log
