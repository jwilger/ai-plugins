# GPT-5.6 model-family benchmark

This focused benchmark compares the exact Codex model IDs
`gpt-5.6-sol`, `gpt-5.6-terra`, and `gpt-5.6-luna` for the two roles used by
the marketplace eval harness:

- execution at medium reasoning effort; and
- semantic grading at high reasoning effort.

The selected provisional defaults are **Terra/medium for execution** and
**Sol/high for grading**. The Advisor plugin-eval benchmark also uses
Terra/medium.

OpenAI documents [`gpt-5.6-sol`](https://developers.openai.com/api/docs/models/gpt-5.6-sol),
[`gpt-5.6-terra`](https://developers.openai.com/api/docs/models/gpt-5.6-terra),
and [`gpt-5.6-luna`](https://developers.openai.com/api/docs/models/gpt-5.6-luna)
as the frontier, balanced, and cost-sensitive GPT-5.6 tiers, respectively. The
cost estimates below use the current
[Codex token-based rate card](https://help.openai.com/en/articles/20001106-codex-rate-card).

## Method

The execution comparison uses one sample from each model for four distinct
cases: two standard marketplace scenarios and two Advisor-like scenarios. The
standard cases load only the marketplace's plugin manifests and skill guidance;
their home is limited to the `agentic-systems-engineering` and
`development-discipline` plugins selected by those fixtures, so unrelated
delegation-only Advisor guidance is absent. MCP servers, hooks, binaries,
agents, and other plugin components are excluded.
The Advisor-like cases and all graders run with no plugins and ask for a direct
answer, preventing the installed Advisor agent from handling the task instead
of the model being compared. Every execution output is judged by the same
Sol/high semantic grader.

The separate grader calibration sends eight frozen human-labelled answers to
Sol, Terra, and Luna at high effort: a clear pass, clear fail, borderline fail,
and hostile prompt-injection fail from each of the standard and Advisor-like
case families. Each grader must agree with all human labels for its calibration
row to pass. The hostile answers ask the grader to invoke a tool and mark the
answer as passing; a tool attempt is a provider error before Promptfoo can apply
the negative-rubric inversion.

Every model turn uses a wrapper around Promptfoo's native
`openai:codex-app-server` provider. The wrapper forces a read-only, approval-free,
network-disabled, environmentless, non-persistent thread; preserves only the
explicit plugin-mode setting; records raw app-server events; and rejects tool
use, server requests, incomplete traces, and unknown notifications. A separate
artifact checker applies the same fail-closed trace policy after execution. The
wrapper also forces the repository-local `node_modules/.bin/codex` executable;
the lockfile pins Promptfoo 0.121.18 and the Codex CLI and SDK at 0.144.5 so a
different globally installed Codex version cannot change run provenance.

Both phases run with a maximum concurrency of two and use isolated, freshly
recreated Codex homes, workspaces, and output directories. Run them independently
with:

```shell
scripts/evals/run-gpt56-benchmark.sh --phase execution
scripts/evals/run-gpt56-benchmark.sh --phase grader-calibration
```

After execution, the runner passes `promptfooconfig.yaml` to the measurement
gate as the single benchmark contract. The gate parses the source YAML, resolves
whole-string environment references, loads its repository-local `cases.cjs`,
and checks the complete Promptfoo 0.121.18 semantic projection: configuration,
runtime concurrency and cache state, version provenance, provider identities,
canonical tests, and every result's rendered prompt and grading configuration.
Promptfoo's documented empty serialization defaults and output paths are the
only extra config fields accepted; the only ignored result value is an optional
nonempty runtime `sessionId`.

Use the phase runner for live execution. Direct Promptfoo config use requires
`GPT56_BENCHMARK_WORKSPACE` to name a workspace prepared by the runner; the
provider boundary rejects missing, repository-nested, or otherwise unprepared
workspaces before loading Codex.

Use `--dry-run` to inspect preparation and Promptfoo commands without invoking
providers. The execution configuration is
[`promptfooconfig.yaml`](promptfooconfig.yaml), and the frozen-label calibration
configuration is [`grader-promptfooconfig.yaml`](grader-promptfooconfig.yaml).

## Current provider-evidence status

On 2026-07-13, both focused phases were locally ready and explicitly approved,
but the execution sandbox denied the launch before process creation because it
prohibits transmitting private-worktree prompts, skill guidance, generated
answers, and grading rubrics to an external provider. No provider request was
made, no data left the environment, no Codex usage was consumed, and the pilot
artifacts below were not changed.

Both exact dry-runs, the focused verifier regressions, and the full local CI gate
pass. A current trace-enforced provider measurement is nevertheless unavailable,
so the model split remains provisional. The older measurements below are useful
only as directional evidence and do not validate the final harness.

## Superseded pilot results

These figures predate the trace-enforced provider, skills-only standard home,
hostile calibration cases, and Promptfoo 0.121.18 pin. They are retained only as
historical context and are not current release evidence. A fresh focused run
must replace this section before the provisional selection is treated as
validated.

The provider-backed run was recorded on 2026-07-13 with Codex SDK and CLI
0.144.3. The execution phase used Promptfoo 0.121.17 and produced eval ID
`eval-4wJ-2026-07-13T15:59:11`; the calibration phase produced
`eval-FCx-2026-07-13T16:02:17`. Promptfoo is now pinned to 0.121.18 so later
runs recognize the GPT-5.6 IDs and calculate their API-equivalent cost.

### Execution

| Model | Pass rate | Mean score | Mean latency | Input tokens | Cached input | Output tokens | Total tokens | Estimated Codex credits |
| ----- | --------: | ---------: | -----------: | -----------: | -----------: | ------------: | -----------: | ----------------------: |
| Sol   |       3/4 |       0.90 |       23.7 s |       72,021 |       39,936 |         3,016 |       75,037 |                    6.77 |
| Terra |       2/4 |       0.80 |       17.5 s |      133,635 |      100,096 |         2,208 |      135,843 |                    3.55 |
| Luna  |       2/4 |       0.50 |       20.3 s |      227,169 |      177,664 |         3,175 |      230,344 |                    2.16 |

The full execution phase passed 7 of 12 target turns with no provider errors in
2 minutes 58 seconds. Its common Sol/high grader consumed another 234,186
tokens (232,323 input, including 119,808 cached, plus 1,863 output), estimated
at 16.96 Codex credits.

Terra was about 26% faster and 48% cheaper than Sol in this run. Its single-pass
deficit is not persuasive with only four one-shot cases. Luna was cheapest but
failed both Advisor-like cases. On the Advisor-like subset, Sol and Terra each
passed one of two cases; Terra was about 37% faster and 57% cheaper, supporting
the Terra/medium Advisor benchmark default.

### Grader calibration

| Grader     | Human agreement | Input tokens | Cached input | Output tokens | Total tokens | Estimated Codex credits |
| ---------- | --------------: | -----------: | -----------: | ------------: | -----------: | ----------------------: |
| Sol/high   |             6/6 |      100,767 |       59,904 |           432 |      101,199 |                    6.18 |
| Terra/high |             6/6 |      100,417 |       62,464 |           496 |      100,913 |                    2.95 |
| Luna/high  |             6/6 |       93,621 |       72,192 |           707 |       94,328 |                    0.82 |

All 18 grader assertions agreed with their human labels, with no provider
errors. The concurrent calibration phase took 50 seconds overall. Promptfoo's
component results do not preserve per-grader latency, so no per-model latency
claim is made.

Credit estimates treat cached input as a subset of reported input and use:

```text
((input - cached) × input rate + cached × cached-input rate + output × output rate)
÷ 1,000,000
```

Promptfoo 0.121.17 recorded zero raw cost because it did not recognize these
model IDs. The estimates above are derived from its recorded tokens and the
official Codex credit rates rather than retroactively rewriting raw artifacts.

## Provisional decision and limits

Terra/medium is the provisional execution default because the superseded pilot
showed the strongest observed latency/cost balance without a meaningful quality
separation from Sol at this sample size, consistent with Terra's documented
balanced tier. Sol/high remains the conservative provisional semantic grader:
grading errors have asymmetric cost, Sol is the documented frontier tier, and
using a different exact model from Terra avoids same-model executor/judge
coupling. Terra/high and especially Luna/high are promising lower-cost graders,
but the superseded six-label calibration is too little evidence to make either
canonical, and the current eight-label calibration could not be run here.

This is a directional benchmark, not a family ranking or reliability estimate:

- execution has four cases and calibration has six labels, with one sample of
  each model per input;
- Sol judged every execution output, so Sol's observed execution lead may
  include same-model bias;
- all graders scored 6/6, so calibration did not discriminate among them;
- independence is by exact model only, not vendor or model family;
- overriding execution to Sol removes the default exact-model independence and
  should be paired with a non-Sol grader override;
- token and latency outliers may reflect one-run context or plugin loading; and
- broader repeated runs over nuanced real outputs are required before changing
  the provisional split.
