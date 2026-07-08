#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  RUNNER="$ROOT/scripts/evals/run.sh"
}

@test "eval runner prints help" {
  run "$RUNNER" --help

  [ "$status" -eq 0 ]
  [[ "$output" == *"Usage: scripts/evals/run.sh"* ]]
  [[ "$output" == *"Claude Code: provider=anthropic:claude-agent-sdk, model=sonnet, skills=all"* ]]
  [[ "$output" == *"Codex:       provider=openai:codex-sdk, model=gpt-5.5, model_reasoning_effort=medium"* ]]
  [[ "$output" == *"Each provider loads the relevant marketplace surface for its harness"* ]]
  [[ "$output" == *"Pinned eval packages are managed by package.json and package-lock.json"* ]]
  [[ "$output" == *"@openai/codex-sdk"* ]]
  [[ "$output" == *"@anthropic-ai/claude-agent-sdk"* ]]
  [[ "$output" == *"Requires working Claude Code and Codex model authentication"* ]]
  [[ "$output" == *"Prompt response caching and hosted sharing are disabled"* ]]
  [[ "$output" == *"EVAL_PROVIDER_FILTER"* ]]
  [[ "$output" == *"PROMPTFOO_MAX_CONCURRENCY    (default: 1)"* ]]
  [[ "$output" == *"EVAL_TIMEOUT                 (default: 90m for full behavior runs, 20m otherwise;"* ]]
  [[ "$output" == *"EVAL_TIMEOUT_FULL_DEFAULT    (default: 90m)"* ]]
  [[ "$output" == *"EVAL_TIMEOUT_FOCUSED_DEFAULT (default: 20m)"* ]]
  [[ "$output" == *"set to 0 to disable)"* ]]
  [[ "$output" == *"EVAL_TIMEOUT_KILL_AFTER      (default: 30s; force-kill grace period)"* ]]
  [[ "$output" == *"results.junit.xml"* ]]
}

@test "eval runner dry-run uses provider-backed harness config and repo-owned artifacts" {
  run "$RUNNER" --dry-run

  [ "$status" -eq 0 ]
  [[ "$output" == *"scripts/evals/ensure-node-deps.sh"* ]]
  [[ "$output" == *"timeout --kill-after 30s 90m"* ]]
  [[ "$output" == *"node_modules/.bin/promptfoo"* ]]
  [[ "$output" != *"npx --yes"* ]]
  [[ "$output" == *"--max-concurrency 1"* ]]
  [[ "$output" == *"--no-cache"* ]]
  [[ "$output" == *"--no-share"* ]]
  [[ "$output" == *"evals/out/generated/agentic-systems-engineering.behavior.yaml"* ]]
  [[ "$output" == *"evals/out/results.json"* ]]
  [[ "$output" == *"evals/out/report.html"* ]]
  [[ "$output" == *"evals/out/results.junit.xml"* ]]
}

@test "eval runner dry-run uses repo-owned generated paths from outside repo cwd" {
  other_cwd="$(mktemp -d)"

  run bash -c 'cd "$1" && "$2" --dry-run' _ "$other_cwd" "$RUNNER"

  rm -rf "$other_cwd"
  [ "$status" -eq 0 ]
  [[ "$output" == *"$ROOT/evals/out/generated/agentic-systems-engineering.behavior.yaml"* ]]
  [[ "$output" != *"$other_cwd/evals/out/generated"* ]]
}

@test "eval runner dry-run prepares targeted Codex home from Codex marketplace plugins" {
  run "$RUNNER" --dry-run

  [ "$status" -eq 0 ]
  targeted_line="$(printf '%s\n' "$output" | grep -- '--plugin-mode targeted-plugins')"
  [[ "$targeted_line" == *"prepare-codex-home.mjs"* ]]
  [[ "$targeted_line" == *"--plugins"* ]]
  [[ "$targeted_line" == *"\\,advisor"* || "$targeted_line" == *"advisor\\,"* || "$targeted_line" == *"--plugins advisor"* ]]
}

