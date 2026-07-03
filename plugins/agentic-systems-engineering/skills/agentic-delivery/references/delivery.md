# Delivery Reference

Agentic delivery is discovery plus engineering evidence. The goal is not to
make a model look good once; it is to learn which workflow, data, constraints,
and system shape can be made dependable.

## Walking Skeleton

- Choose one meaningful workflow path.
- Stub blocked integrations rather than waiting for every dependency.
- Include tracing from the first run.
- Include a small eval set before optimizing.
- Keep the skeleton disposable enough to simplify, but real enough to expose
  workflow and integration risk.

## Experiment Loop

1. Baseline the current system.
2. Inspect traces and failed eval cases.
3. Name the failure category.
4. Change one lever: prompt, retrieval, tool contract, model, routing, guardrail,
   or workflow boundary.
5. Rerun the eval set.
6. Promote only when the data clears the threshold and the new failure profile is
   acceptable.

## Demo Plus Data Story

- Demo the workflow path a stakeholder cares about.
- Show eval rates over the set, not selected examples.
- Attach cost and latency when they affect adoption.
- Name what the system refuses or escalates.
- Show at least one adversarial or failure-path case for trust.
