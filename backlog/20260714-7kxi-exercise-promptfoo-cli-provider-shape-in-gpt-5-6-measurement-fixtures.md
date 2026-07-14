---
title: Exercise Promptfoo CLI provider shape in GPT-5.6 measurement fixtures
blocked_by: []
blocks: []
tags: [minor, evals, gpt-5.6, promptfoo, test-fidelity]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Make the strict GPT-5.6 measurement gate's positive fixture exercise Promptfoo 0.121.18's actual CLI-emitted grading-provider descriptor shape.

## Context / Why

Lightweight review found the checker accepts both configured {id,label,config} descriptors and an alternate resolved {options:{id,config,basePath},label} shape, while the canonical positive fixture covers only the alternate branch. This is a MINOR test-fidelity gap: current production compatibility is supported but not directly proven by the fast fixture.

## Acceptance criteria

## Subtasks

## Notes / Log
