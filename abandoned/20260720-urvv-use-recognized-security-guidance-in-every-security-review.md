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

- 2026-07-20: Implementation notes: Review the security lens and its supporting prompts, policies, tests, and behavior evaluations. Incorporate applicable resources from the Open Worldwide Application Security Project (OWASP) and the United States National Institute of Standards and Technology (NIST). Candidate references include the OWASP Top 10, Application Security Verification Standard, API Security Top 10, and relevant NIST Cybersecurity Framework or Secure Software Development Framework material. Keep the repository's proportional threat-model rule: choose resources based on actual trust boundaries and deployment, distinguish requirements from informative guidance, and keep citations and version references current.
- 2026-07-22: 2026-07-22 curation rejection after duplicate review: These two tickets are the same proportional OWASP/NIST security-review guidance work. The concept ranks below the concrete locked-dependency alert and current blockers, so both are rejected rather than preserving a hidden combined item.
