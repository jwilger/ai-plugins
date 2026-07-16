#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  RUNNER="$ROOT/scripts/evals/run-code-quality-benchmark.sh"
  WORKSPACE_PREPARER="$ROOT/scripts/evals/prepare-code-quality-workspaces.mjs"
  CONTRACT_VALIDATOR="$ROOT/scripts/evals/validate-code-quality-contract.mjs"
  TEMP_ROOT="$(mktemp -d)"
}

teardown() {
  rm -rf "$TEMP_ROOT"
}

@test "code-quality benchmark dry-run plans an isolated three-mode Rust feature slice without writing" {
  work_root="$TEMP_ROOT/workspaces"
  home_root="$TEMP_ROOT/homes"
  out_root="$TEMP_ROOT/out"

  run env \
    CODE_QUALITY_WORK_ROOT="$work_root" \
    CODE_QUALITY_HOME_ROOT="$home_root" \
    CODE_QUALITY_OUT_ROOT="$out_root" \
    CODE_QUALITY_SAMPLES=1 \
    "$RUNNER" --dry-run --case rust-cli-feature

  [ "$status" -eq 0 ]
  [[ "$output" == *"rust-cli-feature/sample-1/no-plugins"* ]]
  [[ "$output" == *"rust-cli-feature/sample-1/targeted-plugins"* ]]
  [[ "$output" == *"rust-cli-feature/sample-1/full-marketplace"* ]]
  [ "$(printf '%s\n' "$output" | grep -c 'prepare-codex-home.mjs')" -eq 3 ]
  [[ "$output" == *"$home_root/no-plugins --plugin-mode no-plugins"* ]]
  [[ "$output" == *"$home_root/targeted-plugins --plugin-mode targeted-plugins --plugins advisor\,development-discipline\,engineering-standards"* ]]
  [[ "$output" == *"$home_root/full-marketplace --plugin-mode full-marketplace"* ]]
  [[ "$output" == *"openai-codex-sdk-no-plugins"* ]]
  [[ "$output" == *"openai-codex-sdk-targeted-plugins"* ]]
  [[ "$output" == *"openai-codex-sdk-full-marketplace"* ]]
  [[ "$output" == *"execution EVAL_CASE_FILTER=rust-cli-feature EVAL_SAMPLES=1"* ]]
  [[ "$output" == *"--filter-pattern rust-cli-feature"* ]]
  [[ "$output" == *"promotion gates disabled: diagnostic noncanonical run"* ]]
  [[ "$output" != *"gate targeted-overall"* ]]
  [[ "$output" == *"$out_root/results.json"* ]]
  [[ "$output" == *"check-code-quality-benchmark.mjs"* ]]
  [ ! -e "$work_root" ]
  [ ! -e "$home_root" ]
  [ ! -e "$out_root" ]
}

@test "code-quality benchmark rejects overlapping workspace and Codex-home roots before planning" {
  work_root="$TEMP_ROOT/workspaces"
  home_root="$work_root/rust-cli-feature/sample-1"

  run env \
    CODE_QUALITY_WORK_ROOT="$work_root" \
    CODE_QUALITY_HOME_ROOT="$home_root" \
    CODE_QUALITY_OUT_ROOT="$TEMP_ROOT/out" \
    CODE_QUALITY_SAMPLES=1 \
    "$RUNNER" --dry-run --case rust-cli-feature

  [ "$status" -eq 2 ]
  [[ "$output" == *"benchmark paths overlap"* ]]
  [[ "$output" != *"prepare-codex-home.mjs"* ]]
  [ ! -e "$work_root" ]
}

