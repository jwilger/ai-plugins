#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  PREPARER="$ROOT/scripts/evals/prepare-code-quality-workspaces.mjs"
  SCORER="$ROOT/evals/benchmarks/downstream-code-quality/verifiers/score-expense-report.mjs"
  TREE_HASH="$ROOT/scripts/evals/code-quality-tree-hash.mjs"
  TRUSTED_FIXTURE="$ROOT/evals/benchmarks/downstream-code-quality/fixtures/expense-report"
  TEST_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/ai-plugins-scorer-security.XXXXXX")"
  printf 'ai-plugins downstream code-quality run root\n' \
    >"$TEST_ROOT/.ai-plugins-code-quality-run-root"
  chmod 600 "$TEST_ROOT/.ai-plugins-code-quality-run-root"
  WORK_ROOT="$TEST_ROOT/workspaces"
  VERIFIER_TMP="$TEST_ROOT/verifier-tmp"

  closure_roots=()
  for tool in bash cargo cargo-clippy cargo-fmt cc cp env git prlimit rustc rustdoc rustfmt; do
    tool_path="$(realpath "$(command -v "$tool")")"
    closure_roots+=("$(dirname "$(dirname "$tool_path")")")
  done
  NIX_STORE_CLOSURE="$TEST_ROOT/nix-store-closure"
  nix-store --query --requisites "${closure_roots[@]}" \
    | LC_ALL=C sort -u >"$NIX_STORE_CLOSURE"
  chmod 400 "$NIX_STORE_CLOSURE"
  export CODE_QUALITY_NIX_STORE_CLOSURE="$NIX_STORE_CLOSURE"
  export CODE_QUALITY_NIX_STORE_CLOSURE_EXPECTED_SHA256
  CODE_QUALITY_NIX_STORE_CLOSURE_EXPECTED_SHA256="$(
    sha256sum "$NIX_STORE_CLOSURE" | cut -d' ' -f1
  )"
  export CODE_QUALITY_SYSTEMD_RUN_BIN
  CODE_QUALITY_SYSTEMD_RUN_BIN="$(realpath "$(command -v systemd-run)")"
  export CODE_QUALITY_SYSTEMD_RUN_EXPECTED_SHA256
  CODE_QUALITY_SYSTEMD_RUN_EXPECTED_SHA256="$(
    sha256sum "$CODE_QUALITY_SYSTEMD_RUN_BIN" | cut -d' ' -f1
  )"
  excluded_nix_source_fixture="$TEST_ROOT/source"
  mkdir "$excluded_nix_source_fixture"
  printf 'unlisted Nix source canary\n' \
    >"$excluded_nix_source_fixture/canary"
  EXCLUDED_NIX_SOURCE="$(nix-store --add "$excluded_nix_source_fixture")"
  [ -d "$EXCLUDED_NIX_SOURCE" ]
  ! grep -Fxq -- "$EXCLUDED_NIX_SOURCE" "$NIX_STORE_CLOSURE"

  node "$PREPARER" "$WORK_ROOT" --case rust-cli-feature --samples 1 \
    >"$TEST_ROOT/manifest.stdout"
  WORKSPACE_MANIFEST="$WORK_ROOT/manifest.json"
  WORKSPACE="$WORK_ROOT/rust-cli-feature/sample-1/no-marketplace-skills"
  BASELINE_OID="$(jq -r '.workspaces[0].baselineOid' "$WORKSPACE_MANIFEST")"
  FIXTURE_DIGEST="$(
    node --input-type=module -e '
      const { snapshotRegularTree } = await import(process.argv[1]);
      process.stdout.write(snapshotRegularTree(process.argv[2]).digest);
    ' "$TREE_HASH" "$TRUSTED_FIXTURE"
  )"
  mkdir "$VERIFIER_TMP"
}

teardown() {
  if [ -n "${PUBLIC_VERIFIER_PID:-}" ]; then
    kill -KILL "$PUBLIC_VERIFIER_PID" 2>/dev/null || true
    wait "$PUBLIC_VERIFIER_PID" 2>/dev/null || true
  fi
  if [ -n "${SCORER_PID:-}" ]; then
    kill -KILL "$SCORER_PID" 2>/dev/null || true
    wait "$SCORER_PID" 2>/dev/null || true
  fi
  rm -rf "$TEST_ROOT"
}

