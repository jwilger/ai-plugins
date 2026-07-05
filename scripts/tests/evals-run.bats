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
  [[ "$output" == *"results.junit.xml"* ]]
}

@test "eval runner dry-run uses provider-backed harness config and repo-owned artifacts" {
  run "$RUNNER" --dry-run

  [ "$status" -eq 0 ]
  [[ "$output" == *"scripts/evals/ensure-node-deps.sh"* ]]
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

@test "eval runner dry-run prepares targeted Codex home from Codex marketplace plugins" {
  run "$RUNNER" --dry-run

  [ "$status" -eq 0 ]
  targeted_line="$(printf '%s\n' "$output" | grep -- '--plugin-mode targeted-plugins')"
  [[ "$targeted_line" == *"prepare-codex-home.mjs"* ]]
  [[ "$targeted_line" == *"--plugins"* ]]
  [[ "$targeted_line" == *"\\,advisor"* || "$targeted_line" == *"advisor\\,"* || "$targeted_line" == *"--plugins advisor"* ]]
}

@test "eval runner passes case filter to Promptfoo CLI" {
  run env EVAL_CASE_FILTER=taskbranch "$RUNNER" --dry-run

  [ "$status" -eq 0 ]
  [[ "$output" == *"--filter-pattern taskbranch"* ]]
}

@test "generated eval config can filter providers" {
  run env EVAL_PROVIDER_FILTER=claude node "$ROOT/scripts/evals/generate-config.mjs" --suite behavior --stdout

  [ "$status" -eq 0 ]
  [[ "$output" == *"label: claude-code-sonnet-full-marketplace"* ]]
  [[ "$output" != *"label: codex-gpt-5.5-full-marketplace"* ]]
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

@test "eval runner writes generated runtime filter options for real generated runs" {
  fixture_bin="$(mktemp -d)"
  mkdir -p "$fixture_bin"
  cat >"$fixture_bin/promptfoo" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
cat evals/out/generated/runtime-options.json
SH
  chmod +x "$fixture_bin/promptfoo"

  run env PROMPTFOO_BIN="$fixture_bin/promptfoo" EVAL_CASE_FILTER=taskbranch "$RUNNER"

  rm -rf "$fixture_bin"
  rm -f "$ROOT/evals/out/generated/runtime-options.json"
  [ "$status" -eq 0 ]
  [[ "$output" == *'"caseFilter":"taskbranch"'* ]]
}

@test "eval runner fails when Codex marketplace has no plugin names" {
  fixture_root="$(mktemp -d)"
  mkdir -p "$fixture_root/scripts/evals" "$fixture_root/.agents/plugins"
  cp "$RUNNER" "$fixture_root/scripts/evals/run.sh"
  chmod +x "$fixture_root/scripts/evals/run.sh"
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
