---
name: engineering-standards
description: Use when starting or substantially changing a serious project that should follow the configured architecture, typing, testing, linting, documentation, and CI quality regime.
---

# Engineering standards

Apply a functional core with an imperative shell, parse input into semantic
types, make errors explicit, and keep boundaries observable. Prefer strict
static checks, behavior-focused tests, mutation testing where valuable, and
small architectural decisions recorded near the code.

Derive security requirements from the system's real trust boundaries. For a
single-owner local tool, trust the owner, repository, environment, toolchain,
PATH, and local configuration unless the project states otherwise.
Do not block delivery on malicious local root, intentional owner bypass, or an
adversarial replacement of that trusted toolchain. Keep ordinary mistakes,
crashes, interruption, stale state, filesystem failure, partial operations,
integrity, reconciliation, recovery, and remote data loss in scope.

Do not scaffold tools blindly. Detect the stack and add only reproducible,
pinned enforcement that the project will actually run. Even when a proposed
security blocker is out of model, require reproducible quality controls,
explicit error handling, and behavior-focused tests before delivery.
