---
name: content-authoring
description: Use for creating or substantially revising human-consumable content, including prose, documentation, instructions, prompts, specifications, imagery, diagrams, product copy, and UI/UX structure or interactions. Delegate artifact creation to the highest-capability eligible writable agent at high effort. Skip routine status updates, ticket metadata, commit messages, and mechanical summaries.
---

# Content authoring

Route substantive human-facing artifact creation through a capable writable
author while keeping scope and authority with the parent.

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
4. Inspect the eligible writable models advertised by the current harness and
   its authoritative capability or upgrade metadata. Select the
   highest-capability eligible writable agent explicitly and use the public
   `strong-worker` role at high effort. Never infer capability from model names,
   lexical or list order, price, or release date.
5. If authoritative ranking, explicit selection, writable launch, or high
   effort is unavailable, report a visible bounded blocked result. Do not draft
   the substantive artifact on a weaker route or silently substitute another
   agent.
6. Give the author the user's goal, audience, source facts, voice, constraints,
   exact authorized paths or output surface, and validation expectations. Do
   not prewrite the artifact and ask for cosmetic polishing.
7. Let the author create or revise the artifact and run targeted checks. The
   parent reviews the result for factual fidelity, scope, repository policy,
   and integration with adjacent work before presenting it.

The author may use a specialized image tool when imagery is part of the
artifact. Apply the same source, license, privacy, and output-path constraints
as any other tool use.

Selecting a stronger model or writable role grants no additional authority.
Keep destructive actions, external publication, credential use, repository
scope, approvals, and delivery gates separate from content quality. The author
may mutate only the files or artifact surfaces already authorized by the user
and parent.
