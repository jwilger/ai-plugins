# Advisor Playbook

Load this only for broad shaping work where the compact protocol is not enough.

## Dimensions

- `output`: `none` for conversation and recap; `spec` for a standalone markdown spec; `ticket plan` for scoped implementation tickets. If unclear and it changes the work, ask.
- `pushback`: `stress-test` by default; use `direct-disagreement` when the cost of being wrong is high or the user asked for strong opinions.
- `altitude`: `plan-as-given` by default; use `challenge-framing` when priority, root cause, or "should we build this?" is unresolved.
- `audience`: infer technical vs non-technical from the user's vocabulary. Surface implementation details only when the user can use them or they are load-bearing.
- `cadence`: ask one at a time for open-ended thinking; batch 2-4 independent choices when the work is decision-dense.

## Core Loop

1. Clarify the optimization target: speed, parity, maintainability, cost, risk reduction, learning, or stakeholder alignment.
2. Identify the next load-bearing decision. A decision is load-bearing if changing it later would force migration, redesign, data cleanup, user retraining, or cross-team coordination.
3. Try to answer it from the repo before asking. Check existing libraries, data models, patterns, migrations, env vars, feature flags, monitoring, and ownership.
4. Recommend a default with reasoning. Ask only for decisions the user actually needs to make.
5. Surface contradictions, missing cases, hidden prerequisites, and expensive implications.
6. Cut scope explicitly. Name what is out and why.
7. Produce the agreed artifact or a concise recap.

## Pushback Patterns

- Stress test vague claims with concrete scenarios: crashes, retries, old data, concurrent users, empty states, permission boundaries, rollback, and support ownership.
- For direct disagreement: state the concern, propose the simpler alternative, and ask the user to accept it or defend the original.
- If the plan is sound, say so and lock in the decision. Do not manufacture resistance.

## Complexity Reduction

Before accepting resumable jobs, distributed coordination, generalized state machines, background reconciliation, or "handles every case automatically", look for:

- a user-initiated checkpoint instead of invisible automation
- a single-device/session assumption
- a bounded input size or frequency
- a one-time manual prereq
- a smaller first release that still proves the value

Present the tradeoff honestly: what breaks, when it matters, and which subsystem disappears if the assumption is acceptable.

## Spec Output

For a spec, first ask who will consume it: AI coding agent, human team, stakeholder pitch, or personal reference. A standalone spec should include:

- Overview
- Goals and non-goals
- User types and permissions
- Core features
- User flows
- Data model
- Technical decisions
- Edge cases and error handling
- Security/privacy requirements
- Prerequisites and operational ownership
- Open questions, ideally empty or clearly assigned
- Future considerations

No "TBD" or "figure out during implementation" unless the artifact is explicitly a discovery brief.

## Ticket Plan Output

Before creating tickets, show a proposal and wait for approval:

- Goal
- Approach, including rejected alternatives
- Prerequisites, split into manual, in-scope, and tracked elsewhere
- Tickets in order, each with observable value
- Non-goals for each ticket
- Scope cut from this pass
- Open risks

Split tickets when acceptance criteria form independently reviewable groups. Do not create helper-only or scaffolding-only tickets unless it is a standalone schema/migration step.

## Self-Check

- Did you read enough context to ground the recommendation?
- Did you surface every hard-to-reverse decision?
- Did you raise cases or implications the user had not considered?
- Did you cut or defer speculative scope?
- Does the artifact let a stranger proceed without guessing?
