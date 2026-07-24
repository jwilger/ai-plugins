---
title: Enforce the stochastic macro SDLC in Pi
blocked_by: []
blocks: []
tags: [pi, sdlc, guardrails]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Add a first-party Pi extension backed by a deterministic Rust state machine that enforces the dependency graph and approval boundaries represented by the Stochastic Macro diagram.

## Context / Why

Prompt guidance alone cannot prevent an agent from skipping definition, review, acceptance, deployment, or production-confirmation gates. Pi event interception and a durable evidence ledger can make ordering and controlled delivery actions fail closed while leaving stochastic quality judgments to the existing skills and reviewers.

## Acceptance criteria

- [ ] Pi exposes deterministic SDLC status and evidence-backed milestone advancement for the complete diagram dependency graph.

## Subtasks

## Notes / Log
