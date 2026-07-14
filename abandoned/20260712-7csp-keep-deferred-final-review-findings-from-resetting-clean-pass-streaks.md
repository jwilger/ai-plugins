---
title: Keep deferred final-review findings from resetting clean-pass streaks
blocked_by: []
blocks: []
tags: [development-discipline, final-review, workflow, policy]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Align final-review guidance and policy plumbing so ticketed, documented, or otherwise explicitly deferred findings do not reset consecutive clean passes over an unchanged diff.

## Context / Why

The current final-review skill tells callers that findings caused by the active diff remain actionable and defaults every in-scope severity/lens disposition to block unless project TOML says otherwise. During 20260707-rpmy, a confirmed MINOR was filed and prioritized per the user's standing policy, but recording it as accepted-risk reset the clean streak even though the diff did not change. The implementation already routes configured ticket/document/ignore dispositions outside the actionable bucket; the guidance and runtime policy mapping need to make that behavior usable for per-workflow user policy. Preserve blocking semantics for findings the policy says must be fixed now. A deferred finding must still be documented and any required ticket reference validated.

## Acceptance criteria

- [ ] Final-review guidance explicitly states that a finding deferred under the user-selected policy is routed non-blocking and does not reset a clean streak when the reviewed diff hash is unchanged.
- [ ] The caller can map a user’s in-scope severity/lens policy into ticket, document, ignore, or block dispositions without editing the reviewed project merely to configure one review.
- [ ] Regression coverage proves deferred findings require their configured report or ticket documentation while only fix-now findings, malformed results, unresolved decisions, or diff changes reset the streak.

## Subtasks

## Notes / Log

- 2026-07-13: Concrete reproduction from 20260712-kwbg formal review: three consecutive full iterations over unchanged diff hash bd9cca00d7d07a58304587cabbbdf07653aa2492 each filtered clean with all MINOR findings routed to existing ticket 20260712-5w5n. After iteration 3, coordinator state reported clean_streak=3 and unresolved_findings=[], but complete=false and verified_clean_iterations=0 because confirmed ticket-routed findings clear the separate verified-clean counter. This is the exact mismatch with the intended same-diff policy: deferred items should not prevent completion or require additional passes.
- 2026-07-14: 2026-07-14 superseded and absorbed into canonical top-priority ticket 20260713-rygd, which now covers non-blocking unchanged-diff deferral in both guidance and final-review machinery.
