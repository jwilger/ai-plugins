---
name: development-workflow
description: Use when making repository changes, debugging, handling review feedback, verifying work, conducting final review, recovering CI, or deciding the next development lifecycle step.
---

# Development workflow

Use `workspace-reader.status` before choosing a workflow. Outside Git, without
configuration, or with invalid configuration, inspection remains available but
the plugin cannot provide configured workflow guidance. This advisory state
does not deny ordinary host mutation capabilities.

For every workflow question, load [workflow
rules](references/workflow-rules.md). Before editing, classify artifact impact
relations and RED applicability. Keep diagnosis causal, bind readiness claims
to post-mutation exact-revision evidence, select review lenses from changed
system boundaries, and use the configured delivery mode.

Load the retained specialist contract only when its intent matches:

- Change classification or preflight: [change
  preflight](../../components/development-discipline/skills/change-preflight/SKILL.md).
- Causal debugging: [systematic
  debugging](../../components/development-discipline/skills/systematic-debugging/SKILL.md).
- RED-GREEN-refactor behavior: [test-driven
  development](../../components/development-discipline/skills/test-driven-development/SKILL.md).
- Exact-revision evidence: [verification before
  completion](../../components/development-discipline/skills/verification-before-completion/SKILL.md).
- Final-review scope, findings, or verifier flow: [final
  review](../../components/development-discipline/skills/final-review/SKILL.md).
- Worker or reviewer selection: [model
  routing](../../components/development-discipline/skills/model-routing/SKILL.md).
- Failed pushed CI: [CI failure
  follow-up](../../components/development-discipline/skills/ci-failure-follow-up/SKILL.md).
- Lifecycle-aware delivery: [delivery
  workflow](../../components/development-discipline/skills/delivery-workflow/SKILL.md).
- Commit-message rationale: [rationale commit
  messages](../../components/development-discipline/skills/rationale-commit-messages/SKILL.md).
- Authoring or reviewing a skill: [writing
  skills](../../components/development-discipline/skills/writing-skills/SKILL.md).

For every final-review path, keep these fail-closed invariants visible in the
working context rather than relying on a later summary. When asked how to run
or complete final review, state every applicable invariant before adjacent
delivery detail:

- Honor every explicit baseline, path scope, and read-only boundary. Otherwise,
  review the pinned baseline plus every unexcluded staged, unstaged, committed,
  renamed, mode-changed, and untracked byte. A review-only request reports
  findings and never edits; remediate valid findings only when remediation was
  requested.
- Use a fresh-context subagent for every assigned lens in every iteration.
  Always include an independent `production-risk-footguns` lens for latent
  traps, fragile defaults, production-scale data/resource access, and
  burst/DOS-like load behavior; add other repository-agnostic lenses according
  to assessed risk.
- Every submitted canonical finding—including out-of-scope, report-only,
  already-known, defended, or later rejected feedback—makes that iteration
  non-clean, clears the clean streak and verified-clean receipts, and requires
  the complete selected lens set again in fresh contexts. Disposition controls
  remediation or reporting only; never carry unaffected peer evidence forward.
- Feed every defense or caller decision back to the relevant fresh lens
  reviewers. Never accept a defense only in the caller context. A defense
  cannot make its finding-bearing iteration clean; completion still requires
  three later consecutive complete finding-free iterations.
- For pushed work, the most recently completed in-scope build must be green.
  A queued or running build is only waiting, never replacement evidence, and a
  completed failed required job creates the CI-recovery hold.
