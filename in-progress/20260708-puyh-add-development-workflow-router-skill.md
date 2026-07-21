---
title: Guide development work to the right specialist workflow
blocked_by: [20260708-dsfg-add-cross-project-change-preflight-skill-to-development-discipline]
blocks: []
tags: [development-discipline, workflow-router, skills, evals]
pr_mr_url: 
pr_mr_status: 
claim:
  host: unknown
  session: unknown
---

## Summary

Add a lightweight guide that recognizes the current stage of development work and directs the agent to the appropriate existing specialist guidance. This should make the overall workflow easier to follow without copying detailed instructions into another skill.

## Context / Why

Implementation notes: The router is a lifecycle dispatcher, not a replacement for test-driven-development, systematic-debugging, verification-before-completion, final-review, receiving-code-review, PR/CI monitoring, documentation, security, OpenAI, or browser skills. Define phase precedence and stop/skip rules, respect repository-local instructions, invoke only skills available in the current harness, and explain a safe fallback when a named specialist is unavailable. The cross-project preflight task 20260708-dsfg is a prerequisite so the router does not immediately need a second structural rewrite.

## Acceptance criteria

- [x] development-discipline includes a new development-workflow router skill that triggers for development tasks from initial request through implementation, PR, CI, review, and merge readiness.
- [x] The router skill delegates detailed mechanics to existing skills including test-driven-development, systematic-debugging, verification-before-completion, final-review, receiving-code-review, babysit-pr, and relevant external documentation/security/GitHub/OpenAI/browser skills.
- [x] development-discipline README, plugin manifests, marketplace metadata, and root catalog are updated consistently, including the appropriate semver bump.
- [x] Behavior fixtures cover normal implementation routing, CI failure routing, and PR-to-merge routing through the full-marketplace eval surface.
- [ ] The change does not duplicate the full 17-step workflow across every plugin and does not embed project-specific implementation rules that belong in a consuming repo's AGENTS.md.
- [ ] An explicit routing table covers answer/review, diagnosis, implementation, verification, PR creation, CI/review response, and merge-readiness phases with named specialist skills and precedence.
- [ ] The router inspects repository state and local instructions before routing, invokes only available harness capabilities, and gives a safe documented fallback for unavailable specialists.
- [ ] Negative fixtures prove the router skips irrelevant development workflows and stops at unresolved approval, security, or external-state gates instead of continuing blindly.

## Subtasks

## Notes / Log

- 2026-07-21: Failure record: 5c97f5663d24b440e3e25d7faf5f58c8402f147d; https://github.com/jwilger/ai-plugins/actions/runs/29803703891; Quality gate; Full gate; Bats test 292 coverage checker failed with `development-discipline:development-workflow missing coverage kinds: scope-boundary, adversarial-safety`. Diagnosis: the first router fixture supplied natural/core/workflow/baseline coverage only; immutable-SHA reproduction confirmed both omissions; classification=caused. Later already-pushed fixture increments added adversarial coverage and then scope-boundary coverage; the checker passes against immutable causal-repair SHA 2e71c84af15014f674c90405a5f192db095d2fac. Next action: no further push; wait for exact repair run 29804160880. Release proof: https://github.com/jwilger/ai-plugins/actions/runs/29804160880; terminal status=pending; queued|pending|running=still blocked.
- 2026-07-21: Failure record update: exact causal-repair run https://github.com/jwilger/ai-plugins/actions/runs/29804160880 for 2e71c84af15014f674c90405a5f192db095d2fac completed successfully. Release proof: run 29804160880; terminal status=success; queued|pending|running=none. The pushed-CI failure hold from run 29803703891 is released.
- 2026-07-21: Delivered in main at c6be0596e71a0fe69da5823d408590d92480fb48. Verification: full local `just ci` green (260 development-discipline Rust tests, mutation 38 caught/6 unviable, 586 Bats); structured workflow benchmark 3/3; Codex affected behavior slices met gates (completed-diff 3/4, review-only 4/4); Claude affected slices met gates after subscription OAuth refresh (completed-diff 4/4, review-only 3/4); generated outputs passed secret scans and were removed; final review clean at diff hash ae10cac007cbec4cb7a8355f554a1ffa945a54b2. Release proof: https://github.com/jwilger/ai-plugins/actions/runs/29832800326 terminal success for exact SHA c6be0596e71a0fe69da5823d408590d92480fb48.
