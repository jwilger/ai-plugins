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
          "behavior": "fixture behavior"
        },
        "gradingResult": {
          "pass": true,
          "score": 1,
          "reason": "ok"
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
  grep -q '"total": 1' "$TMPROOT/site/evals/summary.json"
  grep -q "fixture-pass" "$TMPROOT/site/evals/index.html"
}