- When exceptional risk is at issue, explicitly return this complete checklist;
  never summarize an item away:
  1. Reject an empty, unknown, or invented exceptional trigger. The only
     supported IDs are `destructive-or-irreversible-operation`,
     `authentication-or-authorization-boundary`,
     `sensitive-data-migration`, `cryptographic-behavior`, and
     `safety-critical-behavior`.
  2. Require both a supported trigger and an explicitly exceptional dimension
     for an exceptional overall profile.
  3. Keep supported triggers recorded on a lower profile when concrete
     mitigations keep risk below exceptional.
  4. Give the extra independent discovery sample only to dimensions explicitly
     rated exceptional.
  5. Validate every later delta with the same supported-trigger and
     explicitly-exceptional-dimension rules.
  6. For a legitimate material-delta reassessment, merge and preserve trigger
     evidence in authoritative state, clear the clean streak and verified-clean
     receipts, invalidate every old peer result, and require three fresh,
     consecutive, complete, finding-free iterations of the entire selected
     lens set.

When a `final_review.plan` handoff is truncated, recover it instead of
replanning. Call `final_review.pending_assignments` with the retained
`state_ref`; its compact summary is authoritative for every pending lens,
verifier, or delta-risk assignment, including the exact `subagent_key`, exact
`model_role`, phase metadata, close policy, result-schema version, and the
phase-specific assignment identity and evidence references. Pass one exact
`subagent_key` back to that tool to retrieve only that assignment's original
durable prompt and schema. An incomplete `final_review.resume_latest` returns
the same compact summary. These retrievals are read-only and idempotent: they
append no event, change no revision, fingerprint, scope, or hash, and never
reassign a lens or role. If a pre-upgrade pending verifier or delta-risk record
lacks recoverable assignment facts, follow its explicit
`recovery=restart_final_review` guidance; never reconstruct it from partial
projection state. A missing or invalid lifecycle/model attestation is malformed
assigned evidence: accept the coordinator's non-clean reset transition, discard
the full iteration, and execute every fresh next-iteration assignment. Never
repair the old assignment or preserve peer results from that invalidated
iteration. Apply the same rule to malformed verifier evidence and invalid
verifier assignment provenance or verdict coverage: close the pending verifier
and rerun the full selected lens set returned by the reset transition.

The pinned Git baseline plus in-repository paths, contents, modes, and untracked
inventory define reviewed source scope. Coordinator events, snapshots, state
files, and projections are bookkeeping outside that hash. After terminal source
review, a content-identical commit does not reopen review merely because HEAD,
staging partition, signature, or commit metadata changed. Real path, content,
mode, untracked-content, pinned-baseline, or requested-scope changes do reopen
it. The one-way delivery sequence remains: source-content review, commit, a
fresh full gate bound to that exact commit, then push and exact-revision CI.

Load `delivery-workflow` before the first implementation increment so the
configured delivery mode and green-increment preservation cadence are known
before any commit or push. Recheck it after final review for final delivery.

## Structured lifecycle

The bootstrap plugin exposes structured `final_review.*` coordination, but not
the native `workflow.*`, `workspace-editor.*`, or `project-runner.*` services.
Use ordinary host tools for authorized repository changes and verification, and
follow the RED-GREEN-refactor, exact-evidence, review, and delivery guidance in
the specialist contracts above. Do not claim that plugin instructions, hooks,
or final-review records enforce harness capabilities or authorize external
actions.

The retained source-level native lifecycle is for the planned standalone Tiber
scheduler. It is not bound to the currently installed Tiber task-board binary.
Once that separate scheduler binding ships, assignment-bound editor and runner
receipts support the mechanical RED, GREEN, verification, review, and delivery
transitions: `workflow.record_red`,
`workflow.authorize_implementation`, `workflow.record_green`,
`workflow.begin_verification`, `workflow.record_verification`,
`workflow.record_clean_review`, `workflow.authorize_delivery`, and
`workflow.complete_delivery`. Until then, do not call or emulate those native
transitions through the advisory bootstrap surface or claim that installed
Tiber task transitions enforce them.

Before a new task, inspect forge CI. Require a successful completed run for the
candidate revision; queued/running is not terminal evidence, and any completed
failed required job activates a repository-wide hold. Use Tiber's single
CI-recovery owner and record SHA, run, checks, and terminal status. Development
Discipline reads that Tiber hold when gating delivery; it does not own one.
