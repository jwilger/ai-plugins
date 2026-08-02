---
name: content-authoring
description: Use for creating or substantially revising human-consumable content, including prose, documentation, instructions, prompts, specifications, imagery, diagrams, product copy, and UI/UX structure or interactions. Delegate artifact creation to the highest-capability eligible agent at high effort with a sandbox appropriate to the output surface. Skip routine status updates, ticket metadata, commit messages, and mechanical summaries.
---

# Content authoring

Route substantive human-facing artifact creation through a capable,
appropriately sandboxed author while keeping scope and authority with the
parent.

## Route the artifact

1. Confirm that the task creates or substantially revises prose,
   documentation, instructions, imagery, or UI/UX. Include structure, visual
   hierarchy, interaction design, product copy, specifications, diagrams, and
   other content whose quality depends on human judgment.
2. Skip this route for status updates, ticket metadata, commit messages, or
   mechanical summaries. Keep extraction, transcription, formatting-only
   rewrites, and deterministic transformations on their normal bounded route.
3. If already running as the assigned content-authoring worker, create the
   artifact directly. Do not delegate recursively.
4. Inspect the eligible models advertised by the current harness and its
   authoritative capability or upgrade metadata. Explicitly select the
   highest-capability candidate at high effort. Never infer capability from
   model names, lexical or list order, price, or release date. Dispatch by
   harness:
   - On Claude Code, launch the exact named public `strong-worker` Agent.
   - On Codex, do not depend on plugin-cache agent registration. Call the
     generic `spawn_agent` mechanism with `fork_turns: "none"` or the smallest
     bounded turn count that carries necessary context, the explicitly selected
     model, and high reasoning effort. Put the complete content-author role
     instructions and task context in the spawn message. `task_name` is only an
     operational label; it does not select or load a role.

     Treat accepted explicit Codex spawn controls as authoritative.
     Do not require the spawn result to echo effective configuration.
     Do not require a per-spawn `sandbox_mode` control.

   The author may write within the exact artifact boundary authorized by the
   parent. If the child cannot write, it may instead return the complete
   conversational artifact or complete patch/content for mechanical placement
   by the parent without substantive alteration. Neither route authorizes
   unrelated filesystem changes or publication.

5. On Codex, block only when a required `model`, `reasoning_effort`, or
   `fork_turns` input control is absent, the tool rejects the explicit controls
   or launch fails, or completion produces a failed or empty child result. The
   parent must not draft, outline, exemplify, template, partially create, or
   otherwise substitute any portion of the requested artifact. Mechanically
   placing a complete child-authored artifact or patch is not substitution. Do
   not use a weaker or implicit route, omit a required visual, or silently
   substitute another agent or tool.
6. Give the author the user's goal, audience, source facts, voice, constraints,
   exact authorized paths or output surface, and validation expectations. Do
   not prewrite the artifact and ask for cosmetic polishing.
7. Let the author create or revise the artifact and run targeted checks. Accept
   the route only after the child reaches terminal completion and the parent can
   inspect the substantive conversational artifact, complete patch/content, or
   concrete filesystem evidence. Background or launch metadata, pending status,
   and a parent-authored substitute are not authored results. The parent reviews
   the completed result for factual fidelity, scope, repository policy, and
   integration with adjacent work before presenting it.

The author may use a specialized image tool when imagery is part of the
artifact. Apply the same source, license, privacy, and output-path constraints
as any other tool use.

Selecting a stronger model or writable role grants no additional authority.
Keep destructive actions, external publication, credential use, repository
scope, approvals, and delivery gates separate from content quality. The author
may mutate only the files or artifact surfaces already authorized by the user
and parent.
