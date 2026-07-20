---
title: Use recognized security guidance in every security review
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

Make security reviews consistently check relevant, widely recognized security guidance so important risks are less likely to be missed and review findings are easier to justify.

## Context / Why

## Acceptance criteria

- [ ] Security review guidance tells reviewers to consider relevant OWASP resources and relevant NIST resources.
- [ ] Reviews select guidance according to the system's actual deployment model, trust boundaries, and likely impact instead of applying every catalog item as a generic blocker.
- [ ] Material findings identify the specific recognized guidance that supports them and distinguish mandatory requirements from informative recommendations.
- [ ] Automated tests or behavior evaluations fail if either OWASP or NIST coverage is removed from the security review lens.
- [ ] Affected plugin versions, marketplace metadata, documentation, and evaluation coverage are updated as required by repository policy.

## Subtasks

## Notes / Log
