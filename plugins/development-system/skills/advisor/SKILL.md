---
name: advisor
description: Use for fuzzy product, design, or engineering planning; scope, specification, or ticket shaping; tradeoff analysis; challenge/help-me-think requests; and proactively before finalizing a plan with two or more dependent implementation steps unless an existing executable BDD-style scenario covers the observable behavior and its material failure boundary. Skip recursive invocation when already running as Advisor or sharpen-plan.
---

# Advisor

Delegate load-bearing planning to a read-only Advisor while keeping the parent
accountable for decisions and user communication.

## Dispatch

1. If already running as Advisor or sharpen-plan, do the advisory work directly.
   Do not invoke Advisor again.
2. Before finalizing a plan with two or more dependent implementation steps,
   inspect existing executable BDD-style scenarios. Skip Advisor only when one
   already covers both the planned observable behavior and its material failure
   boundary.
3. Inspect the harness-advertised eligible models and any authoritative
   capability or upgrade metadata. Select the highest-capability eligible model
   explicitly. Never infer capability from a model name, lexical order, list
   order, price, or release date, and do not pin a version-specific model.
4. Dispatch by harness:
   - On Claude Code, launch the exact named public `advisor` Agent with the
     explicit model selection and its configured highest-supported effort
     (currently `max`).
   - On Codex, inspect the exposed `spawn_agent` contract before invoking it.
     Proceed only if it accepts explicit `model` and `reasoning_effort` inputs
     and exposes a reliable way to confirm their effective values. Otherwise
     return exactly `Blocked: development-system:advisor cannot verify required Codex spawn_agent evidence fields: <comma-separated missing fields in task_name, model, reasoning_effort, fork_turns order>. No artifact was produced.` immediately: do not invoke
     `spawn_agent`, wait or poll, or perform or draft the advisory analysis in
     the parent. When supported, call the generic `spawn_agent` mechanism with
     `fork_turns: "none"` or the smallest bounded turn count that carries
     necessary context, the explicitly selected highest-capability model, and
     `xhigh` reasoning effort. After launch, confirm that exact model and effort
     before any wait or poll; if confirmation fails, stop without waiting or
     substituting parent-authored analysis. Supply the full Advisor role
     instructions and task context in the spawn message. Treat `task_name` only
     as an operational label, never as a role selector or evidence that the
     Advisor contract was loaded.

   For either harness, confirm that the effective child sandbox is read-only
   and that the required harness-specific effort is active before relying on
   the analysis. Never merely preserve or assume configured effort, and never
   impose Codex's literal effort label on another harness.

5. If authoritative ranking is unavailable, the model or required
   harness-specific effort cannot be selected and confirmed explicitly, the
   read-only boundary cannot be confirmed, or launch fails, report the route
   failure visibly. Do not use an unranked model, a default agent, a different
   or unconfirmed effort, or silent fallback.
6. Pass the user's goal, relevant repository context, known constraints, and the
   exact decision or artifact needed. Do not pass a preferred conclusion.
7. Wait when the recommendation blocks the plan. Otherwise continue only
   independent work. Accept the route only after the child completes and its
   substantive recommendation is visible to the parent. Empty waits, launch or
   background metadata, pending status, and a parent-authored substitute are
   not Advisor results.
8. Apply the completed recommendation with your own judgment. Present the
   decisions, pushback, risks, scope cuts, and recommended path that affect the
   user.

Keep the Advisor read-only: allow inspection and targeted current research, but
forbid edits, commits, installs, service mutations, and other persistent-state
changes. If the parent cannot confirm that the effective child sandbox enforces
that boundary, block the advisory route visibly. Instructions alone are not a
read-only sandbox. Model choice grants no additional authority.

## Advisory work

When running as Advisor:

- Start from the user's goal and optimization target.
- Ground load-bearing claims in current repository or authoritative external
  evidence when available; label principle-based judgments.
- Recommend one default path. Avoid neutral menus.
- Surface hard-to-reverse decisions, hidden prerequisites, contradictions, and
  material failure boundaries.
- Cut speculative scope while preserving baseline security, privacy, integrity,
  auditability, and recovery requirements appropriate to the real deployment.
- Ask only when a genuine fork materially changes the resulting plan or
  artifact. Otherwise decide reversible details.
- Return a compact report. Do not edit files or delegate implementation.

Read [the planning playbook](references/playbook.md) only for broad shaping,
standalone specifications, or ticket-plan work.
