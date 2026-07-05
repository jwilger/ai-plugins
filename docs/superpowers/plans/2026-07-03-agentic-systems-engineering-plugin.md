# Agentic Systems Engineering Plugin Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a portable `agentic-systems-engineering` marketplace plugin plus free/OSS eval reporting for this repo's plugins and skills.

**Architecture:** The new plugin is separate from `engineering-standards` and keeps short router skills at the plugin root with detailed doctrine in `references/` files. Eval reporting is local-first and repo-owned: promptfoo drives the real Claude Code and Codex harnesses with every repository plugin loaded, produces JSON, HTML, and JUnit artifacts, and a small Node script aggregates those artifacts into a static dashboard under `site/evals/`.

**Tech Stack:** Claude Code plugin manifests, Codex plugin manifests, Markdown skills, shell scripts, Node `.mjs`, promptfoo, Bats, `jq`, Prettier, GitHub Actions, GitHub issue forms.

---

## Source Posture

- Read `../course` and the Artium agentic knowledge base during implementation.
- Use those materials only for learning and synthesis.
- Ship sanitized, paraphrased, clean-room guidance.
- Do not expose client data, proprietary implementation details, or raw private-source excerpts.
- Because the user is an Artium employee and approved digest/sanitization, no KB topic is off-limits for learning, but shipped plugin content must remain portable and non-client-exposing.

## Files

- Create: `plugins/agentic-systems-engineering/.claude-plugin/plugin.json`
- Create: `plugins/agentic-systems-engineering/.codex-plugin/plugin.json`
- Create: `plugins/agentic-systems-engineering/.mcp.json`
- Create: `plugins/agentic-systems-engineering/README.md`
- Create: `plugins/agentic-systems-engineering/skills/agentic-systems-engineering/SKILL.md`
- Create: `plugins/agentic-systems-engineering/skills/evaluate-stochastic-systems/SKILL.md`
- Create: `plugins/agentic-systems-engineering/skills/scaffold-agentic-evals/SKILL.md`
- Create: `plugins/agentic-systems-engineering/skills/agentic-delivery/SKILL.md`
- Create: `plugins/agentic-systems-engineering/skills/*/references/*.md`
- Create: `plugins/eval-case-reporter/.claude-plugin/plugin.json`
- Create: `plugins/eval-case-reporter/.codex-plugin/plugin.json`
- Create: `plugins/eval-case-reporter/README.md`
- Create: `plugins/eval-case-reporter/skills/submit-eval-case/SKILL.md`
- Create: `plugins/eval-case-reporter/skills/submit-eval-case/references/scrubbing.md`
- Modify: `.claude-plugin/marketplace.json`
- Modify: `.agents/plugins/marketplace.json`
- Modify: `README.md`
- Modify: `.gitignore`
- Modify: `plugins/engineering-standards/skills/engineering-standards/SKILL.md`
- Modify: `plugins/engineering-standards/README.md`
- Create: `.github/ISSUE_TEMPLATE/config.yml`
- Create: `.github/ISSUE_TEMPLATE/eval-case.yml`
- Create: `evals/fixtures/agentic-systems-engineering/cases.json`
- Create: `evals/promptfoo/load-harness-cases.cjs`
- Create: `evals/promptfoo/load-canary-cases.cjs`
- Create: `evals/promptfoo/assert-hard-guards.cjs`
- Create: `evals/promptfoo/assert-full-marketplace-canary.cjs`
- Create: `scripts/evals/generate-config.mjs`
- Create: `scripts/evals/prepare-codex-home.mjs`
- Create: `scripts/evals/run.sh`
- Create: `scripts/evals/build-site.mjs`
- Create: `scripts/tests/evals-run.bats`
- Create: `scripts/tests/evals-site.bats`
- Create: `scripts/tests/evals-config.bats`
- Create: `scripts/tests/evals-fixtures.bats`
- Create or modify: `.github/workflows/*.yml`
- Create: `site/evals/.gitkeep` or generated dashboard files if the repo decides to commit a baseline.

## Task 1: Baseline And Source Digest

