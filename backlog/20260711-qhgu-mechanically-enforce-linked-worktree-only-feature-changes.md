---
title: Mechanically enforce linked-worktree-only feature changes
blocked_by: []
blocks: []
tags: [worktrees, engineering-standards, development-discipline, guardrails]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Strengthen the worktrees plugin and the applicable development/engineering workflow so feature changes cannot be made or integrated from the main checkout without an explicit, mechanically enforced exception.

## Context / Why

A completed feature branch was fast-forwarded into the main coordination checkout during normal implementation. Current commit/push hooks block main-checkout commits and pushes, but they do not prevent direct working-tree edits or a local fast-forward merge. Add proportionate guardrails that preserve legitimate coordination operations while making the linked-worktree rule hard to bypass accidentally.

## Acceptance criteria

## Subtasks

## Notes / Log