run_scorer() {
  run env \
    CODE_QUALITY_VERIFIER_TMP_ROOT="$VERIFIER_TMP" \
    node "$SCORER" \
    --workspace "$WORKSPACE" \
    --baseline-oid "$BASELINE_OID" \
    --trusted-fixture-digest "${1:-$FIXTURE_DIGEST}"
}

mark_benchmark_workspace() {
  local workspace="$1"
  git -C "$workspace" init --quiet --initial-branch=main
  printf 'ai-plugins downstream code-quality workspace\n' \
    >"$workspace/.git/.ai-plugins-code-quality-workspace"
}

@test "scorer emits only bounded hashed change evidence and trusted digests" {
  secret_name="secret_github_pat.rs"
  secret_value="ghp_FAKE_SCORER_SECRET_MUST_NOT_APPEAR"
  printf 'const PRIVATE_VALUE: &str = "%s";\n' "$secret_value" \
    >"$WORKSPACE/src/$secret_name"

  run_scorer

  [ "$status" -eq 0 ]
  [[ "$output" != *"$secret_name"* ]]
  [[ "$output" != *"$secret_value"* ]]
  jq -e '
    (.changedPaths | not) and
    (.trustedFixtureSha256 | test("^[0-9a-f]{64}$")) and
    (.verifierSha256 | not) and
    (.verifierCompositionSha256 | test("^[0-9a-f]{64}$")) and
    (.changeEvidence | keys) == [
      "addedFileCount",
      "candidateTreeSha256",
      "changedFileCount",
      "deletedFileCount",
      "diffSha256",
      "modifiedFileCount",
      "sourceByteCount",
      "sourceFileCount"
    ] and
    (.changeEvidence.sourceFileCount | type == "number" and . >= 0 and . <= 64) and
    (.changeEvidence.sourceByteCount | type == "number" and . >= 0 and . <= 2097152) and
    (.changeEvidence.addedFileCount == 1) and
    (.changeEvidence.modifiedFileCount == 0) and
    (.changeEvidence.deletedFileCount == 0) and
    (.changeEvidence.changedFileCount == 1) and
    (.changeEvidence.candidateTreeSha256 | test("^[0-9a-f]{64}$")) and
    (.changeEvidence.diffSha256 | test("^[0-9a-f]{64}$"))
  ' <<<"$output"
  [ -z "$(find "$VERIFIER_TMP" -mindepth 1 -print -quit)" ]
}

@test "scorer rejects a supplied digest that does not match its fixture snapshot" {
  wrong_digest="$(printf '0%.0s' {1..64})"

  run_scorer "$wrong_digest"

  [ "$status" -eq 2 ]
  [ "$output" = \
    "score-expense-report:provenance-failure:trusted-fixture-digest-mismatch" ]
  [ -z "$(find "$VERIFIER_TMP" -mindepth 1 -print -quit)" ]
}

@test "scorer rejects a closure detached from its private owned run marker" {
  printf 'not the benchmark run marker\n' \
    >"$TEST_ROOT/.ai-plugins-code-quality-run-root"

  run_scorer

  [ "$status" -eq 2 ]
  [ "$output" = \
    "score-expense-report:operational-failure:nix-store-closure-run-root-invalid" ]
  [ -z "$(find "$VERIFIER_TMP" -mindepth 1 -print -quit)" ]
}

@test "scorer rejects a closure whose orchestration digest does not match" {
  run env \
    CODE_QUALITY_NIX_STORE_CLOSURE_EXPECTED_SHA256="$(printf '0%.0s' {1..64})" \
    CODE_QUALITY_VERIFIER_TMP_ROOT="$VERIFIER_TMP" \
    node "$SCORER" \
    --workspace "$WORKSPACE" \
    --baseline-oid "$BASELINE_OID" \
    --trusted-fixture-digest "$FIXTURE_DIGEST"

  [ "$status" -eq 2 ]
  [ "$output" = \
    "score-expense-report:operational-failure:nix-store-closure-sha256-mismatch" ]
  [ -z "$(find "$VERIFIER_TMP" -mindepth 1 -print -quit)" ]
}

