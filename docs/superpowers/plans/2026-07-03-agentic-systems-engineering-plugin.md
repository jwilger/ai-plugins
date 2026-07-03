# Agentic Systems Engineering Plugin Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a portable `agentic-systems-engineering` marketplace plugin plus free/OSS eval reporting for this repo's plugins and skills.

**Architecture:** The new plugin is separate from `engineering-standards` and keeps short router skills at the plugin root with detailed doctrine in `references/` files. Eval reporting is local-first and repo-owned: promptfoo produces JSON, HTML, and JUnit artifacts, and a small Node script aggregates those artifacts into a static dashboard under `site/evals/`.

**Tech Stack:** Claude Code plugin manifests, Codex plugin manifests, Markdown skills, shell scripts, Node `.mjs`, promptfoo, Bats, `jq`, Prettier, GitHub Actions, GitHub Pages, GitHub issue forms.

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
- Create: `evals/promptfoo/agentic-systems-engineering.yaml`
- Create: `evals/fixtures/agentic-systems-engineering/*.json`
- Create: `scripts/evals/run.sh`
- Create: `scripts/evals/build-site.mjs`
- Create: `scripts/tests/evals-run.bats`
- Create: `scripts/tests/evals-site.bats`
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

Create `submit-eval-case` to recognize reusable failures, scrub/anonymize sensitive data, preview the sanitized issue, require explicit user approval, and post with `gh issue create --repo slipstream-eng/ai-plugins`.

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

- [x] **Step 1: Add deterministic fixtures**

Create pass/fail/partial/adversarial fixtures for agentic skill triggering, stochastic eval guidance, single-run-proof refusal, and force-push refusal.

- [x] **Step 2: Add promptfoo config**

Create `evals/promptfoo/agentic-systems-engineering.yaml` that can run deterministic local assertions without secrets and emits JSON, HTML, and JUnit outputs.

- [x] **Step 3: Add runner script**

Create `scripts/evals/run.sh`. It must install or invoke promptfoo through the repo's Nix/npm sandbox, write artifacts under `evals/out/`, and avoid hosted sharing as the durable record.

- [x] **Step 4: Add dashboard builder**

Create `scripts/evals/build-site.mjs`. It must read eval artifacts, generate trend-ready JSON summaries, and write static self-contained pages under `site/evals/`.

- [x] **Step 5: Add Bats tests**

Add tests that verify the runner's dry-run/help behavior and the dashboard builder's behavior against fixture artifacts.

## Task 7: Add CI And Reporting

- [x] **Step 1: Add deterministic CI job**

Update CI to run deterministic evals that do not require secrets and upload artifacts with retention.

- [x] **Step 2: Add trusted live eval workflow**

Add scheduled/manual/main workflow coverage for provider-backed evals when secrets are available. Ensure untrusted PRs skip live provider evals.

- [x] **Step 3: Add GitHub Pages deployment**

Deploy the static eval dashboard to GitHub Pages from `main` only. Do not deploy from untrusted PRs.

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

- [x] **Step 5: Run evals**

Run: `scripts/evals/run.sh`

Expected: JSON, HTML, and JUnit artifacts are generated under `evals/out/`.

- [x] **Step 6: Build eval dashboard**

Run: `node scripts/evals/build-site.mjs`

Expected: `site/evals/index.html` and `site/evals/summary.json` are generated and self-contained.

## Implementation Notes

- The first implementation artifact is this plan file.
- The active goal is to complete this plan by file reference.
- This plan may be amended if implementation discovers facts that change scope, sequence, or acceptance criteria.
- Source material inspected for implementation discipline and plugin content:
  `/home/jwilger/projects/course/docs/curriculum/*agentic-systems*`,
  `/home/jwilger/projects/course/docs/*/kb-alignment*`, and the cloned
  `thisisartium/agentic-systems-kb` under ignored `.dependencies/source/`.
  Shipped content is sanitized and paraphrased.
- Execution is following the course/KB practice, not just encoding it:
  work is in a walking-skeleton loop, deterministic evals are run before claims,
  the static dashboard is built as a data-story artifact, and reusable failures
  are routed into the eval-case intake path.
- Verification evidence:
  - Baseline `nix develop --command just ci` passed before implementation.
  - `origin/main` was fetched and fast-forwarded into this worktree during
    implementation.
  - `nix develop --command scripts/evals/run.sh` passed with 6/6 promptfoo
    cases.
  - `nix develop --command node scripts/evals/build-site.mjs` generated
    `site/evals/index.html` and `site/evals/summary.json`; generated outputs are
    ignored and published by CI.
  - `nix develop --command just ci` passed with 39 Bats tests.
  - `plugin-eval analyze` returned grade A / 100 for the six changed skills:
    `agentic-systems-engineering`, `evaluate-stochastic-systems`,
    `scaffold-agentic-evals`, `agentic-delivery`, `submit-eval-case`, and
    `engineering-standards`.
  - `plugin-eval init-benchmark` was run for those six skills. The generated
    configs were generic starter templates with no real verifiers, so they were
    not committed and benchmark execution is deferred until curated scenarios
    and verifiers exist.
