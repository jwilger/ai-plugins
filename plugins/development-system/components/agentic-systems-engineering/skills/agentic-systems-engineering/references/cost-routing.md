# Cost, Routing, Caching, And Provider Bake-Offs

Optimize cost per completed task, not isolated token price.

## Measure First

- Attribute calls, tokens, latency, and cost per request, step, feature, and
  tenant when relevant.
- Multi-step systems multiply cost through planner calls, worker calls, retries,
  tool round trips, and resent context.
- Optimize the measured bottleneck.

## Levers

- Prompt caching: stable prefixes can reduce repeated prompt cost but break when
  volatile context comes first.
- Semantic caching: similar requests can reuse answers but need freshness and
  correctness guards.
- Context compression: reduce repeated context while preserving facts needed for
  the next step.
- Model routing: send each step to the cheapest model that clears that step's
  quality and latency bar.

## Bake-Offs

- Compare providers and models on the project's eval set.
- Include quality, cost, p50/p95 latency, rate limits, data residency,
  compliance, operational fit, and fallback behavior.
- Keep the decision reversible through an adapter or gateway boundary.
