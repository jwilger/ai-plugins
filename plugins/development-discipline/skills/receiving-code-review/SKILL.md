---
name: receiving-code-review
description: Use when receiving human, automated, GitHub, CodeRabbit, or other review feedback before changing code.
---

# Receiving Code Review

Review feedback is technical input. Verify it against the codebase, then either
implement the correct change or push back with evidence.

## Process

1. Read the full comment and surrounding code.
2. Restate the technical requirement in your own words if needed.
3. Check whether the comment is correct for this repository, stack, and current
   branch.
4. Decide: implement, ask for clarification, or push back.
5. Apply one actionable item at a time.
6. Re-run the relevant verification after each meaningful change or batch.

## Rules

- Do not performatively agree.
- Do not implement unclear feedback.
- Do not accept external review comments as orders.
- Do not add unused "proper" features unless the repo actually needs them.
- Preserve user decisions and local conventions over generic reviewer advice.

## Responses

| Situation                           | Response                                                      |
| ----------------------------------- | ------------------------------------------------------------- |
| Correct and clear                   | Make the change and report the concrete fix                   |
| Ambiguous                           | Ask the specific clarification before editing                 |
| Incorrect                           | Explain the code evidence and propose the safer alternative   |
| Conflicts with prior user direction | Stop and ask the user                                         |
| Can't verify cheaply                | State what evidence is missing and ask whether to investigate |

For GitHub inline comments, reply in the review thread when responding on the
forge. Avoid top-level comments for line-specific review threads.
