---
name: agentic-systems
description: Use for LLM, agent, RAG, structured-output, model tool-use, orchestration, AI observability, provider-routing, AI security, cost, or stochastic-evaluation work when agentic_systems is enabled.
---

# Agentic systems

Require `[features].agentic_systems = true` in
`.development-system.toml`.

Define the behavior contract, tool boundaries, failure modes, and measurable
outcomes before optimizing prompts or orchestration. Name the metrics and
acceptance thresholds before collecting production-readiness evidence. Evaluate stochastic
behavior across representative cases; one successful run is not evidence of
reliability. Keep provider calls observable and control prompt, retrieval, and
tool context explicitly.

Use project-owned Promptfoo fixtures when configured. Prefer distinct cases for
population quality and repeated samples only for a named reliability metric.
