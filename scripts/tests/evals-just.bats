#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  TMPROOT="$(mktemp -d)"
  export JUST_TEMPDIR="$TMPROOT"
  mkdir -p "$TMPROOT/scripts/evals"
  cp "$ROOT/justfile" "$TMPROOT/justfile"
  cat >"$TMPROOT/scripts/evals/run-changed.sh" <<'SH'
#!/usr/bin/env bash
printf 'run:%s\n' "$PROMPTFOO_MAX_CONCURRENCY" >> eval-order.log
mkdir -p evals/out
: > evals/out/results.json
SH
  cat >"$TMPROOT/scripts/evals/share.sh" <<'SH'
#!/usr/bin/env bash
echo share >> eval-order.log
echo "Promptfoo share URL: https://promptfoo.example/eval/abc123"
SH
  cat >"$TMPROOT/scripts/evals/run.sh" <<'SH'
#!/usr/bin/env bash
printf 'all:%s\n' "$PROMPTFOO_MAX_CONCURRENCY" >> eval-order.log
SH
  chmod +x "$TMPROOT/scripts/evals/run-changed.sh" "$TMPROOT/scripts/evals/share.sh" "$TMPROOT/scripts/evals/run.sh"
}

teardown() {
  rm -rf "$TMPROOT"
}

@test "just evals runs provider evals then shares the report url" {
  run just --justfile "$TMPROOT/justfile" --working-directory "$TMPROOT" evals

  [ "$status" -eq 0 ]
  [ "$(cat "$TMPROOT/eval-order.log")" = $'run:8\nshare' ]
  [[ "$output" == *"Promptfoo share URL: https://promptfoo.example/eval/abc123"* ]]
}

@test "just evals-all uses one provider process with the global eight-call cap" {
  run just --justfile "$TMPROOT/justfile" --working-directory "$TMPROOT" evals-all

  [ "$status" -eq 0 ]
  [ "$(cat "$TMPROOT/eval-order.log")" = "all:8" ]
}

@test "just evals shares the report before returning a failed eval status" {
  cat >"$TMPROOT/scripts/evals/run-changed.sh" <<'SH'
#!/usr/bin/env bash
printf 'run:%s\n' "$PROMPTFOO_MAX_CONCURRENCY" >> eval-order.log
mkdir -p evals/out
: > evals/out/results.json
exit 100
SH
  chmod +x "$TMPROOT/scripts/evals/run-changed.sh"

  run just --justfile "$TMPROOT/justfile" --working-directory "$TMPROOT" evals

  [ "$status" -eq 100 ]
  [ "$(cat "$TMPROOT/eval-order.log")" = $'run:8\nshare' ]
  [[ "$output" == *"Promptfoo share URL: https://promptfoo.example/eval/abc123"* ]]
}

@test "just evals skips share when a failed run produced no fresh artifacts" {
  cat >"$TMPROOT/scripts/evals/run-changed.sh" <<'SH'
#!/usr/bin/env bash
printf 'run:%s\n' "$PROMPTFOO_MAX_CONCURRENCY" >> eval-order.log
exit 100
SH
  chmod +x "$TMPROOT/scripts/evals/run-changed.sh"

  run just --justfile "$TMPROOT/justfile" --working-directory "$TMPROOT" evals

  [ "$status" -eq 100 ]
  [ "$(cat "$TMPROOT/eval-order.log")" = "run:8" ]
  [[ "$output" == *"Skipping promptfoo share because no fresh eval artifacts were generated."* ]]
}

@test "just evals does not share after user interrupt" {
  cat >"$TMPROOT/scripts/evals/run-changed.sh" <<'SH'
#!/usr/bin/env bash
printf 'run:%s\n' "$PROMPTFOO_MAX_CONCURRENCY" >> eval-order.log
exit 130
SH
  chmod +x "$TMPROOT/scripts/evals/run-changed.sh"

  run just --justfile "$TMPROOT/justfile" --working-directory "$TMPROOT" evals

  [ "$status" -eq 130 ]
  [ "$(cat "$TMPROOT/eval-order.log")" = "run:8" ]
}

@test "just evals does not share after eval timeout" {
  cat >"$TMPROOT/scripts/evals/run-changed.sh" <<'SH'
#!/usr/bin/env bash
printf 'run:%s\n' "$PROMPTFOO_MAX_CONCURRENCY" >> eval-order.log
mkdir -p evals/out
: > evals/out/results.json
exit 124
SH
  chmod +x "$TMPROOT/scripts/evals/run-changed.sh"

  run just --justfile "$TMPROOT/justfile" --working-directory "$TMPROOT" evals

  [ "$status" -eq 124 ]
  [ "$(cat "$TMPROOT/eval-order.log")" = "run:8" ]
}