- [x] **Step 1: Verify the worktree is isolated**

Run: `git rev-parse --git-dir && git rev-parse --git-common-dir && git branch --show-current`

Expected: `.git` resolves to worktree metadata, common dir resolves to the main repo `.git`, and branch is `agentic-systems-engineering-plugin`.

- [x] **Step 2: Run the baseline local gate**

Run: `just ci`

Expected: existing marketplace validation and Bats tests pass before implementation. If they fail, record the pre-existing failure in this plan before proceeding.

- [x] **Step 3: Inspect source material without copying**

Read available files under `../course` and the Artium agentic knowledge base clone. Record only high-level topic coverage in the implementation notes section of this plan. Do not paste source excerpts into plugin files or this plan.

## Task 2: Scaffold The Plugin

- [x] **Step 1: Add plugin manifests**

Create Claude and Codex manifests with name `agentic-systems-engineering`, description `Portable guardrails for building, evaluating, and delivering LLM and agentic systems.`, version `0.1.0`, author `John Wilger`, and license `MIT`.

- [x] **Step 2: Register marketplace entries**

Append `agentic-systems-engineering` to `.claude-plugin/marketplace.json` and `.agents/plugins/marketplace.json`. Use category `Developer Tools`, installation `AVAILABLE`, authentication `ON_INSTALL`, and source paths matching the repo's existing marketplace forms.

- [x] **Step 3: Add catalog rows**

Add the plugin to both README harness tables with the same description and version `0.1.0`.

- [x] **Step 4: Add plugin README**

Document the plugin purpose, supported harnesses, included skills, reference posture, and eval/reporting lane.

## Task 2A: Add Eval Case Reporter

- [x] **Step 1: Add standalone plugin**

Create `eval-case-reporter` as a separate marketplace plugin so teams can install eval-case intake without installing `agentic-systems-engineering`.

- [x] **Step 2: Add safe submit workflow**

Create `submit-eval-case` to recognize reusable failures, scrub/anonymize sensitive data, preview the sanitized issue, require explicit user approval, and post with `gh issue create --repo jwilger/ai-plugins`.

- [x] **Step 3: Add repo-level issue intake**

Create a GitHub issue form for eval cases and document it in the top-level README.

- [x] **Step 4: Register and catalog**

Register `eval-case-reporter` in both marketplace manifests and both README harness tables.

## Task 3: Add Agentic Systems Skills

- [x] **Step 1: Write `agentic-systems-engineering`**

Create the broad router/guardrail skill. It must trigger on LLM features, agents, tool use, RAG, structured output, prompt contracts, agent orchestration, model/provider routing, observability, evals, and delivery of AI-assisted workflows. It should route to the more specific companion skills where appropriate and require reference loading based on the user's task.

- [x] **Step 2: Write `evaluate-stochastic-systems`**

Create the stochastic eval discipline skill. It must require rate-over-set evaluation, adversarial fixtures, k-sample repetition for stochastic paths, thresholds, regression growth, and refusal to treat a single successful run as proof.

- [x] **Step 3: Write `scaffold-agentic-evals`**

Create the eval scaffold skill. It must default to promptfoo, prefer local-first OSS tooling, generate repo-owned artifacts, and describe when provider-backed live evals should skip versus run.

- [x] **Step 4: Write `agentic-delivery`**

Create the delivery practice skill. It must cover experiment loops, walking skeletons, demo plus data-story delivery, release-readiness evidence, and scope control for uncertain AI behavior.

## Task 4: Add Reference Doctrine

- [x] **Step 1: Add eval design reference**

Create a reference covering eval sets, sampling, pass/fail/partial/adversarial fixtures, thresholds, confidence, stochastic repetition, judges, and regression growth.

- [x] **Step 2: Add contracts reference**

Create a reference covering prompt contracts, structured outputs, tool schemas, parse-don't-validate boundaries, and failure modes.

- [x] **Step 3: Add RAG reference**

Create a reference covering retrieval quality, corpus fixtures, citation expectations, answerability, abstention, and retrieval-versus-generation diagnosis.