@test "eval runner dry-run prepares only Codex grader home for Claude-only provider filter" {
  run env EVAL_PROVIDER_FILTER=claude "$RUNNER" --dry-run

  [ "$status" -eq 0 ]
  [[ "$output" == *"generate-config.mjs"* ]]
  [ "$(printf '%s\n' "$output" | grep -c 'prepare-codex-home.mjs')" -eq 1 ]
  [[ "$output" == *"--plugin-mode full-marketplace"* ]]
  [[ "$output" != *"--plugin-mode no-plugins"* ]]
  [[ "$output" != *"--plugin-mode targeted-plugins"* ]]
  [[ "$output" == *"promptfoo eval"* ]]
}

@test "eval runner dry-run prepares only selected Codex plugin mode" {
  run env EVAL_PROVIDER_FILTER=codex-gpt-5.5 "$RUNNER" --dry-run

  [ "$status" -eq 0 ]
  [ "$(printf '%s\n' "$output" | grep -c 'prepare-codex-home.mjs')" -eq 1 ]
  [[ "$output" == *"--plugin-mode full-marketplace"* ]]
  [[ "$output" != *"--plugin-mode no-plugins"* ]]
  [[ "$output" != *"--plugin-mode targeted-plugins"* ]]
}

@test "eval runner passes case filter to Promptfoo CLI" {
  run env EVAL_CASE_FILTER=tiber "$RUNNER" --dry-run

  [ "$status" -eq 0 ]
  [[ "$output" == *"timeout --kill-after 30s 20m"* ]]
  [[ "$output" == *"--filter-pattern tiber"* ]]
}

@test "eval runner dry-run can disable the promptfoo timeout" {
  run env EVAL_TIMEOUT=0 "$RUNNER" --dry-run

  [ "$status" -eq 0 ]
  [[ "$output" != *"timeout --kill-after"* ]]
  [[ "$output" == *"node_modules/.bin/promptfoo eval"* ]]
}

@test "eval runner dry-run supports shorter local default timeout overrides" {
  run env EVAL_TIMEOUT_FULL_DEFAULT=30m EVAL_TIMEOUT_FOCUSED_DEFAULT=5m "$RUNNER" --dry-run

  [ "$status" -eq 0 ]
  [[ "$output" == *"timeout --kill-after 30s 30m"* ]]

  run env EVAL_TIMEOUT_FULL_DEFAULT=30m EVAL_TIMEOUT_FOCUSED_DEFAULT=5m EVAL_CASE_FILTER=tiber "$RUNNER" --dry-run

  [ "$status" -eq 0 ]
  [[ "$output" == *"timeout --kill-after 30s 5m"* ]]
}

@test "generated eval config can filter providers" {
  run env EVAL_PROVIDER_FILTER=claude node "$ROOT/scripts/evals/generate-config.mjs" --suite behavior --stdout

  [ "$status" -eq 0 ]
  [[ "$output" == *"label: claude-code-sonnet-full-marketplace"* ]]
  [[ "$output" != *"label: codex-gpt-5.5-full-marketplace"* ]]
}

@test "generated eval config exact provider variant filter selects one full-marketplace provider" {
  run env EVAL_PROVIDER_FILTER=codex-gpt-5.5 node "$ROOT/scripts/evals/generate-config.mjs" --suite behavior --stdout

  [ "$status" -eq 0 ]
  [[ "$output" == *"label: codex-gpt-5.5-full-marketplace"* ]]
  [[ "$output" != *"label: codex-gpt-5.5-targeted-plugins"* ]]
  [[ "$output" != *"label: codex-gpt-5.5-no-plugins"* ]]
  [[ "$output" != *"label: claude-code-sonnet"* ]]
  [[ "$output" == *"pluginModes:"*$'\n'"      - id: full-marketplace"* ]]
}

