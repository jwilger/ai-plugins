# Architecture authority and decision history

These are strong recommendations for explicit project adoption, not mandatory
requirements for projects using Development System. Recommend the full profile
and explain its tradeoffs during setup; a project may adopt a subset, adapt it,
or keep its existing conventions. Existing project policy and explicit user
choices take precedence. Plugin installation, setup, file presence and invoking
a skill do not constitute adoption.

Record the accepted rules, overrides, document paths, effective prospective
boundary and chosen checks in project-owned guidance (for example `AGENTS.md`
or `docs/rules/`). No additional `.development-system.toml` keys are required.
Do not silently migrate policy, generate unaccepted requirements, block delivery,
or report noncompliance solely because a recommendation is unadopted. Review
against the project's actual policy; keep optional improvements advisory.

The sections below describe the recommended full profile. Their prescriptive
wording applies only to the rules a project has explicitly adopted, with that
project's overrides. If adoption is unclear, inspect existing policy and ask
only when the answer affects dependent work; do not assume the full profile.

## Current architecture

`ARCHITECTURE.md` at the project root is the authoritative, self-contained
account of current architecture and architectural goals. It must provide the
requirements needed for development and review: responsibilities, boundaries,
data and control flows, invariants, constraints, and intended direction. New and
modified code must conform to it. Separate architectural intent from verified
implementation and delivery; name evidence and remaining gaps before claiming
an intended capability exists.

Do not put ADR backlinks, source tables, or directions to consult ADRs
for current requirements in `ARCHITECTURE.md`. Readers must not reconstruct the
current design from decision history. Code references and verification evidence
may support implementation claims without making historical decisions another
source of requirements.

## Historical decisions

Create an ADR only for an architecturally significant or hard-to-reverse
decision: one that changes system responsibilities, boundaries, public or
persisted contracts, long-lived constraints, or creates substantial reversal
cost. Routine implementation and tooling choices need no ADR merely because
they are choices. Document routine exceptions where their owning rule lives.

ADRs preserve context, the decision, alternatives considered and rejected,
consequences, and conditions for reconsideration. Consult them to understand
rationale or reconsider an approach. Before retrying a rejected alternative,
identify the material change in context and explain why the earlier rejection
no longer applies. ADRs are history, not competing current requirements.

## Commit invariants

- Each commit adds at most **one** ADR. A new ADR and its corresponding
  substantive `ARCHITECTURE.md` update belong in the **same commit**.
- In a commit adding an ADR, other ADRs may change only to mark them superseded
  by that new decision. Preserve their original content outside that status.
- Once an ADR has reached the project's main/trunk branch, never edit, delete,
  rename, or otherwise rewrite it except to mark it superseded by a new ADR.
  Corrections also require a replacement ADR and its paired architecture update.
- Before reaching main/trunk, an ADR may be edited, but every edit requires its
  corresponding substantive architecture update in that same commit. Formatting,
  whitespace, timestamps, or unrelated architecture changes do not satisfy pairing.
- Verify that the architecture change states the actual resulting design and
  goals introduced or revised by the decision. A changed filename or nonempty
  diff alone is insufficient evidence of substantive correspondence.

These invariants apply to the **resulting main/trunk commits**, not just branch
history. Inspect the proposed merge strategy and resulting tree/history before
delivery. Squashing several new ADRs into one commit is forbidden even when
each branch commit was valid. Split into separate delivery units when needed;
do not silently rewrite commits or change delivery strategy without authority.

When establishing several decisions from scratch, introduce them sequentially:
one new ADR, its cumulative self-contained architecture update, verification,
independent review, then its commit. Each intermediate architecture document
contains only decisions introduced so far. Do not draft all ADRs first and
manufacture incremental history afterward.

## Existing-project adoption and overrides

Inspect current architecture guidance, ADRs, trunk membership, and proposed
delivery history. Describe differences from the recommended profile without classifying them as
violations. Strongly recommend improvements and explain their tradeoffs, then
record the project's accepted choices before proposing a prospective migration.
Do not override established backlink, batching or history conventions implicitly.

Preserve merged ADR contents and unrelated work. Consolidate current requirements
and architectural goals into a self-contained `ARCHITECTURE.md`, removing its
ADR dependencies. Resolve conflicting guidance in its owning documents rather
than layering another authority over it. Do not reconstruct historical pairing
commits or rewrite old records to make past delivery satisfy the new policy.
If adoption itself introduces significant decisions, add them one at a time
under the commit invariants; a historical inventory does not authorize bulk ADRs.

Record the adoption boundary and known historical exceptions outside merged ADRs.
Apply the checks prospectively from that boundary while still protecting all
previously merged ADRs. Backfill is an explicitly scoped migration in each
existing project, never an automatic consuming-project mutation. On interruption,
preserve the exact migration snapshot and resume after reconciliation. Rollback
must preserve merged history too; revert current guidance through authorized
additive changes and use a replacement decision for an architectural reversal.

