---
name: engineering-standards
description: Use when starting or substantially changing a serious project that should follow the configured architecture, typing, testing, linting, documentation, and CI quality regime.
---

# Engineering standards

Load the retained [engineering standards
contract](../../components/engineering-standards/skills/engineering-standards/SKILL.md)
for architecture and quality-regime decisions. For repository bootstrap or
toolchain enforcement, load the retained [scaffold
contract](../../components/engineering-standards/skills/scaffold/SKILL.md).

Keep deterministic domain decisions in a referentially transparent functional
core. When they require external work, return typed commands or effects for
boundary adapters to execute. Parse external
representations into invariant-carrying value objects, newtypes, refined types,
or closed alternatives; a primitive alias does not prove an invariant, and an
arbitrary primitive with no domain meaning does not need a wrapper. Represent
expected recoverable failures with typed results, stable codes, structured
context, and retained causal chains.

Use warnings-as-errors static checks, public-surface behavior scenarios, and
mutation testing where valuable. For mutation gates, define the actionable
mutant denominator and document equivalent or non-viable mutants, tool errors,
and timeouts. Record architecturally significant or hard-to-reverse decisions in
ADRs; do not create ADRs for routine implementation choices.

Route LLM, RAG, tool-use, and stochastic-evaluation concerns through
`development-system:agentic-systems`; keep this router focused on the general
engineering regime.

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