@test "scorer rejects a symlinked closure manifest" {
  symlinked_closure="$TEST_ROOT/symlinked-nix-store-closure"
  ln -s "$NIX_STORE_CLOSURE" "$symlinked_closure"

  run env \
    CODE_QUALITY_NIX_STORE_CLOSURE="$symlinked_closure" \
    CODE_QUALITY_VERIFIER_TMP_ROOT="$VERIFIER_TMP" \
    node "$SCORER" \
    --workspace "$WORKSPACE" \
    --baseline-oid "$BASELINE_OID" \
    --trusted-fixture-digest "$FIXTURE_DIGEST"

  [ "$status" -eq 2 ]
  [ "$output" = \
    "score-expense-report:operational-failure:nix-store-closure-unsafe" ]
  [ -z "$(find "$VERIFIER_TMP" -mindepth 1 -print -quit)" ]
}

@test "scorer rejects a closure manifest that is not bytewise sorted" {
  unsorted_closure="$TEST_ROOT/unsorted-nix-store-closure"
  tail -n 2 "$NIX_STORE_CLOSURE" | sort -r >"$unsorted_closure"
  chmod 400 "$unsorted_closure"

  run env \
    CODE_QUALITY_NIX_STORE_CLOSURE="$unsorted_closure" \
    CODE_QUALITY_NIX_STORE_CLOSURE_EXPECTED_SHA256="$(
      sha256sum "$unsorted_closure" | cut -d' ' -f1
    )" \
    CODE_QUALITY_VERIFIER_TMP_ROOT="$VERIFIER_TMP" \
    node "$SCORER" \
    --workspace "$WORKSPACE" \
    --baseline-oid "$BASELINE_OID" \
    --trusted-fixture-digest "$FIXTURE_DIGEST"

  [ "$status" -eq 2 ]
  [ "$output" = \
    "score-expense-report:operational-failure:nix-store-closure-invalid" ]
  [ -z "$(find "$VERIFIER_TMP" -mindepth 1 -print -quit)" ]
}

@test "scorer rejects a closure manifest with non-private permissions" {
  chmod 600 "$NIX_STORE_CLOSURE"

  run_scorer

  [ "$status" -eq 2 ]
  [ "$output" = \
    "score-expense-report:operational-failure:nix-store-closure-unsafe" ]
  [ -z "$(find "$VERIFIER_TMP" -mindepth 1 -print -quit)" ]
}

@test "scorer classifies candidate output exhaustion as a safety failure" {
  cat >"$WORKSPACE/tests/output_flood.rs" <<'RUST'
#[test]
fn floods_cargo_output() {
    eprintln!("{}", "x".repeat(256 * 1024));
    panic!("intentional output flood");
}
RUST

  run_scorer

  [ "$status" -eq 0 ]
  [ "$(jq -r '.outcomeClass' <<<"$output")" = safety-failure ]
  [ "$(jq -r '.gates.safety' <<<"$output")" = false ]
  [[ "$output" != *"intentional output flood"* ]]
  [[ "$output" != *"output_flood.rs"* ]]
  [ -z "$(find "$VERIFIER_TMP" -mindepth 1 -print -quit)" ]
}

@test "scorer classifies a missing trusted toolchain as operational" {
  node_bin="$(realpath "$(command -v node)")"
  git_bin_directory="$(dirname "$(realpath "$(command -v git)")")"

  run env \
    PATH="$git_bin_directory" \
    AI_PLUGINS_BWRAP_BIN="$AI_PLUGINS_BWRAP_BIN" \
    AI_PLUGINS_PRLIMIT_BIN="$AI_PLUGINS_PRLIMIT_BIN" \
    CODE_QUALITY_VERIFIER_TMP_ROOT="$VERIFIER_TMP" \
    "$node_bin" "$SCORER" \
    --workspace "$WORKSPACE" \
    --baseline-oid "$BASELINE_OID" \
    --trusted-fixture-digest "$FIXTURE_DIGEST"

  [ "$status" -eq 2 ]
  [[ "$output" == \
    score-expense-report:operational-failure:nix-tool-*-missing ]]
  [ -z "$(find "$VERIFIER_TMP" -mindepth 1 -print -quit)" ]
}

