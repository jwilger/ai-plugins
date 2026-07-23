---
title: Make the installed Tiber command work reliably
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

Tiber's supported installation command can produce a launcher that does not run correctly. Repair the installation path so users can install and invoke Tiber without falling back to repository-relative commands.

## Context / Why

A broken installed command undermines the normal setup and upgrade experience and repeatedly forces users or agents to rediscover a workaround. This ticket should resolve the behavior represented by Tiber GitHub issue 59 and keep dry-run, conflict, and existing-target safety intact.

## Acceptance criteria

## Subtasks

## Notes / Log