@test "generated eval config combines case and provider filters without expanding provider modes" {
  run env EVAL_CASE_FILTER=tiber-new-task-command-backlog-capture EVAL_PROVIDER_FILTER=codex-gpt-5.5 node "$ROOT/scripts/evals/generate-config.mjs" --suite behavior --stdout

  [ "$status" -eq 0 ]
  [ "$(printf '%s\n' "$output" | grep -c '^  - id: openai:codex-sdk$')" -eq 1 ]
  [[ "$output" == *"label: codex-gpt-5.5-full-marketplace"* ]]
  [[ "$output" == *"evals/out/generated/load-harness-cases.runtime.cjs"* ]]
  [[ "$output" != *"label: codex-gpt-5.5-targeted-plugins"* ]]
  [[ "$output" != *"label: codex-gpt-5.5-no-plugins"* ]]
  [[ "$output" != *"label: claude-code-sonnet"* ]]
}

@test "eval runner uses project-local Promptfoo state for real runs" {
  fixture_root="$(mktemp -d)"
  mkdir -p "$fixture_root/scripts/evals" "$fixture_root/bin"
  cp "$RUNNER" "$fixture_root/scripts/evals/run.sh"
  chmod +x "$fixture_root/scripts/evals/run.sh"
  cat >"$fixture_root/scripts/evals/ensure-node-deps.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
SH
  chmod +x "$fixture_root/scripts/evals/ensure-node-deps.sh"
  cat >"$fixture_root/bin/promptfoo" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf 'PROMPTFOO_CONFIG_DIR=%s\n' "${PROMPTFOO_CONFIG_DIR:-}"
printf 'ARGS=%s\n' "$*"
SH
  chmod +x "$fixture_root/bin/promptfoo"
  touch "$fixture_root/promptfooconfig.yaml"

  run env PROMPTFOO_BIN="$fixture_root/bin/promptfoo" "$fixture_root/scripts/evals/run.sh" "$fixture_root/promptfooconfig.yaml"

  rm -rf "$fixture_root"
  [ "$status" -eq 0 ]
  [[ "$output" == *"PROMPTFOO_CONFIG_DIR=$fixture_root/.dependencies/promptfoo"* ]]
}

@test "eval threshold checker honors case min pass rates" {
  fixture_root="$(mktemp -d)"
  results="$fixture_root/results.json"
  cat >"$results" <<'JSON'
{
  "results": {
    "results": [
      {
        "success": true,
        "provider": { "id": "openai:codex-sdk" },
        "vars": { "case_id": "alpha", "plugin_mode": "full-marketplace", "min_pass_rate": 0.67 }
      },
      {
        "success": true,
        "provider": { "id": "openai:codex-sdk" },
        "vars": { "case_id": "alpha", "plugin_mode": "full-marketplace", "min_pass_rate": 0.67 }
      },
      {
        "success": true,
        "provider": { "id": "openai:codex-sdk" },
        "vars": { "case_id": "alpha", "plugin_mode": "full-marketplace", "min_pass_rate": 0.67 }
      },
      {
        "success": false,
        "provider": { "id": "openai:codex-sdk" },
        "vars": { "case_id": "alpha", "plugin_mode": "full-marketplace", "min_pass_rate": 0.67 },
        "gradingResult": { "reason": "Stochastic rubric miss" }
      }
    ]
  }
}
JSON

  run node "$ROOT/scripts/evals/check-thresholds.mjs" "$results"

  rm -rf "$fixture_root"
  [ "$status" -eq 0 ]
  [[ "$output" == *"Eval thresholds passed"* ]]
}

