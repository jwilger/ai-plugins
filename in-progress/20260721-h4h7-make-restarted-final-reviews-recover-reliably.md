---
title: Make restarted final reviews recover reliably
blocked_by: []
blocks: []
tags: [bug, development-discipline, high-priority]
pr_mr_url: 
pr_mr_status: 
claim:
  host: unknown
  session: unknown
---

## Summary

A restarted final review can reject the exact review assignment it just created, leaving an otherwise ready change unable to proceed. Make valid restarts continue and give clear recovery guidance when review state is stale or belongs to another session.

## Context / Why

Final review is a required delivery safeguard. When its restart process cannot continue, legitimate work stops even when the documented steps were followed. A fresh review should accept its own current assignment when the reviewed change and evidence match. Genuine stale or mismatched state should produce sanitized, actionable recovery instructions. Implementation notes: cover final_review.assess_risk and final_review.plan session binding, assignment consumption, caller-carried state, and abandoned-session recovery. Source: GitHub issue #58.

## Acceptance criteria

- [ ] A restarted review accepts the assignment returned by its matching risk assessment when the session, reviewed change, and evidence are current.
- [ ] A stale, consumed, or session-mismatched assignment is rejected with sanitized details that identify the mismatch.
- [ ] The rejection tells the caller how to restart, resume, or abandon the review safely.
- [ ] Automated tests reproduce GitHub issue #58 and prove both the successful restart and each supported recovery path.
- [ ] A current risk assessment reproducing risk_assessment_assignment_id_mismatch=true is accepted through planning, including coordinator restart or recovery.
- [ ] Identity mismatch errors report sanitized expected-versus-received assignment information.

## Subtasks

## Notes / Log

- 2026-07-21: Triaged from GitHub issue #58: https://github.com/jwilger/ai-plugins/issues/58
- 2026-07-22: 2026-07-22 curation: Combined K9C6 because both describe the same cross-project delivery-blocking root cause: final-review assignment identity can be lost or mismatched across restart, assess_risk, and plan. Preserve K9C6 reproduction risk_assessment_assignment_id_mismatch=true plus sanitized expected-versus-received diagnostics.
- 2026-07-23: Failure record: 4c4ec7acaba8e27c9fd7a9a88910a28cc0057b08; run 29973841785; Quality gate; Full gate; scripts/check-development-discipline-release-from-source.sh reported development-discipline-release-parity-mismatch=true because the pushed increment changed the Rust MCP while bundled release binaries still represented the prior source. Diagnosis: the direct cause is stale packaged development-discipline binaries and parity expectations after the active ticket changed coordinator behavior; classification=caused; the exact local source-parity gate reproduces the mismatch before artifact regeneration and passes after rebuilding 0.17.5 plus isolating/resetting the parity fixture durable state. Next action: push the tested causal repair containing synchronized 0.17.5 binaries, manifests, checksums, parity harness state isolation, sanitized diagnostics, and restart guidance; commit body will name the stale-binary/parity diagnosis. Release proof: pending replacement run; terminal status=pending; queued|pending|running=still blocked.
- 2026-07-23: Release proof: GitHub Actions run 29974549063 for exact SHA 17420efbe6863805703555b070920c4f79bfd41b reached terminal success; queued|pending|running=false. The prior failed-run hold is released.
