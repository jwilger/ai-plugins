#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  TMPROOT="$(mktemp -d)"
  export JUST_TEMPDIR="$TMPROOT"
  mkdir -p "$TMPROOT/scripts/evals"
  cp "$ROOT/justfile" "$TMPROOT/justfile"
  for script in improve-plugins improve-evals; do
    cat >"$TMPROOT/scripts/evals/${script}.sh" <<SH
#!/usr/bin/env bash
echo ${script} >> loop-order.log
SH
    chmod +x "$TMPROOT/scripts/evals/${script}.sh"
  done
}

teardown() {
  rm -rf "$TMPROOT"
}

@test "just improve-plugins routes to plugin improvement loop only" {
  run just --justfile "$TMPROOT/justfile" --working-directory "$TMPROOT" improve-plugins

  [ "$status" -eq 0 ]
  [ "$(cat "$TMPROOT/loop-order.log")" = "improve-plugins" ]
}

@test "just improve-evals routes to eval improvement loop only" {
  run just --justfile "$TMPROOT/justfile" --working-directory "$TMPROOT" improve-evals

  [ "$status" -eq 0 ]
  [ "$(cat "$TMPROOT/loop-order.log")" = "improve-evals" ]
}
