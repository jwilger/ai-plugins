---
title: Use trusted security standards in every security review
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

Security reviews should consistently check recognized industry guidance instead of relying only on an individual reviewer’s memory. Add a review lens that uses resources from the Open Worldwide Application Security Project (OWASP) and the U.S. National Institute of Standards and Technology (NIST) when relevant, so reviews are more complete, repeatable, and understandable.

## Context / Why

Choose current official OWASP and NIST resources that fit software and AI-system security reviews. Explain how the coding harness should consult them, cite applicable findings, and distinguish required fixes from non-blocking observations. Apply the guidance proportionately to the system’s real deployment and risks rather than treating every checklist item as a blocker.

## Acceptance criteria

- [ ] The security-review guidance directs reviewers to consult relevant, current NIST resources.
- [ ] Review findings identify which standard or guidance supports them in language a product manager can understand.
- [ ] The security-review guidance directs reviewers to consult relevant, current OWASP resources.
- [ ] Automated coverage demonstrates that both OWASP and NIST remain part of the review lens.
- [ ] The review applies standards proportionately to the system’s actual deployment, trust boundaries, and likely impact.

## Subtasks

## Notes / Log