@test "eval threshold checker treats no-plugin misses as baseline value-gate evidence" {
  fixture_root="$(mktemp -d)"
  results="$fixture_root/results.json"
  cat >"$results" <<'JSON'
{
  "results": {
    "results": [
      {
        "provider": { "label": "codex-gpt-5.5-no-plugins" },
        "testCase": { "vars": { "case_id": "plugin-specific-safety", "plugin_mode": "no-plugins", "min_pass_rate": 1, "value_gate_mode": "safety-critical", "baseline_lift_threshold": 0 } },
        "gradingResult": { "pass": false, "score": 0, "reason": "No plugin-specific command known" }
      },
      {
        "provider": { "label": "codex-gpt-5.5-targeted-plugins" },
        "testCase": { "vars": { "case_id": "plugin-specific-safety", "plugin_mode": "targeted-plugins", "min_pass_rate": 1, "value_gate_mode": "safety-critical", "baseline_lift_threshold": 0 } },
        "gradingResult": { "pass": true, "score": 1 }
      },
      {
        "provider": { "label": "codex-gpt-5.5-full-marketplace" },
        "testCase": { "vars": { "case_id": "plugin-specific-safety", "plugin_mode": "full-marketplace", "min_pass_rate": 1, "value_gate_mode": "safety-critical", "baseline_lift_threshold": 0 } },
        "gradingResult": { "pass": true, "score": 1 }
      }
    ]
  }
}
JSON

  run node "$ROOT/scripts/evals/check-thresholds.mjs" "$results"

  rm -rf "$fixture_root"
  [ "$status" -eq 0 ]
  [[ "$output" == *"Eval thresholds passed"* ]]
}

@test "eval threshold checker skips value gates when fixture marks them none" {
  fixture_root="$(mktemp -d)"
  results="$fixture_root/results.json"
  cat >"$results" <<'JSON'
{
  "results": {
    "results": [
      {
        "provider": { "label": "codex-gpt-5.5-full-marketplace" },
        "testCase": { "vars": { "case_id": "composition", "min_pass_rate": 1, "value_gate_mode": "none" } },
        "gradingResult": { "pass": true, "score": 1 }
      },
      {
        "provider": { "label": "codex-gpt-5.5-no-plugins" },
        "testCase": { "vars": { "case_id": "composition", "min_pass_rate": 1, "value_gate_mode": "none" } },
        "gradingResult": { "pass": true, "score": 1 }
      }
    ]
  }
}
JSON

  run node "$ROOT/scripts/evals/check-thresholds.mjs" "$results"

  rm -rf "$fixture_root"
  [ "$status" -eq 0 ]
  [[ "$output" == *"Eval thresholds passed"* ]]
}

@test "hard guard allows whitelisted tiber command context for task files" {
  run node - <<'NODE'
const assertHardGuards = require("./evals/promptfoo/assert-hard-guards.cjs");
const result = assertHardGuards(
  "Use `tiber list` to inspect the board instead of directly write order.md by hand.",
  { vars: { case_id: "tiber-validation-discipline" } },
);
if (!result.pass) {
  console.error(result.reason);
  process.exit(1);
}
NODE

  [ "$status" -eq 0 ]
}

@test "eval runner exits successfully when promptfoo sample failures meet thresholds" {
  fixture_root="$(mktemp -d)"
  mkdir -p "$fixture_root/scripts/evals" "$fixture_root/bin"
  cp "$RUNNER" "$fixture_root/scripts/evals/run.sh"
  cp "$ROOT/scripts/evals/check-thresholds.mjs" "$fixture_root/scripts/evals/check-thresholds.mjs"
  chmod +x "$fixture_root/scripts/evals/run.sh" "$fixture_root/scripts/evals/check-thresholds.mjs"
  cat >"$fixture_root/scripts/evals/ensure-node-deps.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
SH
  chmod +x "$fixture_root/scripts/evals/ensure-node-deps.sh"
  cat >"$fixture_root/bin/promptfoo" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
mkdir -p evals/out
cat >evals/out/results.json <<'JSON'
{
  "results": {
    "results": [
      {
        "success": true,
        "provider": { "id": "openai:codex-sdk" },
        "vars": { "case_id": "alpha", "plugin_mode": "full-marketplace", "min_pass_rate": 0.67 }
      },
      {
        "success": true,
        "provider": { "id": "openai:codex-sdk" },
        "vars": { "case_id": "alpha", "plugin_mode": "full-marketplace", "min_pass_rate": 0.67 }
      },
      {
        "success": true,
        "provider": { "id": "openai:codex-sdk" },
        "vars": { "case_id": "alpha", "plugin_mode": "full-marketplace", "min_pass_rate": 0.67 }
      },
      {
        "success": false,
        "provider": { "id": "openai:codex-sdk" },
        "vars": { "case_id": "alpha", "plugin_mode": "full-marketplace", "min_pass_rate": 0.67 },
        "gradingResult": { "reason": "Stochastic rubric miss" }
      }
    ]
  }
}
JSON
exit 100
SH
  chmod +x "$fixture_root/bin/promptfoo"
  touch "$fixture_root/promptfooconfig.yaml"

  run env PROMPTFOO_BIN="$fixture_root/bin/promptfoo" "$fixture_root/scripts/evals/run.sh" "$fixture_root/promptfooconfig.yaml"

  rm -rf "$fixture_root"
  [ "$status" -eq 0 ]
  [[ "$output" == *"Eval thresholds passed"* ]]
}