- [x] **Step 4: Add agent loops reference**

Create a reference covering agent loops, orchestration, durability, bounded execution, human-in-the-loop controls, idempotency, and recovery.

- [x] **Step 5: Add observability and security reference**

Create a reference covering tracing, metrics, audit logs, prompt injection, indirect prompt injection, data boundaries, sandboxing, and tool authorization.

- [x] **Step 6: Add cost and routing reference**

Create a reference covering cost budgets, provider routing, caching, bake-offs, fallback behavior, latency, and quality/cost tradeoffs.

- [x] **Step 7: Add delivery reference**

Create a reference covering the experiment loop, walking skeleton, demos, data stories, acceptance evidence, and stakeholder review.

## Task 5: Update Engineering Standards

- [x] **Step 1: Add force-push rule**

Update the engineering standards skill to state: no force push, force-with-lease, `-f`, or forced refspec to any remote without explicit case-by-case human authorization.

- [x] **Step 2: Point agentic work to the new plugin**

Update engineering-standards docs so LLM and agentic-system guidance routes to `agentic-systems-engineering` instead of expanding in `engineering-standards`.

## Task 6: Add Promptfoo Eval Harness

- [x] **Step 1: Add behavior fixtures**

Create pass/fail/partial/adversarial behavior fixtures for agentic skill triggering, stochastic eval guidance, single-run-proof refusal, eval-case intake, delivery practice, and force-push refusal.

- [x] **Step 2: Convert fixtures to semantic rubrics and hard guards**

Replace phrase-list `requiredConcepts` with `semanticRubric`, `hardAssertions`, `minPassRate`, tags, and calibration examples. Preserve natural behavior prompts that do not tell the model to use `ai-plugins`.

- [x] **Step 3: Generate promptfoo configs from the full marketplace**

Generate promptfoo config from `.claude-plugin/marketplace.json` and `.agents/plugins/marketplace.json` so every scenario runs with every repository plugin, skill, command, agent, MCP config, hook, and relevant plugin metadata loaded. Add a separate canary config that is allowed to explicitly verify plugin and skill loading.

- [x] **Step 4: Use native Promptfoo coding-agent providers first**

Use `openai:codex-sdk` for Codex with prepared `CODEX_HOME`, `gpt-5.5`, medium reasoning, read-only sandbox, no approvals, streaming, and deep tracing. Use `anthropic:claude-agent-sdk` / Claude Code provider with repo root as `working_dir`, all local plugins, `skills: all`, the Sonnet alias, and safe read-only/default tool posture. The intended human-facing Claude Code posture remains Sonnet high effort with Opus 4.8 advisor where that harness exposes those controls; Promptfoo's current Claude Agent SDK provider does not expose those knobs in this repo's generated config. Custom CLI providers are fallback only if native-provider canaries cannot prove full-marketplace plugin loading.

- [x] **Step 5: Add semantic and deterministic assertions**

Use promptfoo `llm-rubric` for semantic behavior grading. Use JavaScript assertions only for hard invariants such as no unauthorized force-push, no raw secret/transcript posting, and threshold aggregation. Prefer trajectory, trace, or `skill-used` assertions for canaries when the provider exposes them.

- [x] **Step 6: Add runner script**

Create `scripts/evals/run.sh`. It must install or invoke promptfoo through the repo's Nix/npm sandbox, write artifacts under `evals/out/`, and avoid hosted sharing as the durable record.

Implementation note: Promptfoo's Claude Agent SDK provider requires
`@anthropic-ai/claude-agent-sdk` to be installed as a project dependency.
Commit `package.json` and `package-lock.json` with pinned Promptfoo, Codex SDK,
and Claude Agent SDK dev dependencies; keep `node_modules/` ignored and have
the eval scripts run `npm ci` when those local dependencies are missing.

- [x] **Step 6A: Add Codex Promptfoo MCP server**

Add optional Promptfoo MCP server wiring for Codex in `agentic-systems-engineering` so agents can validate configs, run focused evals, inspect results, and develop eval cases. Keep this separate from the canonical runner and static report artifacts.

