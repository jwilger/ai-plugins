#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  TMPROOT="$(mktemp -d)"
  mkdir -p "$TMPROOT/scripts/evals" "$TMPROOT/evals/out"
  cp "$ROOT/scripts/evals/build-site.mjs" "$TMPROOT/scripts/evals/build-site.mjs"
  cp "$ROOT/scripts/evals/provider-compositions.mjs" "$TMPROOT/scripts/evals/provider-compositions.mjs"
  cat >"$TMPROOT/evals/out/results.json" <<'JSON'
{
  "config": {
    "providers": [
      {
        "id": "openai:codex-sdk",
        "label": "codex-gpt-5.6-terra-targeted-plugins"
      },
      {
        "id": "openai:codex-sdk",
        "label": "codex-gpt-5.6-terra-full-marketplace"
      },
      {
        "id": "openai:codex-sdk",
        "label": "codex-gpt-5.6-terra-no-plugins"
      }
    ],
    "metadata": {
      "providerCompositions": [
        {
          "label": "codex-gpt-5.6-terra-targeted-plugins",
          "provider": "openai:codex-sdk",
          "providerVariant": "codex-gpt-5.6-terra",
          "pluginMode": "targeted-plugins",
          "plugins": ["tiber"]
        },
        {
          "label": "codex-gpt-5.6-terra-full-marketplace",
          "provider": "openai:codex-sdk",
          "providerVariant": "codex-gpt-5.6-terra",
          "pluginMode": "full-marketplace",
          "plugins": ["advisor", "tiber"]
        },
        {
          "label": "codex-gpt-5.6-terra-no-plugins",
          "provider": "openai:codex-sdk",
          "providerVariant": "codex-gpt-5.6-terra",
          "pluginMode": "no-plugins",
          "plugins": []
        }
      ]
    }
  },
  "results": {
    "results": [
      {
        "description": "fixture-pass",
        "testCase": {
          "case_id": "fixture-pass",
          "behavior": "fixture behavior",
          "plugins": ["agentic-systems-engineering"],
          "skills": ["evaluate-stochastic-systems"],
          "sample_index": 1,
          "min_pass_rate": 0.67
        },
        "provider": {
          "label": "codex-gpt-5.6-terra"
        },
        "gradingResult": {
          "pass": true,
          "score": 1,
          "reason": "ok"
        }
      },
      {
        "description": "fixture-pass",
        "testCase": {
          "case_id": "fixture-pass",
          "behavior": "fixture behavior",
          "plugins": ["agentic-systems-engineering"],
          "skills": ["evaluate-stochastic-systems"],
          "sample_index": 2,
          "min_pass_rate": 0.67
        },
        "provider": {
          "label": "codex-gpt-5.6-terra"
        },
        "gradingResult": {
          "pass": false,
          "score": 0,
          "reason": "miss"
        }
      },
      {
        "description": "fixture-pass",
        "testCase": {
          "case_id": "fixture-pass",
          "behavior": "fixture behavior",
          "plugins": ["agentic-systems-engineering"],
          "skills": ["evaluate-stochastic-systems"],
          "sample_index": 3,
          "min_pass_rate": 0.67
        },
        "provider": {
          "label": "codex-gpt-5.6-terra"
        },
        "gradingResult": {
          "pass": true,
          "score": 1,
          "reason": "ok"
        }
      },
      {
        "description": "fixture-zero-defaults",
        "testCase": {
          "case_id": "fixture-zero-defaults",
          "behavior": "zero default fixture",
          "plugins": ["agentic-systems-engineering"],
          "skills": ["evaluate-stochastic-systems"],
          "sample_index": 0,
          "min_pass_rate": 0
        },
        "provider": {
          "label": "codex-gpt-5.6-terra"
        },
        "gradingResult": {
          "pass": false,
          "score": 0,
          "reason": "zero"
        }
      },
      {
        "description": "fixture-provider-limit",
        "testCase": {
          "case_id": "fixture-provider-limit",
          "behavior": "provider limit fixture",
          "plugins": ["agentic-systems-engineering"],
          "skills": ["evaluate-stochastic-systems"],
          "sample_index": 1,
          "min_pass_rate": 1
        },
        "provider": {
          "label": "claude-code-sonnet"
        },
        "gradingResult": {
          "pass": false,
          "score": 0,
          "reason": "Error calling Claude Agent SDK: weekly limit reached for this session"
        }
      },
      {
        "description": "fixture-auth-guidance-failure",
        "testCase": {
          "case_id": "fixture-auth-guidance-failure",
          "behavior": "normal failed rubric mentioning auth",
          "plugins": ["agentic-systems-engineering"],
          "skills": ["evaluate-stochastic-systems"],
          "sample_index": 1,
          "min_pass_rate": 1
        },
        "provider": {
          "label": "claude-code-sonnet"
        },
        "gradingResult": {
          "pass": false,
          "score": 0,
          "reason": "The answer discusses auth policy, but misses the required eval sampling guidance."
        }
      }
    ]
  }
}
JSON
}

teardown() {
  rm -rf "$TMPROOT"
}

