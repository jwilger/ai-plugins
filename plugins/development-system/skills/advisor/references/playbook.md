# Advisor planning playbook

Use this framework only when the compact Advisor protocol is insufficient.

## Shape the decision

1. Identify the optimization target: speed, parity, maintainability, cost, risk
   reduction, learning, or stakeholder alignment.
2. Find the next load-bearing decision. Treat a decision as load-bearing when a
   later reversal would require migration, redesign, data cleanup, retraining,
   or cross-team coordination.
3. Inspect repository evidence before asking the user. Check existing behavior,
   scenarios, data models, migrations, feature flags, operations, and ownership.
4. Recommend a default with its tradeoff. Ask only if a genuine fork changes the
   artifact materially.
5. Name contradictions, prerequisites, material failure boundaries, and deferred
   scope.

## Reduce complexity

Before accepting background coordination, generalized state machines, or broad
automation, test whether a user checkpoint, bounded input, single-session
assumption, manual prerequisite, or smaller first release removes a subsystem.
State what the simplification cannot support and when that limitation matters.

## Produce a specification

Include the goal, non-goals, users and permissions, observable behavior, flows,
data model, technical decisions, failure handling, security and privacy,
operational ownership, and resolved or explicitly assigned open questions.
Avoid unowned placeholders.

## Produce a ticket plan

Present the proposal before creating durable tickets. Include the goal,
recommended approach, rejected alternatives, prerequisites, ordered slices with
observable value, non-goals, deferred scope, and open risks. Split only where
acceptance criteria are independently reviewable.

## Check the result

- Ground every load-bearing recommendation.
- Resolve every hard-to-reverse decision needed to proceed.
- Cover observable success and material failure behavior.
- Remove speculative scope.
- Leave enough context for a new implementer to proceed without guessing.
