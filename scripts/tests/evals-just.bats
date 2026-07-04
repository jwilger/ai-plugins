#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  TMPROOT="$(mktemp -d)"
  mkdir -p "$TMPROOT/scripts/evals"
  cp "$ROOT/justfile" "$TMPROOT/justfile"
  cat >"$TMPROOT/scripts/evals/run.sh" <<'SH'
#!/usr/bin/env bash
echo run >> eval-order.log
mkdir -p evals/out
: > evals/out/results.json
SH
  cat >"$TMPROOT/scripts/evals/share.sh" <<'SH'
#!/usr/bin/env bash
echo share >> eval-order.log
echo "Promptfoo share URL: https://promptfoo.example/eval/abc123"
SH
  chmod +x "$TMPROOT/scripts/evals/run.sh" "$TMPROOT/scripts/evals/share.sh"
}

teardown() {
  rm -rf "$TMPROOT"
}

@test "just evals runs provider evals then shares the report url" {
  run just --justfile "$TMPROOT/justfile" --working-directory "$TMPROOT" evals

  [ "$status" -eq 0 ]
  [ "$(cat "$TMPROOT/eval-order.log")" = $'run\nshare' ]
  [[ "$output" == *"Promptfoo share URL: https://promptfoo.example/eval/abc123"* ]]
}

@test "just evals shares the report before returning a failed eval status" {
  cat >"$TMPROOT/scripts/evals/run.sh" <<'SH'
#!/usr/bin/env bash
echo run >> eval-order.log
mkdir -p evals/out
: > evals/out/results.json
exit 100
SH
  chmod +x "$TMPROOT/scripts/evals/run.sh"

  run just --justfile "$TMPROOT/justfile" --working-directory "$TMPROOT" evals

  [ "$status" -eq 100 ]
  [ "$(cat "$TMPROOT/eval-order.log")" = $'run\nshare' ]
  [[ "$output" == *"Promptfoo share URL: https://promptfoo.example/eval/abc123"* ]]
}

@test "just evals skips share when a failed run produced no fresh artifacts" {
  cat >"$TMPROOT/scripts/evals/run.sh" <<'SH'
#!/usr/bin/env bash
echo run >> eval-order.log
exit 100
SH
  chmod +x "$TMPROOT/scripts/evals/run.sh"

  run just --justfile "$TMPROOT/justfile" --working-directory "$TMPROOT" evals

  [ "$status" -eq 100 ]
  [ "$(cat "$TMPROOT/eval-order.log")" = "run" ]
  [[ "$output" == *"Skipping promptfoo share because no fresh eval artifacts were generated."* ]]
}

@test "just evals does not share after user interrupt" {
  cat >"$TMPROOT/scripts/evals/run.sh" <<'SH'
#!/usr/bin/env bash
echo run >> eval-order.log
exit 130
SH
  chmod +x "$TMPROOT/scripts/evals/run.sh"

  run just --justfile "$TMPROOT/justfile" --working-directory "$TMPROOT" evals

  [ "$status" -eq 130 ]
  [ "$(cat "$TMPROOT/eval-order.log")" = "run" ]
}
