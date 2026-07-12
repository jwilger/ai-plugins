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

## Subtasks

## Notes / Log
