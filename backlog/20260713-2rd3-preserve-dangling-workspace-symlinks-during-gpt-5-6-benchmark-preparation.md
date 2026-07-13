---
title: Preserve dangling workspace symlinks during GPT-5.6 benchmark preparation
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

Refuse and preserve dangling or other unowned symlinks instead of treating them as absent benchmark workspaces.

## Context / Why

The GPT-5.6 workspace helper uses existsSync before recursive recreation. A dangling symlink reports absent, so rmSync removes the unowned link and creates a directory at the same path. This MINOR review finding was deferred from 20260709-spx8.

## Acceptance criteria

## Subtasks

## Notes / Log
