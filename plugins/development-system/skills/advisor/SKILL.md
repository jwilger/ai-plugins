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
     Proceed only if it accepts explicit `model`, `reasoning_effort`, and
     `fork_turns` inputs. Call the generic mechanism with `fork_turns: "none"`
     or the smallest bounded turn count that carries necessary context, the
     explicitly selected highest-capability model, and `xhigh` reasoning
     effort.

     Treat accepted explicit Codex spawn controls as authoritative.
     Do not require the spawn result to echo effective configuration.
     Do not require a per-spawn sandbox override. Supply the full Advisor role,
     read-only behavioral contract, and task context in the spawn message. Treat
     `task_name` only as an operational label, never as a role selector or
     evidence that the Advisor contract was loaded.

   For Claude Code, rely on its effective read-only Agent boundary and required
   harness-specific effort. For Codex, require the child to obey the no-write
   contract and reject a result if observed repository or persistent-state
   evidence shows that the child mutated anything. Under the trusted local-tool
   threat model, lack of a per-spawn read-only sandbox control is not itself a
   route failure. Never impose Codex's literal effort label on another harness.

5. On Codex, block only when a required input control is absent, the tool
   rejects the explicit controls or launch fails, or completion produces a
   failed or empty child result. Treat observed mutation as a failed child
   completion. Do not use an unranked model, a default agent, a different
   effort, or silent fallback.
6. Pass the user's goal, relevant repository context, known constraints, and the
   exact decision or artifact needed. Do not pass a preferred conclusion.
7. Wait when the recommendation blocks the plan. Otherwise continue only
   independent work. Accept the route only after the child reaches terminal
   completion and its substantive recommendation is visible to the parent. An
   agent identifier, launch or background metadata, pending status, and a
   parent-authored substitute are not Advisor results.
8. Apply the completed recommendation with your own judgment. Present the
   decisions, pushback, risks, scope cuts, and recommended path that affect the
   user.

Keep the Advisor read-only: allow inspection and targeted current research, but
forbid edits, commits, installs, service mutations, and other persistent-state
changes. On Codex this is a behavioral child contract within the trusted
local-tool threat model; reject the result if observed evidence shows a
mutation, without requiring a per-spawn sandbox mode. Model choice grants no
additional authority.

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