@test "eval dashboard builder writes summary and index" {
  run node "$TMPROOT/scripts/evals/build-site.mjs"

  [ "$status" -eq 0 ]
  [ -f "$TMPROOT/site/evals/index.html" ]
  [ -f "$TMPROOT/site/evals/summary.json" ]
  [ "$(jq '.total' "$TMPROOT/site/evals/summary.json")" = "6" ]
  [ "$(jq '.blocked' "$TMPROOT/site/evals/summary.json")" = "1" ]
  [ "$(jq '.failed' "$TMPROOT/site/evals/summary.json")" = "3" ]
  [ "$(jq -r '.status.state' "$TMPROOT/site/evals/summary.json")" = "completed" ]
  [ "$(jq -r '.aggregates[] | select(.id == "fixture-pass") | .provider' "$TMPROOT/site/evals/summary.json")" = "codex-gpt-5.6-terra" ]
  [ "$(jq '.aggregates[] | select(.id == "fixture-pass") | .passRate' "$TMPROOT/site/evals/summary.json")" = "0.6666666666666666" ]
  [ "$(jq '.aggregates[] | select(.id == "fixture-zero-defaults") | .samples[0].sampleIndex' "$TMPROOT/site/evals/summary.json")" = "0" ]
  [ "$(jq '.aggregates[] | select(.id == "fixture-zero-defaults") | .minPassRate' "$TMPROOT/site/evals/summary.json")" = "0" ]
  [ "$(jq '.aggregates[] | select(.id == "fixture-pass") | .thresholdMet' "$TMPROOT/site/evals/summary.json")" = "false" ]
  [ "$(jq -r '.aggregates[] | select(.id == "fixture-provider-limit") | .status' "$TMPROOT/site/evals/summary.json")" = "blocked" ]
  [ "$(jq '.aggregates[] | select(.id == "fixture-provider-limit") | .blocked' "$TMPROOT/site/evals/summary.json")" = "1" ]
  [ "$(jq -r '.aggregates[] | select(.id == "fixture-auth-guidance-failure") | .status' "$TMPROOT/site/evals/summary.json")" = "fail" ]
  [ "$(jq '.thresholdsBlocked' "$TMPROOT/site/evals/summary.json")" = "1" ]
  grep -q '"pluginSummaries"' "$TMPROOT/site/evals/summary.json"
  grep -q '"plugin": "agentic-systems-engineering"' "$TMPROOT/site/evals/summary.json"
  grep -q '"skillSummaries"' "$TMPROOT/site/evals/summary.json"
  grep -q '"skill": "evaluate-stochastic-systems"' "$TMPROOT/site/evals/summary.json"
  [ "$(jq -r '.providerCompositionStatus.state' "$TMPROOT/site/evals/summary.json")" = "available" ]
  [ "$(jq -c '.providerCompositions[] | select(.pluginMode == "targeted-plugins") | .plugins' "$TMPROOT/site/evals/summary.json")" = '["tiber"]' ]
  [ "$(jq -c '.providerCompositions[] | select(.pluginMode == "full-marketplace") | .plugins' "$TMPROOT/site/evals/summary.json")" = '["advisor","tiber"]' ]
  [ "$(jq -c '.providerCompositions[] | select(.pluginMode == "no-plugins") | .plugins' "$TMPROOT/site/evals/summary.json")" = '[]' ]
  grep -q "fixture-pass" "$TMPROOT/site/evals/index.html"
  grep -q "codex-gpt-5.6-terra" "$TMPROOT/site/evals/index.html"
  grep -q "66.7%" "$TMPROOT/site/evals/index.html"
  grep -q "fixture-provider-limit" "$TMPROOT/site/evals/index.html"
  grep -q "blocked" "$TMPROOT/site/evals/index.html"
  grep -q "Installed provider composition" "$TMPROOT/site/evals/index.html"
  grep -q "Case-target plugin summary" "$TMPROOT/site/evals/index.html"
  grep -q "codex-gpt-5.6-terra-targeted-plugins" "$TMPROOT/site/evals/index.html"
  grep -q ">tiber<" "$TMPROOT/site/evals/index.html"
  grep -q ">None<" "$TMPROOT/site/evals/index.html"
  grep -q "Skill summary" "$TMPROOT/site/evals/index.html"
}

@test "eval dashboard marks legacy composition provenance unavailable instead of empty" {
  legacy_results="$TMPROOT/evals/out/results.legacy.json"
  jq 'del(.config.metadata.providerCompositions)' \
    "$TMPROOT/evals/out/results.json" >"$legacy_results"
  mv "$legacy_results" "$TMPROOT/evals/out/results.json"

  run node "$TMPROOT/scripts/evals/build-site.mjs"

  [ "$status" -eq 0 ]
  [ "$(jq -r '.providerCompositionStatus.state' "$TMPROOT/site/evals/summary.json")" = "unavailable" ]
  [ "$(jq '.providerCompositions | length' "$TMPROOT/site/evals/summary.json")" = "0" ]
  grep -q "Installed provider composition metadata is unavailable for this artifact" "$TMPROOT/site/evals/index.html"
}