@test "eval runner clears stale timeout status before a successful run" {
  fixture_root="$(mktemp -d)"
  mkdir -p "$fixture_root/scripts/evals" "$fixture_root/bin" "$fixture_root/evals/out"
  cp "$RUNNER" "$fixture_root/scripts/evals/run.sh"
  cp "$ROOT/scripts/evals/check-thresholds.mjs" "$fixture_root/scripts/evals/check-thresholds.mjs"
  chmod +x "$fixture_root/scripts/evals/run.sh" "$fixture_root/scripts/evals/check-thresholds.mjs"
  cat >"$fixture_root/evals/out/status.json" <<'JSON'
{
  "state": "timed-out",
  "reason": "stale timeout"
}
JSON
  cat >"$fixture_root/scripts/evals/ensure-node-deps.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
SH
  chmod +x "$fixture_root/scripts/evals/ensure-node-deps.sh"
  cat >"$fixture_root/bin/promptfoo" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
mkdir -p evals/out
cat >evals/out/results.json <<'JSON'
{
  "results": {
    "results": [
      {
        "success": true,
        "provider": { "id": "openai:codex-sdk" },
        "vars": { "case_id": "alpha", "plugin_mode": "full-marketplace", "min_pass_rate": 1 }
      }
    ]
  }
}
JSON
SH
  chmod +x "$fixture_root/bin/promptfoo"
  touch "$fixture_root/promptfooconfig.yaml"

  run env PROMPTFOO_BIN="$fixture_root/bin/promptfoo" "$fixture_root/scripts/evals/run.sh" "$fixture_root/promptfooconfig.yaml"

  [ "$status" -eq 0 ]
  [ ! -e "$fixture_root/evals/out/status.json" ]
  [[ "$output" == *"Eval thresholds passed"* ]]
  rm -rf "$fixture_root"
}

@test "eval runner writes generated runtime filter options for real generated runs" {
  fixture_bin="$(mktemp -d)"
  mkdir -p "$fixture_bin"
  cat >"$fixture_bin/promptfoo" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
cat evals/out/generated/runtime-options.json
SH
  chmod +x "$fixture_bin/promptfoo"

  run env PROMPTFOO_BIN="$fixture_bin/promptfoo" EVAL_CASE_FILTER=tiber "$RUNNER"

  rm -rf "$fixture_bin"
  rm -f "$ROOT/evals/out/generated/runtime-options.json"
  [ "$status" -eq 0 ]
  [[ "$output" == *'"caseFilter":"tiber"'* ]]
}

@test "eval runner times out a hanging promptfoo invocation" {
  fixture_root="$(mktemp -d)"
  mkdir -p "$fixture_root/scripts/evals" "$fixture_root/bin"
  cp "$RUNNER" "$fixture_root/scripts/evals/run.sh"
  chmod +x "$fixture_root/scripts/evals/run.sh"
  cat >"$fixture_root/scripts/evals/ensure-node-deps.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
SH
  chmod +x "$fixture_root/scripts/evals/ensure-node-deps.sh"
  cat >"$fixture_root/bin/promptfoo" <<'SH'
#!/usr/bin/env bash
sleep 5
SH
  chmod +x "$fixture_root/bin/promptfoo"
  touch "$fixture_root/promptfooconfig.yaml"

  run env PROMPTFOO_BIN="$fixture_root/bin/promptfoo" EVAL_TIMEOUT=1s "$fixture_root/scripts/evals/run.sh" "$fixture_root/promptfooconfig.yaml"

  [ "$status" -eq 124 ]
  [[ "$output" == *"promptfoo eval timed out after EVAL_TIMEOUT=1s"* ]]
  [ "$(jq -r '.state' "$fixture_root/evals/out/status.json")" = "timed-out" ]
  [ "$(jq -r '.reason' "$fixture_root/evals/out/status.json")" = "promptfoo eval timed out after EVAL_TIMEOUT=1s" ]
  rm -rf "$fixture_root"
}

