# Lints — strict, allowlist posture

Lint policy is stack-specific and must live in the repository's checked-in tool
configuration.

Posture: start strict, then decline individual lints case by case. Each
suppression is a **documented project-policy decision**, not a convenience
suppression.

- Keep generated or tool-managed files out of lint scope unless the tool supports
  stable formatting.
- Suppress only with a reason that explains the project tradeoff.
- CI must run the repo's lint/format checks through `just ci`.
