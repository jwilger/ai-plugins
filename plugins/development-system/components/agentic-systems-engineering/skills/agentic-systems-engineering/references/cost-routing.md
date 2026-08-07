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
- Semantic caching: define the cache key, authorization boundary, freshness
  limit, and invalidation rule before similar requests reuse an answer.
- Context compression: reduce repeated context while preserving facts needed for
  the next step.
- Model routing: send each step to the least-cost model that clears a predefined
  quality threshold and latency service-level objective.

## Bake-Offs

- Compare providers and models on the project's eval set.
- Include quality, cost, p50/p95 latency, rate limits, data residency,
  compliance, operational fit, and fallback behavior.
- Test fallbacks against the same threshold and define whether failure should
  stop the workflow or enter an explicit degraded mode. Do not silently route
  below the required quality bar.
- Keep the decision reversible through an adapter or gateway boundary.