## Verification and enforcement claims

Review current architecture for self-contained authority, absence of ADR
dependencies, correspondence to each decision, and truthfulness of implementation
claims. Review ADR significance, rejected alternatives, reconsideration evidence,
and immutable history. Verify each affected commit and each proposed resulting
delivery commit, including supersession-only changes and premerge edits.

Deterministic checks can establish file changes, counts, trunk membership, and
restricted status changes. They cannot establish semantic correspondence,
architectural significance, complete authority, or whether sequential authoring
actually happened. Require independent review and behavioral evaluations for
those claims. Instructions and validation receipts do not enforce host tool
capabilities or forge merge strategy. State which checks actually ran, their
scope, and any missing enforcement; missing evidence cannot earn completion.

### Run the bundled history checker

Use this optional checker only for rules adopted in project policy. Choose the
comma-separated `--rules` subset from the recorded adoption, not from a desired
passing outcome:

- `pairing`: corresponding non-whitespace architecture update for ADR changes,
  including premerge edits and the proposed squash result.
- `one-adr`: at most one new ADR per candidate/resulting commit.
- `immutable-history`: protect all previously merged ADRs, allowing canonical
  status-only supersession by exactly one new ADR.
- `supersession-only`: when adding ADRs, restrict changes to other ADRs to that
  same canonical supersession exception.

No `--rules` means a successful `skipped` receipt with no Git inspection or
policy compliance claim. Partial adoption is supported; selecting mechanical
rules does not adopt semantic recommendations such as sequential authorship.
Custom document paths or exceptions outside the supported profile require
project-appropriate checks or independent review; never force the project to
change its policy to satisfy this parser. Do not wire unadopted rules into gates.

Before committing, review the actual staged ADR and architecture changes; this
checker reads commits, not the index or working tree. After each authorized
checkpoint commit, and again when selecting the final delivery strategy, run:

```sh
<plugin-root>/bin/development-system architecture-check \
  --rules pairing,one-adr,immutable-history,supersession-only \
  --project /absolute/project/path \
  --base <unique-merge-base-of-trunk-and-head> \
  --head <candidate-commit> \
  --trunk <verified-main-or-trunk-commit> \
  --delivery <preserve-or-squash>
```

The example selects all four mechanical rules; remove any the project has not
adopted. Replace each placeholder with an actual value; use the installed plugin root,
not a consuming project's directory. Independently verify the authoritative
main/trunk identity and complete available Git history before choosing inputs.
The command resolves commit identities and verifies the unique merge base; it
does not authenticate a forge's trunk, fetch missing history, or authorize a
commit or delivery. Never choose a later base to hide pending commits. Existing
merged history is protected but not retrospectively required to have pairing
commits. If a pending range crosses a prospective adoption boundary, split out
the already-authorized pre-adoption work explicitly; do not silently exempt it
by manipulating checker inputs or reconstruct its history.

The JSON receipt names the selected rules, resolved inputs, checked commits, mechanical checks,
violations, and remaining `review_required` obligations. Exit 0 means these
mechanical checks passed; exit 1 reports policy violations; exit 2 means invalid
inputs or an unsupported history shape. Retain the receipt with the exact
reviewed identities. For selected adopted rules, any nonzero result prevents claiming mechanical readiness;
preserve the work and resolve the actual cause rather than bypassing the check.

Supported scope is root `ARCHITECTURE.md` and Markdown ADRs below `docs/adr/`
(excluding `README.md`), with linear candidate commits. Trunk history may contain
merges. The checker rejects multiple new ADRs per candidate commit, missing or
whitespace-only architecture pairing, and forbidden merged-history edits. A
status-only supersession exception requires exactly one new ADR, a canonical
leading `## Status` section, an accepted plain status token or previous canonical
supersession, and a relative `Superseded by [ADR label](new-decision.md)` reference
to that new ADR; all surrounding historical content is preserved. Unsupported
historical formats require independent review and appropriately scoped checker
support, never rewriting merged history to satisfy the parser.

`preserve` inspects each source commit. `squash` also checks the aggregate
base-to-head change for multiple new ADRs and missing architecture pairing;
source commits must pass too. Recheck the actual proposed resulting trunk
commits if rebase, conflict resolution, or merge tooling changes the source or
base. The checker does not execute or constrain forge merge settings.

Non-whitespace content change is only a mechanical floor: cosmetic wording or
unrelated content can pass it. Independent review must establish the semantic obligations actually adopted by
the project, which may include correspondence, self-contained authority, no ADR
dependencies, code conformance, significance, evidence and sequential authorship.
The receipt's review reminders do not adopt additional rules. This command is available
for project gates but is not automatically installed as a host hook or remote CI
requirement; setup must propose the appropriate gate integration and report
anything not yet configured or run.
