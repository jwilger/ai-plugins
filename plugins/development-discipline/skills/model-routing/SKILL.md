---
name: model-routing
description: Use when selecting a model for delegated coding work or when a development workflow must escalate a helper to stronger reasoning.
---

# Model routing

Choose a model for each delegated task, not as a global session default. A
cheaper route is valid only when the work and its independent verification are
both explicit.

## Routing matrix

| Route          | Eligible work                                                               | Required boundary                                                                                                                                                    |
| -------------- | --------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `gpt-5.6-luna` | Bounded inventory, extraction, classification, or mechanical transformation | Keep the helper read-only or make its change easily reversible; define the expected result before delegation; independently verify every result before relying on it |

Do not use Luna for substantive implementation, completion claims, ambiguous
work, or any task whose result cannot be independently checked. Do not treat a
helper's own explanation as independent verification.

## Availability is part of the route

Confirm that the current harness can select the requested model before invoking
the helper. State both outcomes in the routing decision:

- when selection succeeds, name the requested route;
- when it is unavailable, inherited, or replaced, report that route failure
  visibly and keep the work in the parent or escalate it.

Never silently substitute another model or claim that a different model
satisfied the requested route.

`/fast` changes service speed and cost for a selected model. It does not select
a lower-cost model and is not a model-routing substitute.
