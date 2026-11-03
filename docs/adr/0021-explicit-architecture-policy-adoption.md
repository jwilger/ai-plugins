# 0021: Explicit adoption of architecture documentation practices

Status: Accepted

## Context

Development System should strongly recommend useful architecture documentation
practices without replacing a consuming project's chosen conventions. Treating
installation or setup as adoption would turn advice into unexpected delivery
blockers and could prompt unauthorized document or history migrations.

## Decision

Recommend a self-contained current architecture guide, historical significant-only
ADRs, substantive same-commit pairing, one new ADR per commit, preserved merged
history with supersession, checks of resulting delivery commits, and sequential
bootstrap with independent review. Projects explicitly choose the accepted subset,
overrides, document paths and prospective boundary in their own guidance.
Existing project policy and user direction take precedence. Installation, setup,
file presence and review prompts do not adopt these practices.

The optional read-only history checker accepts an explicit mechanical rule
selection. With no selected rules it reports skipped, not compliance. Mechanical
rule selection does not adopt additional semantic requirements. Generated project
instructions and reviewer findings must respect the same adoption boundary.
Unsupported conventions need appropriate project checks or review rather than
forced policy changes. This decision does not adopt the full profile for this
repository or migrate its historical ADRs.

## Alternatives considered

A mandatory universal profile would simplify enforcement but incorrectly override
project policy. Dropping the stricter practices or their checker would discard
useful recommendations. A new configuration schema for every practice would add
another policy authority; existing project-owned guidance and explicit checker
arguments are sufficient.

## Consequences and reconsideration

The reference retains the complete strong recommendation profile while routers
point to it with adoption constraints. Checker receipts and independent review
remain separate evidence, and neither can establish actual sequential authorship
from Git history alone. Reconsider the policy representation if real consuming
projects need reliable automatic resolution of many adopted variants; any future
resolver must preserve explicit choices and existing policy precedence.
