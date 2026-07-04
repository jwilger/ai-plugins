#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  TMPROOT="$(mktemp -d)"
  mkdir -p "$TMPROOT/scripts/evals" "$TMPROOT/evals/out"
  cp "$ROOT/scripts/evals/build-site.mjs" "$TMPROOT/scripts/evals/build-site.mjs"
  cat >"$TMPROOT/evals/out/results.json" <<'JSON'
{
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
          "label": "codex-gpt-5.5"
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
          "label": "codex-gpt-5.5"
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
          "label": "codex-gpt-5.5"
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
          "label": "codex-gpt-5.5"
        },
        "gradingResult": {
          "pass": false,
          "score": 0,
          "reason": "zero"
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
  [ "$(jq '.total' "$TMPROOT/site/evals/summary.json")" = "4" ]
  [ "$(jq -r '.aggregates[] | select(.id == "fixture-pass") | .provider' "$TMPROOT/site/evals/summary.json")" = "codex-gpt-5.5" ]
  [ "$(jq '.aggregates[] | select(.id == "fixture-pass") | .passRate' "$TMPROOT/site/evals/summary.json")" = "0.6666666666666666" ]
  [ "$(jq '.aggregates[] | select(.id == "fixture-zero-defaults") | .samples[0].sampleIndex' "$TMPROOT/site/evals/summary.json")" = "0" ]
  [ "$(jq '.aggregates[] | select(.id == "fixture-zero-defaults") | .minPassRate' "$TMPROOT/site/evals/summary.json")" = "0" ]
  [ "$(jq '.aggregates[] | select(.id == "fixture-pass") | .thresholdMet' "$TMPROOT/site/evals/summary.json")" = "false" ]
  grep -q '"pluginSummaries"' "$TMPROOT/site/evals/summary.json"
  grep -q '"plugin": "agentic-systems-engineering"' "$TMPROOT/site/evals/summary.json"
  grep -q '"skillSummaries"' "$TMPROOT/site/evals/summary.json"
  grep -q '"skill": "evaluate-stochastic-systems"' "$TMPROOT/site/evals/summary.json"
  grep -q "fixture-pass" "$TMPROOT/site/evals/index.html"
  grep -q "codex-gpt-5.5" "$TMPROOT/site/evals/index.html"
  grep -q "66.7%" "$TMPROOT/site/evals/index.html"
  grep -q "Plugin summary" "$TMPROOT/site/evals/index.html"
  grep -q "Skill summary" "$TMPROOT/site/evals/index.html"
}
