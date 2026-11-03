# Evaluation guide

## Eval reports

The repo-owned eval dashboard is generated under `site/evals/` by
`node scripts/evals/build-site.mjs`. It is a local/static artifact for review
and workflow uploads; the durable record is repo-owned and does not depend on
promptfoo-hosted sharing.

Local runs reuse the existing Codex/ChatGPT subscription session. They do not
require provider API keys or fresh approval for the
repository-owned evals authorized in [`AGENTS.md`](../AGENTS.md). Unattended trusted
automation may instead use protected provider credentials when interactive
harness sessions are unavailable; untrusted pull-request checks remain
secret-free and validate only the eval configuration and dry-run wiring.

The dashboard includes latest-run status, provider/case/sample pass rates,
threshold status, exact installed provider compositions, and separate
case-target plugin/skill summaries so regressions can be traced back to both the
loaded marketplace surface and the behavior each scenario exercises.

The canonical promptfoo behavior evals run through Promptfoo's native
`openai:codex-sdk` coding-agent provider. The runner generates
the promptfoo config from the current Codex marketplace manifest and labels
no-plugin, targeted-plugin, and full-marketplace behavior modes. Codex uses a
separate generated home for each mode. Targeted mode installs the deterministic,
deduplicated union of plugins declared by the selected behavior cases;
`EVAL_CASE_FILTER` therefore narrows both the cases and their installed plugin
set. Full-marketplace mode installs the complete Codex catalog, while no-plugin
mode installs none. The generated config records the exact installed composition
separately from the plugins targeted by an individual case. An unfiltered
targeted run equals the full catalog today because the selected cases
collectively target every catalog plugin. The two modes remain distinct
controls for filtered runs and future catalog changes.
Promptfoo is pinned at `0.125.1`; Promptfoo and the Codex SDK are pinned in
`package.json` and `package-lock.json`. The SDK pin follows Promptfoo's supported
SDK range; update the pair together and verify the live loading canary.
The separately pinned Codex CLI supplies current model support. Promptfoo's
[custom CLI option](https://www.promptfoo.dev/docs/providers/openai-codex-sdk/#custom-binary-path)
accepts this explicit SDK/CLI version difference;
fresh loading, behavior, event, and usage checks must establish compatibility
before shipping an update.
The runner disables prompt response caching and hosted
sharing so a behavior run is a fresh local record.

Default eval harness posture:

- Codex execution: `openai:codex-sdk`, `gpt-6.1-sol` with
  `model_reasoning_effort=medium`, read-only sandbox, no approvals, streaming,
  deep tracing disabled, and isolated generated homes containing no plugins,
  the selected cases' deterministic plugin union, or the complete
  harness-specific catalog according to the behavior mode.
  Model-graded assertions independently default to
  `gpt-6-astra` with high reasoning through the same SDK, so OpenAI model access
  goes through local Codex auth rather than `OPENAI_API_KEY`. Override the two
  roles separately with `CODEX_EVAL_MODEL` / `CODEX_EVAL_REASONING_EFFORT` and
  `CODEX_GRADER_MODEL` / `CODEX_GRADER_REASONING_EFFORT`.

The evaluation path is the native Codex Promptfoo runner, representative behavior
fixtures, a marketplace-loading canary, and explicit result checks. Run live
evaluations with `just evals`, which uploads the fresh results with Promptfoo
share and prints the hosted URL. For local-only results, run
`nix develop -c scripts/evals/run.sh`. Normal CI checks deterministic
functionality and evaluation configuration without making provider calls.

Historical model-family campaigns, the synthetic downstream code-quality
benchmark, and automatic improvement loops have been retired. Their custom
providers, scoring/sandbox platform, and runner-specific tests cost more to
maintain than the evidence they supplied. Shared credential/home isolation,
workspace ownership, cleanup, locking, result provenance, and sharing checks
remain. Deterministic checks establish those contracts; they do not establish
that the plugin improves agent behavior or downstream code quality. That claim
requires a suitable live evaluation.

## Reporting eval cases

When a plugin, skill, prompt, or workflow behaves incorrectly or only partially
works, file an **Eval case** issue in this repository. Eval cases are the intake
path for future regression fixtures in `evals/fixtures/`.

Include the sanitized input, actual behavior, expected behavior, expected eval
outcome (`pass`, `fail`, `partial`, `adversarial`, or `unsure`), and the
assertion or rubric that would catch the behavior. Do not include secrets,
credentials, auth headers, cookies, session ids, private keys, private client
data, private repository names, internal hostnames, or raw proprietary source
excerpts.
