#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  RUNNER="$ROOT/scripts/evals/run-code-quality-benchmark.sh"
  WORKSPACE_PREPARER="$ROOT/scripts/evals/prepare-code-quality-workspaces.mjs"
  CONTRACT_VALIDATOR="$ROOT/scripts/evals/validate-code-quality-contract.mjs"
  EXPENSE_VERIFIER="$ROOT/evals/benchmarks/downstream-code-quality/verifiers/expense-report.mjs"
  TEMP_ROOT="$(mktemp -d)"
}

teardown() {
  if [ -n "${HOST_SERVER_PID:-}" ]; then
    kill "$HOST_SERVER_PID" 2>/dev/null || true
    wait "$HOST_SERVER_PID" 2>/dev/null || true
  fi
  rm -rf "$TEMP_ROOT"
}

mark_benchmark_workspace() {
  local workspace="$1"
  git -C "$workspace" init --quiet --initial-branch=main
  printf 'ai-plugins downstream code-quality workspace\n' \
    >"$workspace/.git/.ai-plugins-code-quality-workspace"
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
  [[ "$output" == *"rust-cli-feature/sample-1/no-skills"* ]]
  [[ "$output" == *"rust-cli-feature/sample-1/targeted-quality-skills"* ]]
  [[ "$output" == *"rust-cli-feature/sample-1/all-marketplace-skills"* ]]
  [ "$(printf '%s\n' "$output" | grep -c 'prepare-codex-home.mjs')" -eq 3 ]
  [[ "$output" == *"$home_root/no-skills --plugin-mode no-plugins"* ]]
  [[ "$output" == *"$home_root/targeted-quality-skills --plugin-mode skills-only-marketplace --plugins advisor\,development-discipline\,engineering-standards"* ]]
  [[ "$output" == *"$home_root/all-marketplace-skills --plugin-mode skills-only-marketplace"* ]]
  [[ "$output" == *"openai-codex-sdk-no-skills"* ]]
  [[ "$output" == *"openai-codex-sdk-targeted-quality-skills"* ]]
  [[ "$output" == *"openai-codex-sdk-all-marketplace-skills"* ]]
  [[ "$output" == *"execution EVAL_CASE_FILTER=rust-cli-feature EVAL_SAMPLES=1"* ]]
  [[ "$output" == *"--filter-pattern rust-cli-feature"* ]]
  [[ "$output" == *"diagnostic gates disabled: noncanonical run"* ]]
  [[ "$output" != *"gate complete-runs"* ]]
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

@test "code-quality benchmark default dry-run predeclares a nine-turn non-promotional skills diagnostic" {
  run env \
    CODE_QUALITY_WORK_ROOT="$TEMP_ROOT/workspaces" \
    CODE_QUALITY_HOME_ROOT="$TEMP_ROOT/homes" \
    CODE_QUALITY_OUT_ROOT="$TEMP_ROOT/out" \
    "$RUNNER" --dry-run

  [ "$status" -eq 0 ]
  [ "$(printf '%s\n' "$output" | grep -c '^workspace ')" -eq 9 ]
  [[ "$output" == *"rust-cli-feature/sample-3/no-skills"* ]]
  [[ "$output" == *"rust-cli-feature/sample-3/targeted-quality-skills"* ]]
  [[ "$output" == *"rust-cli-feature/sample-3/all-marketplace-skills"* ]]
  [[ "$output" != *"stock-service-"* ]]
  [[ "$output" == *"metric pass@3 capability"* ]]
  [[ "$output" == *"metric pass^3 reliability"* ]]
  [[ "$output" == *"claim non-promotional"* ]]
  [[ "$output" == *"gate complete-runs 9/9"* ]]
  [[ "$output" == *"gate operational-errors 0"* ]]
  [[ "$output" == *"gate provenance-errors 0"* ]]
  [[ "$output" == *"gate safety-failures 0"* ]]
}

@test "code-quality benchmark reduced-sample dry-run does not claim canonical diagnostic gates" {
  run env \
    CODE_QUALITY_WORK_ROOT="$TEMP_ROOT/workspaces" \
    CODE_QUALITY_HOME_ROOT="$TEMP_ROOT/homes" \
    CODE_QUALITY_OUT_ROOT="$TEMP_ROOT/out" \
    CODE_QUALITY_SAMPLES=1 \
    "$RUNNER" --dry-run --case rust-cli-feature

  [ "$status" -eq 0 ]
  [ "$(printf '%s\n' "$output" | grep -c '^workspace ')" -eq 3 ]
  [[ "$output" == *"diagnostic gates disabled: noncanonical run"* ]]
  [[ "$output" != *"gate complete-runs"* ]]
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
  for mode in no-skills targeted-quality-skills all-marketplace-skills; do
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

@test "code-quality contract validation rejects drift from the non-promotional diagnostic invariants" {
  contract="$ROOT/evals/benchmarks/downstream-code-quality/benchmark.json"

  while IFS=$'\t' read -r name mutation expected; do
    candidate="$TEMP_ROOT/diagnostic-$name.json"
    jq "$mutation" "$contract" >"$candidate"

    run node "$CONTRACT_VALIDATOR" "$candidate"

    [ "$status" -eq 2 ]
    [[ "$output" == *"$expected"* ]]
  done <<'CASES'
samples	.sampleCount = 2	sampleCount must be exactly 3
targeted	.conditions[1].plugins = ["development-discipline"]	targeted-quality-skills plugins must be exactly
auth	.provider.authentication = "copied-oauth-file"	provider authentication must be dedicated-api-key-only
cases	.cases += [.cases[0]]	duplicate case id: rust-cli-feature
task	.cases[0].taskType = "refactor"	rust-cli-feature taskType must be feature
fixture-shape	.cases[0].fixture = "unrelated-fixture"	rust-cli-feature fixture must be expense-report
gates	.cases[0].deterministicGates = ["format"]	rust-cli-feature deterministic gates must be exactly
metrics	.metrics.aggregates = ["success-rate"]	benchmark aggregate metrics must be exactly
turns	.diagnosticGates.expectedExecutionTurns = 8	expectedExecutionTurns must equal cases x conditions x samples
complete	.diagnosticGates.completeRuns = 8	completeRuns must equal expectedExecutionTurns
operational	.diagnosticGates.operationalErrors = 1	operationalErrors must be zero
provenance	.diagnosticGates.provenanceErrors = 1	provenanceErrors must be zero
safety	.diagnosticGates.safetyFailures = 1	safetyFailures must be zero
candidate	.diagnosticGates.candidateFailuresAreMeasurementOutcomes = false	candidate failures must remain measurement outcomes
CASES
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

@test "code-quality workspace preparation rejects an unknown case before replacing owned work" {
  work_root="$TEMP_ROOT/workspaces"
  node "$WORKSPACE_PREPARER" "$work_root" \
    --case rust-cli-feature \
    --samples 1 >/dev/null
  printf 'preserve prior work\n' >"$work_root/sentinel"

  run node "$WORKSPACE_PREPARER" "$work_root" \
    --case stock-service-bugfix \
    --samples 1

  [ "$status" -eq 2 ]
  [[ "$output" == *"unknown benchmark case: stock-service-bugfix"* ]]
  grep -q 'preserve prior work' "$work_root/sentinel"
  [ -d "$work_root/rust-cli-feature/sample-1/no-skills/.git" ]
}

@test "expense-report verifier rejects the baseline and accepts a known-good public CLI" {
  work_root="$TEMP_ROOT/workspaces"
  node "$WORKSPACE_PREPARER" "$work_root" \
    --case rust-cli-feature \
    --samples 1 >/dev/null
  workspace="$work_root/rust-cli-feature/sample-1/no-skills"
  target_dir="$workspace/target"
  CARGO_TARGET_DIR="$target_dir" cargo build \
    --locked \
    --manifest-path "$workspace/Cargo.toml" >/dev/null

  run node "$EXPENSE_VERIFIER" \
    --workspace "$workspace" \
    --bin "$target_dir/debug/expense-report"

  [ "$status" -eq 1 ]
  [ "$(jq -r '.pass' <<<"$output")" = false ]
  [ "$(jq -r '.checks[] | select(.id == "totals-duplicate-order") | .pass' <<<"$output")" = false ]

  candidate_workspace="$TEMP_ROOT/candidates"
  mkdir "$candidate_workspace"
  mark_benchmark_workspace "$candidate_workspace"
  passing_cli="$candidate_workspace/expense-report-passing"
  rustc --edition 2024 \
    --check-cfg 'cfg(host_escape_probe)' \
    --check-cfg 'cfg(known_bad_adjacent)' \
    --check-cfg 'cfg(known_bad_total_order)' \
    --check-cfg 'cfg(known_bad_u8_minimum)' \
    --check-cfg 'cfg(sandbox_root_probe)' \
    --check-cfg 'cfg(stderr_secret_probe)' \
    "$ROOT/scripts/tests/fixtures/expense-report-passing.rs" \
    -o "$passing_cli"
  run node "$EXPENSE_VERIFIER" \
    --workspace "$candidate_workspace" \
    --bin "$passing_cli"

  [ "$status" -eq 0 ]
  [ "$(jq -r '.pass' <<<"$output")" = true ]
  [ "$(jq '[.checks[] | select(.pass == false)] | length' <<<"$output")" -eq 0 ]

  stderr_leak_cli="$candidate_workspace/expense-report-stderr-leak"
  rustc --edition 2024 \
    --cfg stderr_secret_probe \
    --check-cfg 'cfg(host_escape_probe)' \
    --check-cfg 'cfg(known_bad_adjacent)' \
    --check-cfg 'cfg(known_bad_total_order)' \
    --check-cfg 'cfg(known_bad_u8_minimum)' \
    --check-cfg 'cfg(sandbox_root_probe)' \
    --check-cfg 'cfg(stderr_secret_probe)' \
    "$ROOT/scripts/tests/fixtures/expense-report-passing.rs" \
    -o "$stderr_leak_cli"
  run node "$EXPENSE_VERIFIER" \
    --workspace "$candidate_workspace" \
    --bin "$stderr_leak_cli"

  [ "$status" -eq 1 ]
  [[ "$output" != *"ghp_FAKE_BENCHMARK_SECRET_DO_NOT_PERSIST"* ]]

  sandbox_root_cli="$candidate_workspace/expense-report-sandbox-root"
  rustc --edition 2024 \
    --cfg sandbox_root_probe \
    --check-cfg 'cfg(host_escape_probe)' \
    --check-cfg 'cfg(known_bad_adjacent)' \
    --check-cfg 'cfg(known_bad_total_order)' \
    --check-cfg 'cfg(known_bad_u8_minimum)' \
    --check-cfg 'cfg(sandbox_root_probe)' \
    --check-cfg 'cfg(stderr_secret_probe)' \
    "$ROOT/scripts/tests/fixtures/expense-report-passing.rs" \
    -o "$sandbox_root_cli"
  run node "$EXPENSE_VERIFIER" \
    --workspace "$candidate_workspace" \
    --bin "$sandbox_root_cli"

  [ "$status" -eq 0 ]
  [ "$(jq -r '.pass' <<<"$output")" = true ]

  adjacent_only_cli="$candidate_workspace/expense-report-adjacent-only"
  rustc --edition 2024 \
    --cfg known_bad_adjacent \
    --check-cfg 'cfg(host_escape_probe)' \
    --check-cfg 'cfg(known_bad_adjacent)' \
    --check-cfg 'cfg(known_bad_total_order)' \
    --check-cfg 'cfg(known_bad_u8_minimum)' \
    --check-cfg 'cfg(sandbox_root_probe)' \
    --check-cfg 'cfg(stderr_secret_probe)' \
    "$ROOT/scripts/tests/fixtures/expense-report-passing.rs" \
    -o "$adjacent_only_cli"
  run node "$EXPENSE_VERIFIER" \
    --workspace "$candidate_workspace" \
    --bin "$adjacent_only_cli"

  [ "$status" -eq 1 ]
  [ "$(jq -r '.checks[] | select(.id == "totals-duplicate-order") | .pass' <<<"$output")" = false ]

  total_order_cli="$candidate_workspace/expense-report-total-order"
  rustc --edition 2024 \
    --cfg known_bad_total_order \
    --check-cfg 'cfg(host_escape_probe)' \
    --check-cfg 'cfg(known_bad_adjacent)' \
    --check-cfg 'cfg(known_bad_total_order)' \
    --check-cfg 'cfg(known_bad_u8_minimum)' \
    --check-cfg 'cfg(sandbox_root_probe)' \
    --check-cfg 'cfg(stderr_secret_probe)' \
    "$ROOT/scripts/tests/fixtures/expense-report-passing.rs" \
    -o "$total_order_cli"
  run node "$EXPENSE_VERIFIER" \
    --workspace "$candidate_workspace" \
    --bin "$total_order_cli"

  [ "$status" -eq 1 ]
  [ "$(jq -r '.checks[] | select(.id == "totals-orders-by-category-not-amount") | .pass' <<<"$output")" = false ]

  undersized_minimum_cli="$candidate_workspace/expense-report-u8-minimum"
  rustc --edition 2024 \
    --cfg known_bad_u8_minimum \
    --check-cfg 'cfg(host_escape_probe)' \
    --check-cfg 'cfg(known_bad_adjacent)' \
    --check-cfg 'cfg(known_bad_total_order)' \
    --check-cfg 'cfg(known_bad_u8_minimum)' \
    --check-cfg 'cfg(sandbox_root_probe)' \
    --check-cfg 'cfg(stderr_secret_probe)' \
    "$ROOT/scripts/tests/fixtures/expense-report-passing.rs" \
    -o "$undersized_minimum_cli"
  run node "$EXPENSE_VERIFIER" \
    --workspace "$candidate_workspace" \
    --bin "$undersized_minimum_cli"

  [ "$status" -eq 1 ]
  [ "$(jq -r '.checks[] | select(.id == "totals-minimum-larger-than-u32") | .pass' <<<"$output")" = false ]
}

@test "expense-report verifier bounds a timed-out candidate and its pipe-holding descendant" {
  candidate_workspace="$TEMP_ROOT/candidates"
  mkdir "$candidate_workspace"
  mark_benchmark_workspace "$candidate_workspace"
  hanging_cli="$candidate_workspace/expense-report-hanging"
  rustc --edition 2024 \
    "$ROOT/scripts/tests/fixtures/expense-report-hanging.rs" \
    -o "$hanging_cli"

  run timeout 6.5s node "$EXPENSE_VERIFIER" \
    --workspace "$candidate_workspace" \
    --bin "$hanging_cli"
  verifier_status="$status"
  verifier_output="$output"

  [ "$verifier_status" -eq 1 ]
  [ "$(jq -r '.checks[0].observed.status' <<<"$verifier_output")" = "error:TIMEOUT" ]
  [ "$(jq -r '.checks[0].observed.cleanup.trackedProcesses' <<<"$verifier_output")" -ge 2 ]
  [ "$(jq -r '.checks[0].observed.cleanup.survivingProcesses' <<<"$verifier_output")" -eq 0 ]
}

@test "expense-report verifier rejects unusable executables and bounds output flooding" {
  candidate_workspace="$TEMP_ROOT/candidates"
  mkdir "$candidate_workspace"
  mark_benchmark_workspace "$candidate_workspace"
  node_bin="$(realpath "$(command -v node)")"
  run node "$EXPENSE_VERIFIER" \
    --workspace "$candidate_workspace" \
    --bin "$candidate_workspace/missing"

  [ "$status" -eq 2 ]
  [[ "$output" == *"expense-report executable is missing"* ]]

  non_executable="$candidate_workspace/not-executable"
  cp "$ROOT/scripts/tests/fixtures/expense-report-passing.rs" "$non_executable"
  run node "$EXPENSE_VERIFIER" \
    --workspace "$candidate_workspace" \
    --bin "$non_executable"

  [ "$status" -eq 2 ]
  [[ "$output" == *"expense-report file is not executable"* ]]

  run node "$EXPENSE_VERIFIER" \
    --workspace "$candidate_workspace" \
    --bin "$node_bin"

  [ "$status" -eq 2 ]
  [[ "$output" == *"expense-report executable must be inside its workspace"* ]]

  root_workspace_cli="$candidate_workspace/expense-report-root-workspace"
  rustc --edition 2024 \
    --check-cfg 'cfg(host_escape_probe)' \
    --check-cfg 'cfg(known_bad_adjacent)' \
    --check-cfg 'cfg(known_bad_total_order)' \
    --check-cfg 'cfg(known_bad_u8_minimum)' \
    --check-cfg 'cfg(sandbox_root_probe)' \
    --check-cfg 'cfg(stderr_secret_probe)' \
    "$ROOT/scripts/tests/fixtures/expense-report-passing.rs" \
    -o "$root_workspace_cli"
  run node "$EXPENSE_VERIFIER" \
    --workspace / \
    --bin "$root_workspace_cli"

  [ "$status" -eq 2 ]
  [[ "$output" == *"prepared benchmark workspace marker"* ]]

  flooding_cli="$candidate_workspace/expense-report-flooding"
  rustc --edition 2024 \
    "$ROOT/scripts/tests/fixtures/expense-report-flooding.rs" \
    -o "$flooding_cli"
  run node "$EXPENSE_VERIFIER" \
    --workspace "$candidate_workspace" \
    --bin "$flooding_cli"

  [ "$status" -eq 1 ]
  [ "$(jq -r '.checks[0].observed.status' <<<"$output")" = "error:OUTPUT_LIMIT" ]
  [ "${#output}" -lt 10000 ]

  run env \
    -u AI_PLUGINS_BWRAP_BIN \
    -u AI_PLUGINS_PRLIMIT_BIN \
    PATH="$candidate_workspace" \
    "$node_bin" "$EXPENSE_VERIFIER" \
    --workspace "$candidate_workspace" \
    --bin "$flooding_cli"

  [ "$status" -eq 2 ]
  [[ "$output" == *"AI_PLUGINS_BWRAP_BIN must be set by the ai-plugins Nix devshell"* ]]

  run env AI_PLUGINS_BWRAP_BIN="$node_bin" \
    "$node_bin" "$EXPENSE_VERIFIER" \
    --workspace "$candidate_workspace" \
    --bin "$flooding_cli"

  [ "$status" -eq 2 ]
  [[ "$output" == *"bwrap is not the flake-selected Nix package executable"* ]]
}

@test "expense-report verifier isolates candidates from host files and network" {
  candidate_workspace="$TEMP_ROOT/candidate-workspace"
  mkdir "$candidate_workspace"
  mark_benchmark_workspace "$candidate_workspace"
  secret_path="$TEMP_ROOT/host-secret"
  mutation_path="$TEMP_ROOT/host-mutation"
  port_path="$TEMP_ROOT/host-server-port"
  printf 'benchmark-host-secret\n' >"$secret_path"

  node -e '
    const fs = require("node:fs");
    const net = require("node:net");
    const server = net.createServer((socket) => socket.end());
    server.listen(0, "127.0.0.1", () => {
      fs.writeFileSync(process.argv[1], String(server.address().port));
    });
  ' "$port_path" &
  HOST_SERVER_PID=$!
  for _ in {1..100}; do
    [ -s "$port_path" ] && break
    sleep 0.01
  done
  [ -s "$port_path" ]
  network_address="127.0.0.1:$(<"$port_path")"

  hostile_cli="$candidate_workspace/expense-report-hostile"
  env \
    EXPENSE_REPORT_TEST_SECRET_PATH="$secret_path" \
    EXPENSE_REPORT_TEST_MUTATION_PATH="$mutation_path" \
    EXPENSE_REPORT_TEST_NETWORK_ADDRESS="$network_address" \
    rustc --edition 2024 \
      --cfg host_escape_probe \
      --check-cfg 'cfg(host_escape_probe)' \
      --check-cfg 'cfg(known_bad_adjacent)' \
      --check-cfg 'cfg(known_bad_total_order)' \
      --check-cfg 'cfg(known_bad_u8_minimum)' \
      --check-cfg 'cfg(sandbox_root_probe)' \
      --check-cfg 'cfg(stderr_secret_probe)' \
      "$ROOT/scripts/tests/fixtures/expense-report-passing.rs" \
      -o "$hostile_cli"

  run "$hostile_cli" totals
  [ "$status" -eq 1 ]
  [[ "$output" == *"benchmark-host-secret"* ]]
  [[ "$output" == *"wrote-host"* ]]
  [[ "$output" == *"reached-host-network"* ]]
  [[ "$output" == *"resource-limit:Max cpu time="* ]]
  [[ "$output" == *"resource-limit:Max address space="* ]]
  [[ "$output" == *"resource-limit:Max processes="* ]]
  [ -f "$mutation_path" ]
  rm -f "$mutation_path"

  run node "$EXPENSE_VERIFIER" \
    --workspace "$candidate_workspace" \
    --bin "$hostile_cli"

  [ "$status" -eq 0 ]
  [ "$(jq -r '.pass' <<<"$output")" = true ]
  [[ "$output" != *"benchmark-host-secret"* ]]
  [ ! -e "$mutation_path" ]

  fake_tools="$TEMP_ROOT/fake-tools"
  mkdir "$fake_tools"
  node_bin="$(realpath "$(command -v node)")"
  ln -s "$node_bin" "$fake_tools/bwrap"
  ln -s "$node_bin" "$fake_tools/prlimit"
  run env PATH="$fake_tools:$PATH" \
    node "$EXPENSE_VERIFIER" \
    --workspace "$candidate_workspace" \
    --bin "$hostile_cli"

  [ "$status" -eq 0 ]
  [ "$(jq -r '.pass' <<<"$output")" = true ]
}
