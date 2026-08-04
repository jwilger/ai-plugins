#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd -P)"
}

@test "pre-commit runs every non-expensive deterministic local check" {
  run just --justfile "$ROOT/justfile" --dry-run pre-commit

  [ "$status" -eq 0 ]
  [[ "$output" == *'prettier --check'* ]]
  [[ "$output" == *'cargo fmt'* ]]
  [[ "$output" == *'cargo clippy'* ]]
  [[ "$output" == *'cargo test'* ]]
  [[ "$output" == *'bats '* ]]
  [[ "$output" != *'check-development-discipline-release-from-source.sh'* ]]
  [[ "$output" != *'scripts/evals/'* ]]
}

@test "the managed pre-commit hook invokes the canonical gate" {
  grep -Eq '^pre-commit:' "$ROOT/lefthook.yml"
  grep -Fq 'scripts/pre-commit-gate.sh' "$ROOT/lefthook.yml"
}
