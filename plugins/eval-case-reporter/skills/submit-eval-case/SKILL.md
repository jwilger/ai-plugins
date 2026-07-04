---
name: submit-eval-case
description: Use when a plugin, skill, prompt, tool, command, or agent behavior was surprising, wrong, partial, unsafe, brittle, or worth preserving as a future eval fixture; also use when the user asks to report, submit, capture, or file an eval case.
---

# Submit Eval Case

Use this skill to turn observed AI-assistant behavior into a sanitized eval-case
issue for `slipstream-eng/ai-plugins`.

## When To Offer

Offer to submit an eval case when:

- A plugin or skill gave wrong, incomplete, overconfident, unsafe, or brittle
  guidance.
- A workflow succeeded only after retries, manual rescue, or hidden context.
- A user corrects the assistant in a way that reveals a reusable failure mode.
- A behavior should be promoted into `evals/fixtures/` as pass, fail, partial,
  or adversarial coverage.
- The user says this should become an eval, regression test, scenario, fixture,
  or GitHub issue.

Do not interrupt every ordinary bug fix. Offer only when the scenario is
reusable or diagnostic.

## Required Workflow

1. Summarize the candidate eval case in neutral terms.
2. Scrub and anonymize the sanitized input, actual behavior, expected behavior,
   and artifacts using `references/scrubbing.md`.
3. Classify the expected eval outcome: `pass`, `fail`, `partial`,
   `adversarial`, or `unsure`.
4. Draft the issue title and body locally.
5. Show the preview to the user and ask for explicit approval before posting.
6. If approved, post with:

   ```shell
   gh issue create \
     --repo slipstream-eng/ai-plugins \
     --title "[eval-case]: <short title>" \
     --label eval-case \
     --label needs-triage \
     --body-file <sanitized-body-file>
   ```

7. Report the created issue URL.

## Hard Rules

- Never post without explicit user approval of the sanitized preview.
- Never include secrets, credentials, private client data, raw proprietary
  excerpts, or access tokens.
- If you cannot confidently scrub the data, ask the user to provide a sanitized
  version instead of posting.
- If `gh` is unavailable or unauthenticated, leave the sanitized issue body in a
  local temp file and give the exact `gh issue create` command.