- [x] **Step 7: Add full-marketplace dashboard builder**

Create `scripts/evals/build-site.mjs`. It must read eval artifacts, generate trend-ready JSON summaries, show per-provider/per-case/per-sample pass rates and thresholds, and write static self-contained HTML reports under `site/evals/`.

- [x] **Step 8: Add Bats tests**

Add tests that verify the runner's dry-run/help behavior, dependency installation plan, generated config contents, fixture schema, hard-guard assertions, threshold summaries, and dashboard builder behavior against fixture artifacts.

## Task 7: Add CI And Reporting

- [x] **Step 1: Add PR eval config job**

Update CI to validate promptfoo configuration with a secret-free dry run on pull requests. Do not claim behavior evidence from this dry run.

- [x] **Step 2: Add trusted live eval workflow**

Add scheduled/manual/main workflow coverage for provider-backed evals when secrets are available. Ensure untrusted PRs skip live provider evals.

- [x] **Step 3: Add trusted eval artifact reporting**

Preserve trusted eval artifacts from `main`, manual, or scheduled runs without running provider-backed evals on untrusted PRs.

## Task 8: Verify

- [x] **Step 1: Validate JSON manifests**

Run: `jq empty .claude-plugin/marketplace.json && jq empty .agents/plugins/marketplace.json && find plugins -name plugin.json -exec jq empty {} \\;`

Expected: all manifests are valid JSON.

- [x] **Step 2: Validate marketplace sync**

Run: `bash scripts/validate-manifests.sh`

Expected: Claude and Codex marketplace/plugin metadata remain synchronized.

- [x] **Step 3: Check formatting**

Run: `prettier --check "**/*.{json,md}"`

Expected: all JSON and Markdown files are formatted.

- [x] **Step 4: Run full local gate**

Run: `just ci`

Expected: marketplace validation and Bats tests pass.

- [ ] **Step 5: Run full-marketplace canaries**

Run the native-provider canary suite through Claude Code and Codex.

Expected: canaries prove every repository plugin is loaded and representative skills are discoverable through native Promptfoo providers. If a native provider cannot prove this, record the evidence and fallback decision in this plan.

Status: Codex native-provider canary passed locally with the prepared isolated
`CODEX_EVAL_HOME`, absolute plugin paths, and representative-skill assertions.
Claude Code canary requires `ANTHROPIC_API_KEY`, which is not present in this
local environment.

- [ ] **Step 6: Run evals**

Run: `scripts/evals/run.sh`

Expected: full provider-backed behavior evals run with `EVAL_SAMPLES=3` through both harnesses, no cache/no share, and JSON, HTML, and JUnit artifacts are generated under `evals/out/`.

Status: blocked locally because `OPENAI_API_KEY` and `ANTHROPIC_API_KEY` are
absent. The trusted `live-evals` workflow runs this path when those repository
secrets are available.

- [x] **Step 7: Build eval dashboard**

Run: `node scripts/evals/build-site.mjs`

Expected: `site/evals/index.html` and `site/evals/summary.json` are generated and self-contained.

## Implementation Notes

- The first implementation artifact is this plan file.
- The active goal is to complete this plan by file reference.
- This plan may be amended if implementation discovers facts that change scope, sequence, or acceptance criteria.
- 2026-07-04 amendment: the eval harness must follow Promptfoo's documented coding-agent/provider patterns. Native `openai:codex-sdk` and `anthropic:claude-agent-sdk` providers are the target path; custom CLI providers are fallback only after a canary demonstrates that native providers cannot faithfully load the full repository marketplace.
- 2026-07-04 amendment: every behavior scenario runs with the complete repository plugin context, not only `agentic-systems-engineering`. The canary suite must prove full-marketplace loading, while behavior prompts remain natural and do not explicitly instruct the model to use `ai-plugins`.
- 2026-07-04 source refresh: course guidance reinforced rate-over-set evals, k-sample repetition, pass/fail/partial/adversarial fixtures, fixture growth from real misses, cache bypass for behavior evidence, deterministic guardrails outside the model, trajectory checks, evidence-anchored rubric judging, and model-family/bias controls for LLM judges.
- Source material inspected for implementation discipline and plugin content:
  `/home/jwilger/projects/course/docs/curriculum/*agentic-systems*`,
  `/home/jwilger/projects/course/docs/*/kb-alignment*`, and the cloned
  `thisisartium/agentic-systems-kb` under ignored `.dependencies/source/`.
  Shipped content is sanitized and paraphrased.