@test "eval dashboard rejects present but semantically invalid composition provenance" {
  original_results="$TMPROOT/evals/out/results.original.json"
  cp "$TMPROOT/evals/out/results.json" "$original_results"

  for composition_case in \
    explicit_empty \
    targeted_empty \
    no_plugins_nonempty \
    unknown_provider \
    unknown_mode \
    label_mismatch \
    duplicate_label \
    duplicate_plugin \
    unsorted_plugins \
    invalid_plugin_name \
    inconsistent_codex_mode_set \
    missing_composition \
    extra_composition \
    missing_configured_providers; do
    node - \
      "$original_results" \
      "$TMPROOT/evals/out/results.json" \
      "$composition_case" <<'NODE'
const fs = require("node:fs");

const artifact = JSON.parse(fs.readFileSync(process.argv[2], "utf8"));
const output = process.argv[3];
const compositionCase = process.argv[4];
const compositions = artifact.config.metadata.providerCompositions;
const byMode = (mode) =>
  compositions.find((composition) => composition.pluginMode === mode);

switch (compositionCase) {
  case "explicit_empty":
    artifact.config.metadata.providerCompositions = [];
    break;
  case "targeted_empty":
    byMode("targeted-plugins").plugins = [];
    break;
  case "no_plugins_nonempty":
    byMode("no-plugins").plugins = ["tiber"];
    break;
  case "unknown_provider":
    byMode("targeted-plugins").provider = "unknown:provider";
    break;
  case "unknown_mode": {
    const targeted = byMode("targeted-plugins");
    targeted.pluginMode = "unknown-mode";
    targeted.label = `${targeted.providerVariant}-unknown-mode`;
    break;
  }
  case "label_mismatch":
    byMode("targeted-plugins").label = "mismatched-label";
    break;
  case "duplicate_label":
    compositions.push({ ...byMode("targeted-plugins") });
    break;
  case "duplicate_plugin":
    byMode("targeted-plugins").plugins = ["tiber", "tiber"];
    break;
  case "unsorted_plugins":
    byMode("full-marketplace").plugins = ["tiber", "advisor"];
    break;
  case "invalid_plugin_name":
    byMode("targeted-plugins").plugins = ["Tiber"];
    break;
  case "inconsistent_codex_mode_set": {
    const targeted = byMode("targeted-plugins");
    compositions.push({
      ...targeted,
      label: "codex-second-targeted-plugins",
      providerVariant: "codex-second",
      plugins: ["advisor"],
    });
    break;
  }
  case "missing_composition":
    artifact.config.metadata.providerCompositions = compositions.slice(0, -1);
    break;
  case "extra_composition":
    compositions.push({
      label: "claude-extra-full-marketplace",
      provider: "anthropic:claude-agent-sdk",
      providerVariant: "claude-extra",
      pluginMode: "full-marketplace",
      plugins: ["advisor"],
    });
    break;
  case "missing_configured_providers":
    delete artifact.config.providers;
    break;
  default:
    throw new Error(`unknown composition case: ${compositionCase}`);
}

fs.writeFileSync(output, JSON.stringify(artifact));
NODE

    run node "$TMPROOT/scripts/evals/build-site.mjs"

    [ "$status" -eq 0 ]
    [ "$(jq -r '.providerCompositionStatus.state' "$TMPROOT/site/evals/summary.json")" = "invalid" ]
    [ "$(jq '.providerCompositions | length' "$TMPROOT/site/evals/summary.json")" = "0" ]
    grep -q "Installed provider composition metadata is invalid for this artifact" "$TMPROOT/site/evals/index.html"
  done
}

@test "eval dashboard builder surfaces skipped provider status" {
  rm "$TMPROOT/evals/out/results.json"
  cat >"$TMPROOT/evals/out/status.json" <<'JSON'
{
  "generatedAt": "2026-07-04T00:00:00.000Z",
  "suite": "agentic-systems-engineering",
  "state": "skipped",
  "reason": "Provider-backed evals were not run because credentials are missing.",
  "providerCredentials": "missing"
}
JSON

  run node "$TMPROOT/scripts/evals/build-site.mjs"

  [ "$status" -eq 0 ]
  [ "$(jq '.total' "$TMPROOT/site/evals/summary.json")" = "0" ]
  [ "$(jq -r '.status.state' "$TMPROOT/site/evals/summary.json")" = "skipped" ]
  [ "$(jq -r '.status.providerCredentials' "$TMPROOT/site/evals/summary.json")" = "missing" ]
  grep -q "Provider-backed evals were not run" "$TMPROOT/site/evals/index.html"
  grep -q "No eval samples are available" "$TMPROOT/site/evals/index.html"
}

@test "eval dashboard builder falls back when status file is malformed" {
  printf '{not-json' >"$TMPROOT/evals/out/status.json"

  run node "$TMPROOT/scripts/evals/build-site.mjs"

  [ "$status" -eq 0 ]
  [ "$(jq '.total' "$TMPROOT/site/evals/summary.json")" = "6" ]
  [ "$(jq -r '.status.state' "$TMPROOT/site/evals/summary.json")" = "completed" ]
  grep -q "Promptfoo results were found and summarized" "$TMPROOT/site/evals/index.html"
}
