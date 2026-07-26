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

## Two-Week Trust-Rebuild Plan

When a stakeholder was burned by a flashy demo, replace the next big reveal with
observable checkpoints:

1. Day 1: choose one sponsor-relevant workflow path, define the release claim,
   name the refusal/escalation boundaries, and ship a walking skeleton.
2. Days 2-5: build a small eval set with representative, edge, and failure-path
   cases; baseline the skeleton; inspect traces; and change one lever per loop.
3. Week 2: run repeated eval samples against thresholds, track cost and latency,
   and keep a visible list of failures that remain out of scope.
4. Final review: demo the workflow and present the data story side by side. The
   decision is promote, narrow scope, keep iterating, or stop.

## Demo Plus Data Story

- Demo the workflow path a stakeholder cares about.
- Show eval rates over the set, not selected examples.
- Attach cost and latency when they affect adoption.
- Name what the system refuses or escalates.
- Show at least one adversarial or failure-path case for trust.