@test "code-quality benchmark recognizes root and delimiter characters in overlapping paths" {
  run env \
    CODE_QUALITY_WORK_ROOT=/ \
    CODE_QUALITY_HOME_ROOT=/rust-cli-feature/sample-1 \
    CODE_QUALITY_OUT_ROOT="$TEMP_ROOT/root-out" \
    CODE_QUALITY_SAMPLES=1 \
    "$RUNNER" --dry-run --case rust-cli-feature

  [ "$status" -eq 2 ]
  [[ "$output" == *"benchmark paths overlap"* ]]

  work_root="$TEMP_ROOT/work|spaces"
  run env \
    CODE_QUALITY_WORK_ROOT="$work_root" \
    CODE_QUALITY_HOME_ROOT="$work_root/rust-cli-feature/sample-1" \
    CODE_QUALITY_OUT_ROOT="$TEMP_ROOT/delimiter-out" \
    CODE_QUALITY_SAMPLES=1 \
    "$RUNNER" --dry-run --case rust-cli-feature

  [ "$status" -eq 2 ]
  [[ "$output" == *"benchmark paths overlap"* ]]
}

@test "code-quality benchmark default dry-run predeclares three task types by three modes by three samples" {
  run env \
    CODE_QUALITY_WORK_ROOT="$TEMP_ROOT/workspaces" \
    CODE_QUALITY_HOME_ROOT="$TEMP_ROOT/homes" \
    CODE_QUALITY_OUT_ROOT="$TEMP_ROOT/out" \
    "$RUNNER" --dry-run

  [ "$status" -eq 0 ]
  [ "$(printf '%s\n' "$output" | grep -c '^workspace ')" -eq 27 ]
  [[ "$output" == *"rust-cli-feature/sample-3/full-marketplace"* ]]
  [[ "$output" == *"stock-service-bugfix/sample-3/full-marketplace"* ]]
  [[ "$output" == *"stock-service-refactor/sample-3/full-marketplace"* ]]
  [[ "$output" == *"metric pass@3 capability"* ]]
  [[ "$output" == *"metric pass^3 reliability"* ]]
  [[ "$output" == *"gate targeted-overall 8/9"* ]]
  [[ "$output" == *"gate full-overall 7/9"* ]]
  [[ "$output" == *"gate targeted-lift 2/9"* ]]
  [[ "$output" == *"gate targeted-per-case-no-regression >=0"* ]]
}

@test "code-quality workspace preparation creates three clean standalone Rust fixture repositories with identical baselines" {
  work_root="$TEMP_ROOT/workspaces"

  run node "$WORKSPACE_PREPARER" "$work_root" \
    --case rust-cli-feature \
    --samples 1

  [ "$status" -eq 0 ]
  [ "$(jq '.workspaces | length' <<<"$output")" -eq 3 ]
  [ -f "$work_root/.ai-plugins-code-quality-work-root" ]

  baseline=""
  for mode in no-plugins targeted-plugins full-marketplace; do
    workspace="$work_root/rust-cli-feature/sample-1/$mode"
    [ -f "$workspace/Cargo.toml" ]
    [ -f "$workspace/Cargo.lock" ]
    [ -f "$workspace/src/main.rs" ]
    [ -f "$workspace/AGENTS.md" ]
    [ -x "$workspace/.git/hooks/pre-push" ]
    [ "$(git -C "$workspace" rev-parse --is-inside-work-tree)" = true ]
    [ -z "$(git -C "$workspace" remote)" ]
    [ -z "$(git -C "$workspace" status --porcelain)" ]
    current_baseline="$(git -C "$workspace" rev-parse HEAD)"
    if [ -z "$baseline" ]; then
      baseline="$current_baseline"
    else
      [ "$current_baseline" = "$baseline" ]
    fi
  done
}

@test "code-quality workspace preparation rejects a missing case selection before mutating the work root" {
  work_root="$TEMP_ROOT/workspaces"

  run node "$WORKSPACE_PREPARER" "$work_root" --samples 1

  [ "$status" -eq 2 ]
  [[ "$output" == *"--case is required until every declared fixture is available"* ]]
  [ ! -e "$work_root" ]
}

