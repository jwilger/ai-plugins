---
title: Keep live evaluation approval consistent across providers
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

Make every repository rule agree that the owner has already approved live, repository-owned evaluations through both supported model providers. This prevents authorized quality checks from stopping for unnecessary approval requests while preserving restrictions against sending secrets or unrelated private material.

## Context / Why

The current repository guidance was narrow enough that an approval reviewer interpreted standing approval as covering only Codex/OpenAI. The owner clarified that the same standing approval covers Claude/Anthropic through the owner's existing authentication. Review the repository rules, execution policies, skills, documentation, and behavior fixtures that describe live evaluation authorization and make them consistent. Name both providers explicitly while preserving isolated authentication, secret-leak checks, protected-data exclusions, provider-specific credential handling, and restrictions on untrusted events.

## Acceptance criteria

- [ ] Repository guidance explicitly says standing approval covers both Codex/OpenAI and Claude/Anthropic live evaluations.

## Subtasks

## Notes / Log
