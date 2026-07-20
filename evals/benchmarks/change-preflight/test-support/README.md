# Change-preflight benchmark workspace

Each scenario writes `change-preflight.json` in this workspace. The verifier
checks timing, classification, repository-policy evidence, all ten required
surface decisions, scenario-specific applicability, and the absence of a
speculative implementation plan.

`project/implementation-target.txt` represents the file that later
implementation would change. The verifier requires it to remain pristine so
the benchmark proves the preflight phase did not begin implementation early.
