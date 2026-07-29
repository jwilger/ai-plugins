#!/usr/bin/env bats

setup() {
  REPO_ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  TEST_ROOT="$(mktemp -d)"
  export REPO_ROOT TEST_ROOT
  cat >"$TEST_ROOT/cargo" <<'SCRIPT'
#!/usr/bin/env bash
printf '%s\n' "$@" >"$TEST_ROOT/cargo.args"
SCRIPT
  chmod +x "$TEST_ROOT/cargo"
}

teardown() {
  rm -rf -- "$TEST_ROOT"
}

@test "development-discipline stateful tests serialize their shared durable store" {
  run env PATH="$TEST_ROOT:$PATH" just --justfile "$REPO_ROOT/justfile" development-discipline-rust
  [ "$status" -eq 0 ]

  mapfile -t args <"$TEST_ROOT/cargo.args"
  [ "${args[0]}" = "test" ]
  [ "${args[1]}" = "--manifest-path" ]
  [ "${args[3]}" = "--" ]
  [ "${args[4]}" = "--test-threads=1" ]
}

@test "tiber mutation gate serializes mutants and gives cold runs deterministic headroom" {
  run env PATH="$TEST_ROOT:$PATH" just --justfile "$REPO_ROOT/justfile" tiber-mutants
  [ "$status" -eq 0 ]

  mapfile -t args <"$TEST_ROOT/cargo.args"
  [ "${args[0]}" = "mutants" ]
  [ "${args[1]}" = "--jobs" ]
  [ "${args[2]}" = "1" ]
  [ "${args[3]}" = "--minimum-test-timeout" ]
  [ "${args[4]}" = "60" ]
}
