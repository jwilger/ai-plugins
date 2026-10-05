#!/usr/bin/env bats

setup() {
  PROJECT="$BATS_TEST_TMPDIR/project"
  PARENT="$BATS_TEST_TMPDIR/parent"
  MOCK_BIN="$BATS_TEST_TMPDIR/bin"
  mkdir -p "$PROJECT/scripts" "$PROJECT/tiber/scripts" "$PARENT" "$MOCK_BIN"
  git -C "$PARENT" init -q
  printf 'parent source\n' > "$PARENT/seed.txt"
  git -C "$PARENT" add seed.txt
  cp "$PARENT/.git/config" "$BATS_TEST_TMPDIR/parent.config"
  cp "$PARENT/.git/index" "$BATS_TEST_TMPDIR/parent.index"
  cp "$BATS_TEST_DIRNAME/../run-pre-commit-gate.sh" "$PROJECT/scripts/"
  printf '#!/usr/bin/env bash\nexit 0\n' > "$PROJECT/tiber/scripts/check-lint-policy.sh"
  export GATE_CALLS="$BATS_TEST_TMPDIR/calls"
  export GATE_FIXTURE_ROOT="$BATS_TEST_TMPDIR/fixture.git"
  export GATE_FIXTURE_WORKTREE="$BATS_TEST_TMPDIR/fixture-source"
  cat > "$MOCK_BIN/just" <<'SH'
#!/usr/bin/env bash
printf 'just %s\n' "$*" >> "$GATE_CALLS"
SH
  cat > "$MOCK_BIN/cargo" <<'SH'
#!/usr/bin/env bash
printf 'cargo %s\n' "$*" >> "$GATE_CALLS"
[[ "${GATE_FAIL_STATUS:-0}" == 0 ]] || exit "$GATE_FAIL_STATUS"
if [[ ! -d "$GATE_FIXTURE_ROOT" ]]; then
  git init --bare -q "$GATE_FIXTURE_ROOT"
  mkdir -p "$GATE_FIXTURE_WORKTREE"
  printf 'fixture source\n' > "$GATE_FIXTURE_WORKTREE/evidence.txt"
  git --git-dir="$GATE_FIXTURE_ROOT" --work-tree="$GATE_FIXTURE_WORKTREE" add evidence.txt
fi
[[ "${SSH_AUTH_SOCK:-}" == /test/signing-agent ]] || exit 48
[[ "${CARGO_TARGET_DIR:-}" == /test/build-cache* ]] || exit 49
SH
  chmod +x "$MOCK_BIN/just" "$MOCK_BIN/cargo"
}

run_gate() {
  run env PATH="$MOCK_BIN:$PATH" \
    GIT_DIR="$PARENT/.git" GIT_COMMON_DIR="$PARENT/.git" \
    GIT_WORK_TREE="$PARENT" GIT_INDEX_FILE="$PARENT/.git/index" \
    GIT_OBJECT_DIRECTORY="$PARENT/.git/objects" \
    SSH_AUTH_SOCK=/test/signing-agent CARGO_TARGET_DIR=/test/build-cache \
    bash "$PROJECT/scripts/run-pre-commit-gate.sh"
}

@test "fixture Git commands cannot alter the parent hook repository" {
  run_gate
  [ "$status" -eq 0 ]
  cmp "$PARENT/.git/config" "$BATS_TEST_TMPDIR/parent.config"
  cmp "$PARENT/.git/index" "$BATS_TEST_TMPDIR/parent.index"
  [ "$(git --git-dir="$GATE_FIXTURE_ROOT" config core.bare)" = true ]
  [ "$(head -n 1 "$GATE_CALLS")" = 'just validate-marketplace github-actions pi-extension' ]
  [[ "$(cat "$GATE_CALLS")" == *'--bin development-discipline-mcp -- --test-threads=1'* ]]
}

@test "an actual gate failure stops later checks and retains parent metadata" {
  export GATE_FAIL_STATUS=47
  run_gate
  [ "$status" -eq 47 ]
  [ "$(wc -l < "$GATE_CALLS")" -eq 2 ]
  cmp "$PARENT/.git/config" "$BATS_TEST_TMPDIR/parent.config"
  cmp "$PARENT/.git/index" "$BATS_TEST_TMPDIR/parent.index"
}