@test "code-quality contract validation rejects path traversal in condition, case, and fixture identifiers" {
  contract="$ROOT/evals/benchmarks/downstream-code-quality/benchmark.json"

  for mutation in condition case fixture; do
    candidate="$TEMP_ROOT/$mutation.json"
    case "$mutation" in
      condition)
        jq '.conditions[0].id = "../outside"' "$contract" >"$candidate"
        expected="invalid condition id"
        ;;
      case)
        jq '.cases[0].id = "../outside"' "$contract" >"$candidate"
        expected="invalid case id"
        ;;
      fixture)
        jq '.cases[0].fixture = "../../outside"' "$contract" >"$candidate"
        expected="invalid fixture id"
        ;;
    esac

    run node "$CONTRACT_VALIDATOR" "$candidate"

    [ "$status" -eq 2 ]
    [[ "$output" == *"$expected"* ]]
  done
}

@test "code-quality workspace preparation preserves symlink-marked and unowned nonempty roots" {
  symlink_root="$TEMP_ROOT/symlink-root"
  marker_target="$TEMP_ROOT/marker-target"
  mkdir "$symlink_root"
  printf 'ai-plugins downstream code-quality work root\n' >"$marker_target"
  ln -s "$marker_target" \
    "$symlink_root/.ai-plugins-code-quality-work-root"
  printf 'keep symlink root\n' >"$symlink_root/user-file"

  run node "$WORKSPACE_PREPARER" "$symlink_root" \
    --case rust-cli-feature \
    --samples 1

  [ "$status" -eq 2 ]
  [[ "$output" == *"ownership marker must be a regular file"* ]]
  grep -q 'keep symlink root' "$symlink_root/user-file"

  unowned_root="$TEMP_ROOT/unowned-root"
  mkdir "$unowned_root"
  printf 'keep unowned root\n' >"$unowned_root/user-file"

  run node "$WORKSPACE_PREPARER" "$unowned_root" \
    --case rust-cli-feature \
    --samples 1

  [ "$status" -eq 2 ]
  [[ "$output" == *"refusing to replace unowned workspace root"* ]]
  grep -q 'keep unowned root' "$unowned_root/user-file"
}

@test "code-quality workspace preparation refuses a concurrent invocation for the same root" {
  work_root="$TEMP_ROOT/workspaces"
  lock_hash="$(node -e 'const crypto = require("node:crypto"); process.stdout.write(crypto.createHash("sha256").update(process.argv[1]).digest("hex"));' "$(realpath -m "$work_root")")"
  lock_dir="$(node -p 'require("node:os").tmpdir()')/ai-plugins-code-quality-locks-$UID"
  lock_file="$lock_dir/$lock_hash.lock"
  mkdir -p "$lock_dir"
  exec 8>>"$lock_file"
  flock --nonblock 8

  run node "$WORKSPACE_PREPARER" "$work_root" \
    --case rust-cli-feature \
    --samples 1

  flock --unlock 8
  exec 8>&-
  rm -f "$lock_file"

  [ "$status" -eq 75 ]
  [[ "$output" == *"workspace preparation already active for root"* ]]
  [ ! -e "$work_root" ]
}

@test "code-quality workspace preparation preflights a selected fixture before replacing owned work" {
  work_root="$TEMP_ROOT/workspaces"
  node "$WORKSPACE_PREPARER" "$work_root" \
    --case rust-cli-feature \
    --samples 1 >/dev/null
  printf 'preserve prior work\n' >"$work_root/sentinel"

  run node "$WORKSPACE_PREPARER" "$work_root" \
    --case stock-service-bugfix \
    --samples 1

  [ "$status" -eq 2 ]
  [[ "$output" == *"missing benchmark fixture: stock-reservation-service-buggy"* ]]
  grep -q 'preserve prior work' "$work_root/sentinel"
  [ -d "$work_root/rust-cli-feature/sample-1/no-plugins/.git" ]
}
