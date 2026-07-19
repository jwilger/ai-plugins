---
name: rationale-commit-messages
description: Use when writing, reviewing, or proposing a Git commit message for authored development work, including after an implementation increment is complete.
---

# Rationale-Bearing Commit Messages

Write every authored commit with both:

1. A concise Conventional Commit subject.
2. A non-empty body explaining why the change is necessary: the motivation,
   tradeoff, or failure being prevented.

Reject a subject-only message. Also reject a body that merely repeats what the
subject or diff already says. Do not add `Co-Authored-By` or other
AI-attribution trailers.

Example:

```text
fix(ci): validate workflow status

A failed workflow was previously followed by unrelated changes. Checking its
result keeps the next increment focused on repairing the known failure.
```
