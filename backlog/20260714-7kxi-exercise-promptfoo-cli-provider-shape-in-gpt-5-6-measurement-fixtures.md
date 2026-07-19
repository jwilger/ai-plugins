---
title: Test the result format actually produced by Promptfoo
blocked_by: []
blocks: []
tags: [minor, evals, gpt-5.6, promptfoo, test-fidelity]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Make the successful GPT-5.6 measurement test use the grading-provider record shape emitted by the supported Promptfoo command-line version. Keep separate coverage for any other accepted normalized format.

## Context / Why

Implementation notes:\n\nLightweight review found the checker accepts both configured {id,label,config} descriptors and an alternate resolved {options:{id,config,basePath},label} shape, while the canonical positive fixture covers only the alternate branch. This is a MINOR test-fidelity gap: current production compatibility is supported but not directly proven by the fast fixture.

## Acceptance criteria

- [ ] The canonical positive measurement artifact uses Promptfoo 0.121.18's CLI-emitted {id, label, config} grading-provider shape for both testCase.options.provider.text and prompt.config.provider.text, while a separate test retains coverage for any supported normalized alternate shape.

## Subtasks

## Notes / Log
