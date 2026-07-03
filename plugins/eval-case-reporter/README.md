# eval-case-reporter

Capture bad, surprising, partial, or borderline AI-assistant behavior as a
sanitized GitHub issue that can later become a regression eval fixture.

## What it provides

- **`submit-eval-case`** — recognizes when a scenario may deserve an eval case,
  scrubs or anonymizes sensitive data, asks for explicit approval, and posts a
  structured issue to `slipstream-eng/ai-plugins`.

The reporter is intentionally standalone so teams can install it without also
installing any domain-specific plugin such as `agentic-systems-engineering`.

## Safety posture

The skill must never post raw secrets, credentials, private client data, or
proprietary excerpts. It prepares a preview first, asks for user approval, and
uses `gh issue create` only after that approval.

## Harnesses

Harness-agnostic — the skill (`SKILL.md` + frontmatter) is consumed by Claude
Code and Codex, with per-harness manifests included.