@test "eval runner treats timeout as failure even when partial results pass thresholds" {
  fixture_root="$(mktemp -d)"
  mkdir -p "$fixture_root/scripts/evals" "$fixture_root/bin"
  cp "$RUNNER" "$fixture_root/scripts/evals/run.sh"
  cp "$ROOT/scripts/evals/check-thresholds.mjs" "$fixture_root/scripts/evals/check-thresholds.mjs"
  chmod +x "$fixture_root/scripts/evals/run.sh" "$fixture_root/scripts/evals/check-thresholds.mjs"
  cat >"$fixture_root/scripts/evals/ensure-node-deps.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
SH
  chmod +x "$fixture_root/scripts/evals/ensure-node-deps.sh"
  cat >"$fixture_root/bin/promptfoo" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
mkdir -p evals/out
cat >evals/out/results.json <<'JSON'
{
  "results": {
    "results": [
      {
        "success": true,
        "provider": { "id": "openai:codex-sdk" },
        "vars": { "case_id": "alpha", "plugin_mode": "full-marketplace", "min_pass_rate": 1 }
      }
    ]
  }
}
JSON
sleep 5
SH
  chmod +x "$fixture_root/bin/promptfoo"
  touch "$fixture_root/promptfooconfig.yaml"

  run env PROMPTFOO_BIN="$fixture_root/bin/promptfoo" EVAL_TIMEOUT=1s "$fixture_root/scripts/evals/run.sh" "$fixture_root/promptfooconfig.yaml"

  [ ! -e "$fixture_root/evals/out/results.json" ]
  [ -f "$fixture_root/evals/out/timeout-artifacts/"*/results.json ]
  [ "$status" -eq 124 ]
  [[ "$output" == *"promptfoo eval timed out after EVAL_TIMEOUT=1s"* ]]
  [ "$(jq -r '.state' "$fixture_root/evals/out/status.json")" = "timed-out" ]
  [[ "$output" == *"retained partial eval artifacts in"* ]]
  [[ "$output" == *"-exit-124."* ]]
  [[ "$output" != *"Eval thresholds passed"* ]]
  rm -rf "$fixture_root"
}

@test "eval runner treats interrupted promptfoo as failure even when partial results pass thresholds" {
  fixture_root="$(mktemp -d)"
  mkdir -p "$fixture_root/scripts/evals" "$fixture_root/bin"
  cp "$RUNNER" "$fixture_root/scripts/evals/run.sh"
  cp "$ROOT/scripts/evals/check-thresholds.mjs" "$fixture_root/scripts/evals/check-thresholds.mjs"
  chmod +x "$fixture_root/scripts/evals/run.sh" "$fixture_root/scripts/evals/check-thresholds.mjs"
  cat >"$fixture_root/scripts/evals/ensure-node-deps.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
SH
  chmod +x "$fixture_root/scripts/evals/ensure-node-deps.sh"
  cat >"$fixture_root/bin/promptfoo" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
mkdir -p evals/out
cat >evals/out/results.json <<'JSON'
{
  "results": {
    "results": [
      {
        "success": true,
        "provider": { "id": "openai:codex-sdk" },
        "vars": { "case_id": "alpha", "plugin_mode": "full-marketplace", "min_pass_rate": 1 }
      }
    ]
  }
}
JSON
exit 130
SH
  chmod +x "$fixture_root/bin/promptfoo"
  touch "$fixture_root/promptfooconfig.yaml"

  run env PROMPTFOO_BIN="$fixture_root/bin/promptfoo" "$fixture_root/scripts/evals/run.sh" "$fixture_root/promptfooconfig.yaml"

  [ ! -e "$fixture_root/evals/out/results.json" ]
  [ -f "$fixture_root/evals/out/timeout-artifacts/"*/results.json ]
  [ "$status" -eq 130 ]
  [[ "$output" == *"promptfoo eval was interrupted before completion with status 130"* ]]
  [ "$(jq -r '.state' "$fixture_root/evals/out/status.json")" = "interrupted" ]
  [[ "$output" != *"Eval thresholds passed"* ]]
  rm -rf "$fixture_root"
}

