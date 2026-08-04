#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd -P)"
}

@test "pre-commit runs every non-expensive deterministic local check" {
  run just --justfile "$ROOT/justfile" --dry-run pre-commit

  [ "$status" -eq 0 ]
  [[ "$output" == *"prettier --check \$(git ls-files --cached --others --exclude-standard -- '*.json' '*.md')"* ]]
  [[ "$output" == *'scripts/check-development-system-rust.sh'* ]]
  [[ "$output" == *"bats \$(find plugins scripts -name '*.bats'"* ]]
  [[ "$output" == *"! -path 'scripts/tests/evals-code-quality-*.bats'"* ]]
  [[ "$output" == *"! -path 'scripts/tests/development-discipline-release-integration.bats'"* ]]
  [[ "$output" != *'check-development-discipline-release-from-source.sh'* ]]
  [[ "$output" != *'bats scripts/tests/evals-code-quality-'* ]]
}

@test "pre-commit runs the Rust quality gate for every shipped component" {
  run just --justfile "$ROOT/justfile" --dry-run pre-commit

  [ "$status" -eq 0 ]
  [[ "$output" == *'scripts/check-development-system-rust.sh'* ]]

  run "$ROOT/scripts/check-development-system-rust.sh" --list-manifests

  [ "$status" -eq 0 ]
  [[ "$output" == *'components/development-discipline/rust/Cargo.toml'* ]]
  [[ "$output" == *'components/tiber/rust/Cargo.toml'* ]]
}

@test "the component Rust gate runs formatting, strict Clippy, and tests for each manifest" {
  local fake_bin="$BATS_TEST_TMPDIR/fake-bin"
  local invocation_log="$BATS_TEST_TMPDIR/cargo-invocations"

  mkdir -p "$fake_bin"
  cat >"$fake_bin/cargo" <<'SH'
#!/usr/bin/env bash
printf '%s\n' "$*" >>"$CARGO_INVOCATION_LOG"
SH
  chmod +x "$fake_bin/cargo"

  run env \
    PATH="$fake_bin:$PATH" \
    CARGO_INVOCATION_LOG="$invocation_log" \
    "$ROOT/scripts/check-development-system-rust.sh"

  [ "$status" -eq 0 ]
  [ "$(wc -l <"$invocation_log")" -eq 6 ]
  grep -Fq 'fmt --manifest-path plugins/development-system/components/development-discipline/rust/Cargo.toml --all --check' "$invocation_log"
  grep -Fq 'clippy --manifest-path plugins/development-system/components/development-discipline/rust/Cargo.toml --all-targets -- -D warnings' "$invocation_log"
  grep -Fq 'test --manifest-path plugins/development-system/components/development-discipline/rust/Cargo.toml -- --test-threads=1' "$invocation_log"
  grep -Fq 'fmt --manifest-path plugins/development-system/components/tiber/rust/Cargo.toml --all --check' "$invocation_log"
  grep -Fq 'clippy --manifest-path plugins/development-system/components/tiber/rust/Cargo.toml --all-targets -- -D warnings' "$invocation_log"
  grep -Fq 'test --manifest-path plugins/development-system/components/tiber/rust/Cargo.toml -- --test-threads=1' "$invocation_log"
}

@test "full CI retains the explicitly expensive integration checks" {
  run just --justfile "$ROOT/justfile" --dry-run ci

  [ "$status" -eq 0 ]
  [[ "$output" == *"find scripts/tests -name 'evals-code-quality-*.bats'"* ]]
  [[ "$output" == *'development-discipline-release-integration.bats'* ]]
}

@test "the managed pre-commit hook invokes the canonical gate" {
  grep -Eq '^pre-commit:' "$ROOT/lefthook.yml"
  grep -Fq 'scripts/pre-commit-gate.sh' "$ROOT/lefthook.yml"
}

@test "the pre-commit gate does not leak Git hook repository variables into checks" {
  local fixture_root="$BATS_TEST_TMPDIR/fixture"
  local fake_bin="$BATS_TEST_TMPDIR/fake-bin"
  local captured="$BATS_TEST_TMPDIR/environment"

  mkdir -p "$fixture_root" "$fake_bin"
  cat >"$fake_bin/git" <<'SH'
#!/usr/bin/env bash
case "$*" in
  'rev-parse --show-toplevel') printf '%s\n' "$FAKE_REPO_ROOT" ;;
  'diff --quiet') exit 0 ;;
  *) exit 1 ;;
esac
SH
  cat >"$fake_bin/nix" <<'SH'
#!/usr/bin/env bash
env >"$CAPTURED_ENVIRONMENT"
SH
  chmod +x "$fake_bin/git" "$fake_bin/nix"

  run env \
    PATH="$fake_bin:$PATH" \
    FAKE_REPO_ROOT="$fixture_root" \
    CAPTURED_ENVIRONMENT="$captured" \
    GIT_DIR="$fixture_root/.git" \
    GIT_WORK_TREE="$fixture_root" \
    GIT_INDEX_FILE="$fixture_root/.git/index" \
    "$ROOT/scripts/pre-commit-gate.sh"

  [ "$status" -eq 0 ]
  ! grep -Eq '^GIT_(DIR|WORK_TREE|INDEX_FILE)=' "$captured"
}