@test "scorer refuses direct execution without its pinned aggregate scope tool" {
  run env \
    -u CODE_QUALITY_SYSTEMD_RUN_BIN \
    -u CODE_QUALITY_SYSTEMD_RUN_EXPECTED_SHA256 \
    CODE_QUALITY_VERIFIER_TMP_ROOT="$VERIFIER_TMP" \
    node "$SCORER" \
    --workspace "$WORKSPACE" \
    --baseline-oid "$BASELINE_OID" \
    --trusted-fixture-digest "$FIXTURE_DIGEST"

  [ "$status" -eq 2 ]
  [ "$output" = \
    "score-expense-report:operational-failure:systemd-run-path-missing" ]
  [ -z "$(find "$VERIFIER_TMP" -mindepth 1 -print -quit)" ]
}

@test "scorer contains its full process tree in the fixed aggregate systemd scope" {
  scorer_stdout="$TEST_ROOT/scorer.stdout"
  scorer_stderr="$TEST_ROOT/scorer.stderr"
  env \
    ALL_PROXY=http://ambient-proxy.invalid \
    AWS_ACCESS_KEY_ID=AKIAFAKESCORERBOUNDARY \
    AWS_SECRET_ACCESS_KEY=fake-scorer-secret-access-key \
    CODEX_API_KEY=fake-codex-key-must-not-cross-scorer-scope \
    CODE_QUALITY_VERIFIER_TMP_ROOT="$VERIFIER_TMP" \
    HTTPS_PROXY=http://ambient-proxy.invalid \
    OPENAI_API_KEY=fake-openai-key-must-not-cross-scorer-scope \
    node "$SCORER" \
    --workspace "$WORKSPACE" \
    --baseline-oid "$BASELINE_OID" \
    --trusted-fixture-digest "$FIXTURE_DIGEST" \
    >"$scorer_stdout" 2>"$scorer_stderr" &
  SCORER_PID=$!

  scope_unit=""
  for _ in {1..500}; do
    scope_unit="$(
      systemctl --user list-units \
        --type=scope \
        --state=running \
        --no-legend \
        --plain \
        "ai-plugins-code-quality-scorer-$SCORER_PID-*.scope" \
        | awk 'NR == 1 { print $1 }'
    )"
    [ -n "$scope_unit" ] && break
    kill -0 "$SCORER_PID" 2>/dev/null || break
    sleep 0.01
  done

  [ -n "$scope_unit" ]
  [ "$(systemctl --user show "$scope_unit" --property=MemoryMax --value)" = \
    8589934592 ]
  [ "$(systemctl --user show "$scope_unit" --property=MemorySwapMax --value)" = \
    0 ]
  [ "$(systemctl --user show "$scope_unit" --property=TasksMax --value)" = 512 ]
  [ "$(systemctl --user show "$scope_unit" --property=CPUQuotaPerSecUSec --value)" = \
    4s ]
  [ "$(systemctl --user show "$scope_unit" --property=OOMPolicy --value)" = kill ]
  [ "$(systemctl --user show "$scope_unit" --property=KillMode --value)" = \
    control-group ]

  descendant_pids=()
  pending_pids=("$SCORER_PID")
  while [ "${#pending_pids[@]}" -gt 0 ]; do
    parent_pid="${pending_pids[0]}"
    pending_pids=("${pending_pids[@]:1}")
    [ -r "/proc/$parent_pid/task/$parent_pid/children" ] || continue
    for child_pid in $(<"/proc/$parent_pid/task/$parent_pid/children"); do
      descendant_pids+=("$child_pid")
      pending_pids+=("$child_pid")
    done
  done
  [ "${#descendant_pids[@]}" -ge 2 ]
  for child_pid in "${descendant_pids[@]}"; do
    [ -r "/proc/$child_pid/environ" ] || continue
    ! tr '\0' '\n' <"/proc/$child_pid/environ" \
      | grep -aEq \
        '^(ALL_PROXY|AWS_ACCESS_KEY_ID|AWS_SECRET_ACCESS_KEY|CODEX_API_KEY|HTTPS_PROXY|OPENAI_API_KEY)='
  done

  set +e
  wait "$SCORER_PID"
  scorer_status=$?
  set -e
  SCORER_PID=""
  [ "$scorer_status" -eq 0 ]
  [ ! -s "$scorer_stderr" ]
  [ "$(jq -r '.outcomeClass' "$scorer_stdout")" = candidate-failure ]
  [ "$(systemctl --user show "$scope_unit" --property=LoadState --value)" = \
    not-found ]
  [ -z "$(find "$VERIFIER_TMP" -mindepth 1 -print -quit)" ]
}