@test "eval runner force-kills a promptfoo process that ignores timeout termination" {
  fixture_root="$(mktemp -d)"
  mkdir -p "$fixture_root/scripts/evals" "$fixture_root/bin"
  cp "$RUNNER" "$fixture_root/scripts/evals/run.sh"
  chmod +x "$fixture_root/scripts/evals/run.sh"
  cat >"$fixture_root/scripts/evals/ensure-node-deps.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
SH
  chmod +x "$fixture_root/scripts/evals/ensure-node-deps.sh"
  cat >"$fixture_root/bin/promptfoo" <<'SH'
#!/usr/bin/env bash
trap '' TERM
while true; do sleep 1; done
SH
  chmod +x "$fixture_root/bin/promptfoo"
  touch "$fixture_root/promptfooconfig.yaml"

  run env PROMPTFOO_BIN="$fixture_root/bin/promptfoo" EVAL_TIMEOUT=1s EVAL_TIMEOUT_KILL_AFTER=1s "$fixture_root/scripts/evals/run.sh" "$fixture_root/promptfooconfig.yaml"

  [ "$status" -eq 137 ]
  [[ "$output" == *"promptfoo eval timed out after EVAL_TIMEOUT=1s"* ]]
  [ "$(jq -r '.state' "$fixture_root/evals/out/status.json")" = "timed-out" ]
  rm -rf "$fixture_root"
}

@test "eval runner fails when Codex marketplace has no plugin names" {
  fixture_root="$(mktemp -d)"
  mkdir -p "$fixture_root/scripts/evals" "$fixture_root/.agents/plugins"
  cp "$RUNNER" "$fixture_root/scripts/evals/run.sh"
  chmod +x "$fixture_root/scripts/evals/run.sh"
  cat >"$fixture_root/scripts/evals/generate-config.mjs" <<'NODE'
#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
const output = process.argv[process.argv.indexOf('--output') + 1];
const metadataOutput = process.argv[process.argv.indexOf('--metadata-output') + 1];
fs.mkdirSync(path.dirname(output), { recursive: true });
fs.writeFileSync(output, `providers:
  - id: openai:codex-sdk
    label: codex-gpt-5.5-targeted-plugins
    pluginMode: targeted-plugins
`);
fs.mkdirSync(path.dirname(metadataOutput), { recursive: true });
fs.writeFileSync(metadataOutput, JSON.stringify({
  usesCodexGrader: true,
  codexProviderPluginModes: ['targeted-plugins'],
}));
NODE
  cat >"$fixture_root/.agents/plugins/marketplace.json" <<'JSON'
{
  "plugins": []
}
JSON

  run "$fixture_root/scripts/evals/run.sh" --dry-run

  [ "$status" -ne 0 ]
  [[ "$output" == *"Codex marketplace has no plugins"* ]]
}

@test "package manifest pins promptfoo and coding harness provider SDKs" {
  run node - "$ROOT/package.json" <<'NODE'
const fs = require('fs');

const pkg = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
const deps = pkg.devDependencies || {};
const expected = {
  promptfoo: '0.121.17',
  '@openai/codex-sdk': '0.142.5',
  '@anthropic-ai/claude-agent-sdk': '0.3.201',
};

for (const [name, version] of Object.entries(expected)) {
  if (deps[name] !== version) {
    throw new Error(`${name} should be pinned to ${version}, got ${deps[name] || 'missing'}`);
  }
}
NODE

  [ "$status" -eq 0 ]
}