- Promptfoo docs reviewed during implementation:
  `https://www.promptfoo.dev/docs/getting-started/`,
  `https://www.promptfoo.dev/docs/providers/openai-codex-sdk/`,
  `https://www.promptfoo.dev/docs/providers/anthropic/`,
  `https://www.promptfoo.dev/docs/providers/mcp/`, and
  `https://www.promptfoo.dev/docs/integrations/mcp-server/`. The harness now
  follows the relevant shape: generated YAML config with schema, native
  `openai:codex-sdk` and `anthropic:claude-agent-sdk` providers, `llm-rubric`
  semantic grading, JavaScript hard-guard assertions returning
  pass/score/reason, JSON/HTML/JUnit outputs, trusted CI artifacts, pinned
  `promptfoo@0.121.17` plus pinned Codex/Claude SDK dev dependencies in
  `package-lock.json`, hosted sharing disabled, prompt response caching disabled
  for behavior evidence, and an optional Promptfoo MCP server for Codex
  assistant workflows.
- 2026-07-04 review fix: generated Claude Code plugin paths are absolute so the
  generated config can live under `evals/out/generated/`; Codex eval-home
  preparation refuses realpath/symlink targets for the real Codex home or auth
  source home unless explicitly overridden; canary assertions require
  representative skills, not only plugin names; unsupported Claude effort/advisor
  env vars were removed from active workflow config; failed trusted eval runs
  upload diagnostics when files exist.
- Execution is following the course/KB practice, not just encoding it:
  work is in a walking-skeleton loop, deterministic checks protect hard safety
  invariants, native-provider canaries run before behavior claims, the static
  dashboard is built as a data-story artifact, and reusable failures are routed
  into the eval-case intake path.
- Verification evidence:
  - Baseline `nix develop --command just ci` passed before implementation.
  - `origin/main` was fetched and fast-forwarded into this worktree during
    implementation.
  - `promptfoo validate config` passed for generated behavior and canary configs
    from `scripts/evals/generate-config.mjs`.
  - Native `openai:codex-sdk` canary passed locally with 5/5 marketplace plugins
    and representative skills loaded from the prepared isolated
    `CODEX_EVAL_HOME`.
  - Full local behavior eval execution is blocked in this environment because
    `OPENAI_API_KEY` and `ANTHROPIC_API_KEY` are absent. The trusted GitHub
    Actions workflows run that path when repository secrets are available.
  - `nix develop --command node scripts/evals/build-site.mjs` generated
    `site/evals/index.html` and `site/evals/summary.json`; generated outputs are
    ignored and retained only as local artifacts unless uploaded by a workflow.
  - `nix develop --command just ci` passed with 52 Bats tests.
  - `plugin-eval analyze` returned grade A / 100 for the six changed skills:
    `agentic-systems-engineering`, `evaluate-stochastic-systems`,
    `scaffold-agentic-evals`, `agentic-delivery`, `submit-eval-case`, and
    `engineering-standards`.
  - `plugin-eval init-benchmark` was run for those six skills. The generated
    configs were generic starter templates with no real verifiers, so they were
    not committed and benchmark execution is deferred until curated scenarios
    and verifiers exist.

Post-merge live-eval follow-up:

- The first post-merge trusted eval run proved that repository Actions secrets
  were not configured for `OPENAI_API_KEY` or `ANTHROPIC_API_KEY`.
- Provider-backed Claude Code and Codex eval results remain gated on the actual
  repository secrets; skipped live-eval runs are not behavior evidence.