@test "scorer cancellation kills the aggregate scope and removes private scratch state" {
  scorer_stdout="$TEST_ROOT/cancelled-scorer.stdout"
  scorer_stderr="$TEST_ROOT/cancelled-scorer.stderr"
  env \
    CODE_QUALITY_VERIFIER_TMP_ROOT="$VERIFIER_TMP" \
    node "$SCORER" \
    --workspace "$WORKSPACE" \
    --baseline-oid "$BASELINE_OID" \
    --trusted-fixture-digest "$FIXTURE_DIGEST" \
    >"$scorer_stdout" 2>"$scorer_stderr" &
  SCORER_PID=$!

  scope_unit=""
  for _ in {1..500}; do
    scope_unit="$(
      systemctl --user list-units \
        --type=scope \
        --state=running \
        --no-legend \
        --plain \
        "ai-plugins-code-quality-scorer-$SCORER_PID-*.scope" \
        | awk 'NR == 1 { print $1 }'
    )"
    [ -n "$scope_unit" ] && break
    kill -0 "$SCORER_PID" 2>/dev/null || break
    sleep 0.01
  done
  [ -n "$scope_unit" ]

  scratch_entry=""
  for _ in {1..500}; do
    scratch_entry="$(find "$VERIFIER_TMP" -mindepth 1 -maxdepth 1 -print -quit)"
    [ -n "$scratch_entry" ] && break
    kill -0 "$SCORER_PID" 2>/dev/null || break
    sleep 0.01
  done
  [ -n "$scratch_entry" ]

  kill -TERM "$SCORER_PID"
  scorer_status=0
  wait "$SCORER_PID" || scorer_status=$?
  SCORER_PID=""

  [ "$scorer_status" -eq 2 ]
  [ ! -s "$scorer_stdout" ]
  [ "$(<"$scorer_stderr")" = \
    score-expense-report:operational-failure:resource-scope-cancelled ]
  for _ in {1..500}; do
    load_state="$(
      systemctl --user show "$scope_unit" --property=LoadState --value
    )"
    [ "$load_state" = not-found ] && break
    sleep 0.01
  done
  [ "$load_state" = not-found ]
  [ -z "$(find "$VERIFIER_TMP" -mindepth 1 -print -quit)" ]
}

@test "public verifier rejects a closure that omits its required tool root" {
  candidate_workspace="$TEST_ROOT/public-candidate"
  mkdir "$candidate_workspace"
  mark_benchmark_workspace "$candidate_workspace"
  passing_cli="$candidate_workspace/expense-report"
  rustc --edition 2024 \
    --check-cfg 'cfg(host_escape_probe)' \
    --check-cfg 'cfg(known_bad_adjacent)' \
    --check-cfg 'cfg(known_bad_total_order)' \
    --check-cfg 'cfg(known_bad_u8_minimum)' \
    --check-cfg 'cfg(nix_store_source_probe)' \
    --check-cfg 'cfg(sandbox_root_probe)' \
    --check-cfg 'cfg(stderr_secret_probe)' \
    "$ROOT/scripts/tests/fixtures/expense-report-passing.rs" \
    -o "$passing_cli"
  incomplete_closure="$TEST_ROOT/incomplete-nix-store-closure"
  printf '%s\n' "$EXCLUDED_NIX_SOURCE" >"$incomplete_closure"
  chmod 400 "$incomplete_closure"

  run env \
    CODE_QUALITY_NIX_STORE_CLOSURE="$incomplete_closure" \
    CODE_QUALITY_NIX_STORE_CLOSURE_EXPECTED_SHA256="$(
      sha256sum "$incomplete_closure" | cut -d' ' -f1
    )" \
    node "$ROOT/evals/benchmarks/downstream-code-quality/verifiers/expense-report.mjs" \
    --workspace "$candidate_workspace" \
    --bin "$passing_cli"

  [ "$status" -eq 2 ]
  [ "$output" = "expense-report:operational-failure:nix-store-closure-incomplete" ]
}

@test "public verifier refuses to execute candidate code without an aggregate scope" {
  candidate_workspace="$TEST_ROOT/unscoped-public-candidate"
  mkdir "$candidate_workspace"
  mark_benchmark_workspace "$candidate_workspace"
  passing_cli="$candidate_workspace/expense-report"
  rustc --edition 2024 \
    --check-cfg 'cfg(host_escape_probe)' \
    --check-cfg 'cfg(known_bad_adjacent)' \
    --check-cfg 'cfg(known_bad_total_order)' \
    --check-cfg 'cfg(known_bad_u8_minimum)' \
    --check-cfg 'cfg(nix_store_source_probe)' \
    --check-cfg 'cfg(sandbox_root_probe)' \
    --check-cfg 'cfg(stderr_secret_probe)' \
    "$ROOT/scripts/tests/fixtures/expense-report-passing.rs" \
    -o "$passing_cli"

  run env \
    -u CODE_QUALITY_SYSTEMD_RUN_BIN \
    -u CODE_QUALITY_SYSTEMD_RUN_EXPECTED_SHA256 \
    node "$ROOT/evals/benchmarks/downstream-code-quality/verifiers/expense-report.mjs" \
    --workspace "$candidate_workspace" \
    --bin "$passing_cli"

  [ "$status" -eq 2 ]
  [ "$output" = \
    "expense-report:operational-failure:systemd-run-path-missing" ]
}

@test "direct public verifier uses the fixed aggregate scope without ambient secrets" {
  candidate_workspace="$TEST_ROOT/public-scope-candidate"
  mkdir "$candidate_workspace"
  mark_benchmark_workspace "$candidate_workspace"
  hanging_cli="$candidate_workspace/expense-report"
  rustc --edition 2024 \
    "$ROOT/scripts/tests/fixtures/expense-report-hanging.rs" \
    -o "$hanging_cli"
  verifier_stdout="$TEST_ROOT/public-verifier.stdout"
  verifier_stderr="$TEST_ROOT/public-verifier.stderr"
  env \
    ALL_PROXY=http://ambient-public-proxy.invalid \
    AWS_SECRET_ACCESS_KEY=fake-public-verifier-secret \
    CODEX_API_KEY=fake-public-codex-key \
    OPENAI_API_KEY=fake-public-openai-key \
    node "$ROOT/evals/benchmarks/downstream-code-quality/verifiers/expense-report.mjs" \
    --workspace "$candidate_workspace" \
    --bin "$hanging_cli" \
    >"$verifier_stdout" 2>"$verifier_stderr" &
  PUBLIC_VERIFIER_PID=$!

  scope_unit=""
  for _ in {1..500}; do
    scope_unit="$(
      systemctl --user list-units \
        --type=scope \
        --state=running \
        --no-legend \
        --plain \
        "ai-plugins-code-quality-public-verifier-$PUBLIC_VERIFIER_PID-*.scope" \
        | awk 'NR == 1 { print $1 }'
    )"
    [ -n "$scope_unit" ] && break
    kill -0 "$PUBLIC_VERIFIER_PID" 2>/dev/null || break
    sleep 0.01
  done
  [ -n "$scope_unit" ]
  [ "$(systemctl --user show "$scope_unit" --property=MemoryMax --value)" = \
    8589934592 ]
  [ "$(systemctl --user show "$scope_unit" --property=MemorySwapMax --value)" = \
    0 ]
  [ "$(systemctl --user show "$scope_unit" --property=TasksMax --value)" = 512 ]
  [ "$(systemctl --user show "$scope_unit" --property=CPUQuotaPerSecUSec --value)" = \
    4s ]

  descendant_pids=()
  for _ in {1..500}; do
    descendant_pids=()
    pending_pids=("$PUBLIC_VERIFIER_PID")
    while [ "${#pending_pids[@]}" -gt 0 ]; do
      parent_pid="${pending_pids[0]}"
      pending_pids=("${pending_pids[@]:1}")
      [ -r "/proc/$parent_pid/task/$parent_pid/children" ] || continue
      for child_pid in $(<"/proc/$parent_pid/task/$parent_pid/children"); do
        descendant_pids+=("$child_pid")
        pending_pids+=("$child_pid")
      done
    done
    [ "${#descendant_pids[@]}" -ge 2 ] && break
    kill -0 "$PUBLIC_VERIFIER_PID" 2>/dev/null || break
    sleep 0.01
  done
  [ "${#descendant_pids[@]}" -ge 2 ]
  for child_pid in "${descendant_pids[@]}"; do
    [ -r "/proc/$child_pid/environ" ] || continue
    ! tr '\0' '\n' <"/proc/$child_pid/environ" \
      | grep -aEq \
        '^(ALL_PROXY|AWS_SECRET_ACCESS_KEY|CODEX_API_KEY|OPENAI_API_KEY)='
  done

  verifier_status=0
  wait "$PUBLIC_VERIFIER_PID" || verifier_status=$?
  PUBLIC_VERIFIER_PID=""
  [ "$verifier_status" -eq 1 ]
  [ ! -s "$verifier_stderr" ]
  [ "$(jq -r '.checks[0].observed.status' "$verifier_stdout")" = \
    error:TIMEOUT ]
  [ "$(jq -r '.checks[0].observed.cleanup.survivingProcesses' \
    "$verifier_stdout")" -eq 0 ]
  [ "$(systemctl --user show "$scope_unit" --property=LoadState --value)" = \
    not-found ]
}

@test "public verifier hides unlisted Nix source snapshots while required tools run" {
  candidate_workspace="$TEST_ROOT/public-source-canary"
  mkdir "$candidate_workspace"
  mark_benchmark_workspace "$candidate_workspace"
  passing_cli="$candidate_workspace/expense-report"
  EXPENSE_REPORT_TEST_NIX_SOURCE_PATH="$EXCLUDED_NIX_SOURCE" \
    rustc --edition 2024 \
      --cfg nix_store_source_probe \
      --check-cfg 'cfg(host_escape_probe)' \
      --check-cfg 'cfg(known_bad_adjacent)' \
      --check-cfg 'cfg(known_bad_total_order)' \
      --check-cfg 'cfg(known_bad_u8_minimum)' \
      --check-cfg 'cfg(nix_store_source_probe)' \
      --check-cfg 'cfg(sandbox_root_probe)' \
      --check-cfg 'cfg(stderr_secret_probe)' \
      "$ROOT/scripts/tests/fixtures/expense-report-passing.rs" \
      -o "$passing_cli"

  run "$passing_cli" totals
  [ "$status" -eq 1 ]
  [[ "$output" == *"unlisted-nix-source-visible"* ]]

  run node "$ROOT/evals/benchmarks/downstream-code-quality/verifiers/expense-report.mjs" \
    --workspace "$candidate_workspace" \
    --bin "$passing_cli"

  [ "$status" -eq 0 ]
  [ "$(jq -r '.pass' <<<"$output")" = true ]
  [[ "$output" != *"$EXCLUDED_NIX_SOURCE"* ]]
}

@test "trusted scorer hides unlisted Nix sources and retains the locked Rust toolchain" {
  cp "$ROOT/scripts/tests/fixtures/expense-report-passing.rs" \
    "$WORKSPACE/src/main.rs"
  cp "$ROOT/scripts/tests/fixtures/expense-report-totals-test.rs" \
    "$WORKSPACE/tests/totals.rs"
  cat >"$WORKSPACE/tests/nix_source_visibility.rs" <<RUST
#[test]
fn unlisted_nix_source_is_hidden() {
    assert!(!std::path::Path::new("$EXCLUDED_NIX_SOURCE").exists());
}
RUST

  run_scorer

  [ "$status" -eq 0 ]
  [ "$(jq -r '.outcomeClass' <<<"$output")" = pass ]
  [ "$(jq '[.gates[]] | all' <<<"$output")" = true ]
  [[ "$output" != *"$EXCLUDED_NIX_SOURCE"* ]]
  [ -z "$(find "$VERIFIER_TMP" -mindepth 1 -print -quit)" ]
}
