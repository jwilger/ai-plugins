#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  transition="$ROOT/plugins/development-system/scripts/transition-local-checkpoint.sh"
  repo="$BATS_TEST_TMPDIR/repo"
  records="$BATS_TEST_TMPDIR/records"
  mkdir -p "$repo" "$records"
  export GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
  git -C "$repo" init -q
  git -C "$repo" config user.name Test
  git -C "$repo" config user.email test@example.invalid
  git -C "$repo" config commit.gpgsign false
  git -C "$repo" config core.hooksPath "$repo/.git/hooks"
  printf 'baseline\n' >"$repo/source"
  git -C "$repo" add source
  git -C "$repo" commit -qm baseline
  baseline=$(git -C "$repo" rev-parse HEAD)
  target="$repo/.git/development-system/checkpoints/task.latest"
  evidence="$records/evidence.log"
  printf 'bounded actual test or review evidence\n' >"$evidence"
  sequence=0
}

invoke() {
  local operation=$1 fields=${2:-'{}'} id=${3:-"op-$sequence"}
  if [ -f "$target" ]; then
    generation=$(tail -c +15 "$target" | jq '.generation + 1')
    predecessor=$(sha256sum "$target" | cut -d ' ' -f 1)
  else generation=0; predecessor=null; fi
  jq -cn --argjson generation "$generation" --arg predecessor "$predecessor" --argjson fields "$fields" \
    '$fields + {expected_generation:$generation, expected_predecessor:(if $predecessor == "null" then null else $predecessor end)}' >"$records/request.json"
  run bash -c 'cd "$1" && exec "$2" task "$3" "$4" "$5"' _ "${invoke_cwd:-$repo}" "$transition" "$id" "$operation" "$records/request.json"
  sequence=$((sequence + 1))
}

accepted() {
  if [ "$status" -ne 0 ]; then echo "$output"; return 1; fi
  record=$(tail -c +15 "$target")
}

initialize() {
  invoke initialize '{"mode":"local-only","causal_edit":"implement fixture"}'
  accepted
}

evidence_fields() {
  local extra=${1:-'{}'}
  jq -cn --arg receipt "$evidence" --argjson extra "$extra" '{receipt_file:$receipt,command:"focused check"} + $extra'
}

passing() {
  initialize
  printf 'implemented\n' >"$repo/source"
  invoke edit-pass "$(evidence_fields)"
  accepted
}

reviewed() {
  passing
  invoke lightweight-review-pass "$(evidence_fields '{"route":"commit"}')"
  accepted
}

committed() {
  reviewed
  cat >"$repo/.git/hooks/pre-commit" <<'HOOK'
#!/bin/sh
printf 'real pre-commit passed\n'
HOOK
  chmod +x "$repo/.git/hooks/pre-commit"
  git -C "$repo" add source
  git -C "$repo" commit -m 'fixture committed' >"$evidence" 2>&1
  invoke commit-success "$(evidence_fields '{"mode":"local-only"}')"
  accepted
}

local_delivery() {
  committed
  invoke exact-verify-pass "$(evidence_fields)"
  accepted
  invoke local-delivery '{}'
  accepted
}

@test "typed initialization derives clean identity and edit pass requires the next causal action" {
  initialize
  [ "$(jq -r '.baseline_oid' <<<"$record")" = "$baseline" ]
  [ "$(jq -r '.generation' <<<"$record")" = 0 ]
  [ "$(jq -r '.next_action' <<<"$record")" = 'causal-edit: implement fixture' ]
  invoke commit-success "$(evidence_fields '{"mode":"local-only"}')"
  [ "$status" -ne 0 ]
  printf 'implemented\n' >"$repo/source"
  invoke edit-pass "$(evidence_fields)"
  accepted
  [ "$(jq -r '.next_action' <<<"$record")" = lightweight-review ]
}

@test "stable operation retries return the original receipt after later progress and reject changed input" {
  initialize
  cp "$records/request.json" "$records/initialize.json"
  initial_digest=$(sha256sum "$target" | cut -d ' ' -f 1)
  printf 'implemented\n' >"$repo/source"
  invoke edit-pass "$(evidence_fields)"
  accepted
  before=$(sha256sum "$target")
  run bash -c 'cd "$1" && exec "$2" task op-0 initialize "$3"' _ "$repo" "$transition" "$records/initialize.json"
  [ "$status" -eq 0 ]
  [ "$(jq -r '.record_sha256' <<<"$output")" = "$initial_digest" ]
  [ "$(sha256sum "$target")" = "$before" ]
  invoke edit-pass "$(evidence_fields)" op-0
  [ "$status" -ne 0 ]
  [ "$(sha256sum "$target")" = "$before" ]
}

@test "edit failure and unexpected passing RED clear all credits until fresh review" {
  initialize
  printf 'bad\n' >"$repo/source"
  invoke edit-fail "$(evidence_fields '{"failure_kind":"assertion","causal_repair":"repair fixture"}')"
  accepted
  [ "$(jq -r '.state' <<<"$record")" = failing ]
  invoke edit-invalid-test "$(evidence_fields '{"causal_repair":"make the assertion discriminate"}')"
  accepted
  [ "$(jq -r '.test.outcome' <<<"$record")" = pass ]
  [ "$(jq -r '.next_action' <<<"$record")" = 'rewrite-invalid-test: make the assertion discriminate' ]
  invoke edit-pass "$(evidence_fields)"
  accepted
  [ "$(jq '[.gates[]] | all(. == null)' <<<"$record")" = true ]
}

@test "lightweight and standalone fast gate failures preserve evidence and allow only causal repair" {
  passing
  invoke lightweight-review-fail "$(evidence_fields '{"causal_repair":"repair review finding"}')"
  accepted
  [ "$(jq -r '.test.failure_kind' <<<"$record")" = lightweight-review ]
  invoke edit-pass "$(evidence_fields)"
  accepted
  invoke lightweight-review-pass "$(evidence_fields '{"route":"local-snapshot"}')"
  accepted
  invoke fast-gate-fail "$(evidence_fields '{"causal_repair":"repair fast check"}')"
  accepted
  [ "$(jq -r '.test.failure_kind' <<<"$record")" = fast-gate ]
  [ "$(jq '[.gates[]] | all(. == null)' <<<"$record")" = true ]
}

@test "hook failure preserves staged unstaged and newly staged content" {
  reviewed
  printf '#!/bin/sh\necho actual-failure >&2\nexit 1\n' >"$repo/.git/hooks/pre-commit"
  chmod +x "$repo/.git/hooks/pre-commit"
  printf 'new content\n' >"$repo/new file"
  git -C "$repo" add source 'new file'
  run git -C "$repo" commit -m failed
  [ "$status" -eq 1 ]
  printf '%s\n' "$output" >"$evidence"
  printf 'unstaged\n' >>"$repo/source"
  index_before=$(git -C "$repo" write-tree)
  source_before=$(sha256sum "$repo/source" "$repo/new file")
  invoke hook-failure "$(evidence_fields '{"causal_repair":"repair actual hook failure"}')"
  accepted
  [ "$(jq -r '.test.failure_kind' <<<"$record")" = pre-commit-hook ]
  [ "$(git -C "$repo" write-tree)" = "$index_before" ]
  [ "$(sha256sum "$repo/source" "$repo/new file")" = "$source_before" ]
  [ "$(git -C "$repo" rev-parse HEAD)" = "$baseline" ]
}

@test "successful real commit exact verification failure repair retry and local delivery follow gate order" {
  committed
  [ "$(jq -r '.state' <<<"$record")" = committed ]
  invoke local-delivery '{}'
  [ "$status" -ne 0 ]
  invoke exact-verify-fail "$(evidence_fields)"
  accepted
  [ "$(jq -r '.next_action' <<<"$record")" = repair-exact-identity-verification ]
  invoke exact-verify-pass "$(evidence_fields)"
  [ "$status" -ne 0 ]
  invoke exact-verify-retry "$(evidence_fields)"
  accepted
  invoke exact-verify-pass "$(evidence_fields)"
  accepted
  invoke local-delivery '{}'
  accepted
  [ "$(jq -r '.next_action' <<<"$record")" = terminal-review ]
  [ "$(jq -r '.baseline_oid' <<<"$record")" = "$baseline" ]
}

@test "local snapshot delivery never invents a commit" {
  passing
  invoke lightweight-review-pass "$(evidence_fields '{"route":"local-snapshot"}')"
  accepted
  invoke fast-gate-pass "$(evidence_fields)"
  accepted
  invoke local-snapshot-delivery "$(evidence_fields)"
  accepted
  [ "$(jq -r '.delivery.commit_oid' <<<"$record")" = null ]
  [ "$(git -C "$repo" rev-parse HEAD)" = "$baseline" ]
  [ "$(jq -r '.next_action' <<<"$record")" = terminal-review ]
}

@test "terminal pass records exact source and terminal remediation clears gates" {
  local_delivery
  invoke terminal-review-remediation "$(evidence_fields '{"causal_repair":"repair terminal finding"}')"
  accepted
  [ "$(jq -r '.state' <<<"$record")" = failing ]
  [ "$(jq '[.gates[]] | all(. == null)' <<<"$record")" = true ]
}

@test "terminal pass is idempotent and rejects changed source" {
  local_delivery
  printf 'unreviewed\n' >"$repo/source"
  before=$(sha256sum "$target")
  invoke terminal-review-pass "$(evidence_fields)"
  [ "$status" -ne 0 ]
  [ "$(sha256sum "$target")" = "$before" ]
  git -C "$repo" restore source
  invoke terminal-review-pass "$(evidence_fields)"
  accepted
  [ "$(jq -r '.next_action' <<<"$record")" = complete ]
}

@test "gate pass rejects source drift and absent evidence without publishing" {
  passing
  before=$(sha256sum "$target")
  invoke lightweight-review-pass '{"receipt_file":"/nonexistent","command":"review","route":"commit"}'
  [ "$status" -ne 0 ]
  printf 'unreviewed\n' >"$repo/source"
  invoke lightweight-review-pass "$(evidence_fields '{"route":"commit"}')"
  [ "$status" -ne 0 ]
  [ "$(sha256sum "$target")" = "$before" ]
}

@test "operations reject unknown fields stale CAS and malformed authority" {
  initialize
  before=$(sha256sum "$target")
  invoke edit-pass "$(evidence_fields '{"baseline_oid":"invented"}')"
  [ "$status" -ne 0 ]
  [ "$(sha256sum "$target")" = "$before" ]
  jq '.expected_generation = 42 | del(.baseline_oid)' "$records/request.json" >"$records/stale.json"
  run bash -c 'cd "$1" && exec "$2" task stale edit-pass "$3"' _ "$repo" "$transition" "$records/stale.json"
  [ "$status" -ne 0 ]
  [ "$(sha256sum "$target")" = "$before" ]
  corrupt=$(jq -c '.test = {bad:"data"}' <<<"$record")
  printf 'checkpoint-v1 %s\n' "$corrupt" >"$target"
  before=$(sha256sum "$target")
  invoke edit-pass "$(evidence_fields)"
  [ "$status" -ne 0 ]
  [ "$(sha256sum "$target")" = "$before" ]
}

remote_reviewed() {
  remote="$BATS_TEST_TMPDIR/remote.git"
  git init --bare -q "$remote"
  git -C "$repo" remote add origin "$remote"
  git -C "$repo" push -q origin HEAD:refs/heads/main
  invoke initialize "$(jq -cn --arg mode "${remote_initialize_mode:-direct-to-trunk}" '{mode:$mode,causal_edit:"implement remote fixture",remote:"origin",ref:"refs/heads/main"}')"
  accepted
  printf 'remote implementation\n' >"$repo/source"
  invoke edit-pass "$(evidence_fields)"
  accepted
  invoke lightweight-review-pass "$(evidence_fields '{"route":"commit"}')"
  accepted
}

remote_committed() {
  remote_reviewed
  printf '#!/bin/sh\necho real-hook-pass\n' >"$repo/.git/hooks/pre-commit"
  chmod +x "$repo/.git/hooks/pre-commit"
  git -C "$repo" add source
  git -C "$repo" commit -m 'remote fixture' >"$evidence" 2>&1
  invoke commit-success "$(evidence_fields "$(jq -cn --arg mode "${remote_commit_mode:-direct-to-trunk}" '{mode:$mode}')")"
  accepted
  invoke exact-verify-pass "$(evidence_fields)"
  accepted
}

ci_fields() {
  evidence_fields "$(jq -cn --arg head "$(git -C "$repo" rev-parse HEAD)" --arg run "$1" --arg status "$2" '{provider:"fixture",run_id:$run,commit_oid:$head,status:$status}')"
}

@test "remote delivery requires authoritative readback and CI observations append exact SHA history" {
  remote_committed
  invoke push-readback '{"remote":"origin","ref":"refs/heads/main"}'
  [ "$status" -ne 0 ]
  [ "$(git --git-dir="$remote" rev-parse refs/heads/main)" = "$baseline" ]
  git -C "$repo" push -q origin HEAD:refs/heads/main
  invoke push-readback '{"remote":"origin","ref":"refs/heads/main"}'
  accepted
  [ "$(jq -r '.next_action' <<<"$record")" = register-exact-sha-ci-monitor ]
  invoke ci-register "$(ci_fields run-1 queued)"
  accepted
  first=$(jq -c '.ci.runs' <<<"$record")
  invoke ci-observe "$(ci_fields run-1 running)"
  accepted
  invoke ci-observe "$(ci_fields run-1 success)"
  accepted
  [ "$(jq -c '.ci.runs[0:1]' <<<"$record")" = "$first" ]
  [ "$(jq '.ci.runs | length' <<<"$record")" = 3 ]
  [ "$(jq -r '.next_action' <<<"$record")" = terminal-review ]
  invoke terminal-review-pass "$(evidence_fields)"
  accepted
  [ "$(jq -r '.next_action' <<<"$record")" = complete ]
}

@test "failed CI preserves its observations across a new run and recovery cannot reuse a terminal run" {
  remote_committed
  git -C "$repo" push -q origin HEAD:refs/heads/main
  invoke push-readback '{"remote":"origin","ref":"refs/heads/main"}'
  accepted
  invoke ci-register "$(ci_fields run-failed failure)"
  accepted
  [ "$(jq -r '.next_action' <<<"$record")" = enter-ci-recovery ]
  invoke ci-observe "$(ci_fields run-failed success)"
  [ "$status" -ne 0 ]
  invoke ci-register "$(ci_fields run-retry running)"
  accepted
  invoke ci-observe "$(ci_fields run-retry success)"
  accepted
  [ "$(jq '.ci.runs | length' <<<"$record")" = 3 ]
  [ "$(jq -r '.ci.runs[0].status' <<<"$record")" = failure ]
  [ "$(jq -r '.ci.terminal_success_run_id' <<<"$record")" = run-retry ]
  [ "$(jq -r '.next_action' <<<"$record")" = terminal-review ]
}

@test "publication interruption after rename recovers exactly one operation without rewriting authority" {
  shim="$BATS_TEST_TMPDIR/shim"
  mkdir "$shim"
  real_mv=$(command -v mv)
  cat >"$shim/mv" <<'SHIM'
#!/usr/bin/env bash
"$REAL_MV" "$@"
case "${!#}" in *.latest) kill -KILL "$PPID";; esac
SHIM
  chmod +x "$shim/mv"
  export REAL_MV="$real_mv"
  saved_path=$PATH
  export PATH="$shim:$PATH"
  invoke initialize '{"mode":"local-only","causal_edit":"implement fixture"}'
  [ "$status" -ne 0 ]
  [ -f "$target" ]
  [ -f "$target.pending-operation" ]
  before=$(sha256sum "$target")
  export PATH=$saved_path
  run bash -c 'cd "$1" && exec "$2" task op-0 initialize "$3"' _ "$repo" "$transition" "$records/request.json"
  [ "$status" -eq 0 ]
  [ "$(sha256sum "$target")" = "$before" ]
  [ ! -e "$target.pending-operation" ]
  [ "$(jq '.record.generation' <<<"$output")" = 0 ]
  printf '{}\n' >"$records/read.json"
  run bash -c 'cd "$1" && exec "$2" task op-0 read "$3"' _ "$repo" "$transition" "$records/read.json"
  [ "$status" -eq 0 ]
  [ "$(jq '.record.generation' <<<"$output")" = 0 ]
}

@test "concurrent retries share one operation receipt and one generation" {
  jq -cn '{expected_generation:0,expected_predecessor:null,mode:"local-only",causal_edit:"implement fixture"}' >"$records/request.json"
  run bash -c '
    cd "$1"
    "$2" task same initialize "$3" >"$4/first" & a=$!
    "$2" task same initialize "$3" >"$4/second" & b=$!
    wait "$a" && wait "$b"
  ' _ "$repo" "$transition" "$records/request.json" "$records"
  [ "$status" -eq 0 ]
  cmp "$records/first" "$records/second"
  [ "$(tail -c +15 "$target" | jq '.generation')" = 0 ]
}

@test "recording a commit rejects unreviewed source and never creates Git commits" {
  reviewed
  before=$(sha256sum "$target")
  invoke commit-success "$(evidence_fields '{"mode":"local-only"}')"
  [ "$status" -ne 0 ]
  [ "$(git -C "$repo" rev-parse HEAD)" = "$baseline" ]
  printf 'unreviewed change\n' >"$repo/source"
  git -C "$repo" add source
  git -C "$repo" commit -qm unreviewed
  invoke commit-success "$(evidence_fields '{"mode":"local-only"}')"
  [ "$status" -ne 0 ]
  [ "$(sha256sum "$target")" = "$before" ]
}

@test "exact verification retry accepts a metadata-only repair and rejects source-changing amendments" {
  committed
  invoke exact-verify-fail "$(evidence_fields)"
  accepted
  original_head=$(git -C "$repo" rev-parse HEAD)
  git -C "$repo" commit --amend -qm 'repair commit metadata' >"$evidence" 2>&1
  [ "$(git -C "$repo" rev-parse HEAD)" != "$original_head" ]
  invoke exact-verify-retry "$(evidence_fields)"
  accepted
  [ "$(jq -r '.delivery.commit_oid' <<<"$record")" = "$(git -C "$repo" rev-parse HEAD)" ]
  invoke exact-verify-fail "$(evidence_fields)"
  accepted
  before=$(sha256sum "$target")
  printf 'unverified source\n' >"$repo/source"
  git -C "$repo" add source
  git -C "$repo" commit --amend -qm 'source changed' >"$evidence" 2>&1
  invoke exact-verify-retry "$(evidence_fields)"
  [ "$status" -ne 0 ]
  [ "$(sha256sum "$target")" = "$before" ]
}

@test "reviewed untracked executable symlink and newline paths survive staging and commit" {
  initialize
  printf '#!/bin/sh\nexit 0\n' >"$repo/executable"
  chmod +x "$repo/executable"
  printf 'odd path\n' >"$repo/line
break"
  ln -s executable "$repo/link"
  invoke edit-pass "$(evidence_fields)"
  accepted
  invoke lightweight-review-pass "$(evidence_fields '{"route":"commit"}')"
  accepted
  printf '#!/bin/sh\necho actual-hook-pass\n' >"$repo/.git/hooks/pre-commit"
  chmod +x "$repo/.git/hooks/pre-commit"
  git -C "$repo" add .
  git -C "$repo" commit -m 'path fixture' >"$evidence" 2>&1
  invoke commit-success "$(evidence_fields '{"mode":"local-only"}')"
  accepted
  [ "$(jq -r '.next_action' <<<"$record")" = verify-exact-commit ]
}

@test "pre-publication interruption retries without treating a prepared operation as applied" {
  shim="$BATS_TEST_TMPDIR/shim"
  mkdir "$shim"
  real_mv=$(command -v mv)
  cat >"$shim/mv" <<'SHIM'
#!/usr/bin/env bash
case "${!#}" in *.latest) kill -KILL "$PPID"; exit 1;; esac
exec "$REAL_MV" "$@"
SHIM
  chmod +x "$shim/mv"
  export REAL_MV="$real_mv"
  saved_path=$PATH
  export PATH="$shim:$PATH"
  invoke initialize '{"mode":"local-only","causal_edit":"implement fixture"}'
  [ "$status" -ne 0 ]
  [ ! -f "$target" ]
  [ -f "$target.pending-operation" ]
  export PATH=$saved_path
  run bash -c 'cd "$1" && exec "$2" task op-0 initialize "$3"' _ "$repo" "$transition" "$records/request.json"
  [ "$status" -eq 0 ]
  [ "$(tail -c +15 "$target" | jq '.generation')" = 0 ]
  [ ! -f "$target.pending-operation" ]
}

@test "new CI registered before terminal review supersedes earlier green readiness" {
  remote_committed
  git -C "$repo" push -q origin HEAD:refs/heads/main
  invoke push-readback '{"remote":"origin","ref":"refs/heads/main"}'
  accepted
  invoke ci-register "$(ci_fields first-build success)"
  accepted
  invoke ci-register "$(ci_fields later-build running)"
  accepted
  [ "$(jq -r '.ci.terminal_success_run_id' <<<"$record")" = null ]
  invoke terminal-review-pass "$(evidence_fields)"
  [ "$status" -ne 0 ]
  invoke ci-observe "$(ci_fields later-build failure)"
  accepted
  [ "$(jq -r '.next_action' <<<"$record")" = enter-ci-recovery ]
  invoke ci-recovery "$(evidence_fields '{"causal_repair":"repair actual build failure"}')"
  accepted
  [ "$(jq -r '.state' <<<"$record")" = failing ]
  [ "$(jq '.ci.runs | length' <<<"$record")" = 3 ]
  [ "$(jq '[.gates[]] | all(. == null)' <<<"$record")" = true ]
}

@test "subdirectory calls reject commits containing unreviewed tracked or untracked source elsewhere" {
  mkdir -p "$repo/sub"
  printf 'baseline\n' >"$repo/sub/inside"
  git -C "$repo" add sub/inside
  git -C "$repo" commit -qm 'subdirectory fixture'
  baseline=$(git -C "$repo" rev-parse HEAD)
  invoke_cwd="$repo/sub"
  initialize
  printf 'reviewed\n' >"$repo/sub/inside"
  invoke edit-pass "$(evidence_fields)"
  accepted
  invoke lightweight-review-pass "$(evidence_fields '{"route":"commit"}')"
  accepted
  before=$(sha256sum "$target")
  printf 'unreviewed tracked\n' >"$repo/source"
  git -C "$repo" add .
  git -C "$repo" commit -m 'fixture with unreviewed outside tracked source' >"$evidence" 2>&1
  invoke commit-success "$(evidence_fields '{"mode":"local-only"}')"
  [ "$status" -ne 0 ]
  [ "$(sha256sum "$target")" = "$before" ]

  git -C "$repo" reset -q "$baseline"
  git -C "$repo" checkout -- source
  printf 'unreviewed untracked\n' >"$repo/outside-new"
  git -C "$repo" add .
  git -C "$repo" commit -m 'fixture with unreviewed outside new source' >"$evidence" 2>&1
  invoke commit-success "$(evidence_fields '{"mode":"local-only"}')"
  [ "$status" -ne 0 ]
  [ "$(sha256sum "$target")" = "$before" ]
}

@test "explicit remote delivery mode rebind preserves required verification and push gates" {
  remote_commit_mode=pull-request
  remote_committed
  [ "$(jq -r '.delivery.mode' <<<"$record")" = pull-request ]
  [ "$(jq -r '.next_action' <<<"$record")" = push ]
  [ "$(jq -r '.baseline_oid' <<<"$record")" = "$baseline" ]
}

@test "explicit PR to trunk rebind retains remote delivery gates" {
  remote_initialize_mode=pull-request
  remote_commit_mode=direct-to-trunk
  remote_committed
  [ "$(jq -r '.delivery.mode' <<<"$record")" = direct-to-trunk ]
  [ "$(jq -r '.next_action' <<<"$record")" = push ]
}

@test "delivery mode rebind cannot cross local and remote gate families" {
  reviewed
  git -C "$repo" add source
  git -C "$repo" commit -m 'actual local fixture' >"$evidence" 2>&1
  before=$(sha256sum "$target")
  invoke commit-success "$(evidence_fields '{"mode":"pull-request"}')"
  [ "$status" -ne 0 ]
  [ "$(sha256sum "$target")" = "$before" ]
}

@test "begin edit from pending CI clears credits preserves history and permits the next gated increment" {
  remote_committed
  git -C "$repo" push -q origin HEAD:refs/heads/main
  invoke push-readback '{"remote":"origin","ref":"refs/heads/main"}'
  accepted
  invoke ci-register "$(ci_fields pending-build running)"
  accepted
  first=$(jq -c '.ci.runs' <<<"$record")
  invoke begin-edit "$(evidence_fields '{"causal_edit":"repair reproduced delivery helper gap"}')" causal-increment
  accepted
  [ "$(jq -r '.state' <<<"$record")" = awaiting-causal-edit ]
  [ "$(jq -r '.next_action' <<<"$record")" = 'causal-edit: repair reproduced delivery helper gap' ]
  [ "$(jq -r '.baseline_oid' <<<"$record")" = "$baseline" ]
  [ "$(jq -c '.ci.runs' <<<"$record")" = "$first" ]
  [ "$(jq '[.test,.delivery,.ci.terminal_success_run_id,.gates[]] | all(. == null)' <<<"$record")" = true ]
  cp "$records/request.json" "$records/begin-request.json"
  before=$(sha256sum "$target")
  run bash -c 'cd "$1" && exec "$2" task causal-increment begin-edit "$3"' _ "$repo" "$transition" "$records/begin-request.json"
  [ "$status" -eq 0 ]
  [ "$(sha256sum "$target")" = "$before" ]
  invoke lightweight-review-pass "$(evidence_fields '{"route":"commit"}')"
  [ "$status" -ne 0 ]
  printf 'causal next increment\n' >"$repo/source"
  invoke edit-pass "$(evidence_fields)"
  accepted
  invoke lightweight-review-pass "$(evidence_fields '{"route":"commit"}')"
  accepted
  git -C "$repo" add source
  git -C "$repo" commit -m 'causal next increment' >"$evidence" 2>&1
  invoke commit-success "$(evidence_fields '{"mode":"pull-request"}')"
  accepted
  invoke exact-verify-pass "$(evidence_fields)"
  accepted
  git -C "$repo" push -q origin HEAD:refs/heads/main
  invoke push-readback '{"remote":"origin","ref":"refs/heads/main"}'
  accepted
  invoke ci-register "$(ci_fields next-build queued)"
  accepted
  [ "$(jq -c '.ci.runs[0:1]' <<<"$record")" = "$first" ]
  [ "$(jq '.ci.runs | length' <<<"$record")" = 2 ]
}

@test "begin edit rejects source drift unverified delivery and failed CI recovery holds" {
  remote_committed
  before=$(sha256sum "$target")
  invoke begin-edit "$(evidence_fields '{"causal_edit":"next increment"}')"
  [ "$status" -ne 0 ]
  [ "$(sha256sum "$target")" = "$before" ]
  git -C "$repo" push -q origin HEAD:refs/heads/main
  invoke push-readback '{"remote":"origin","ref":"refs/heads/main"}'
  accepted
  printf 'premature edit\n' >"$repo/source"
  before=$(sha256sum "$target")
  invoke begin-edit "$(evidence_fields '{"causal_edit":"next increment"}')"
  [ "$status" -ne 0 ]
  [ "$(sha256sum "$target")" = "$before" ]
  git -C "$repo" checkout -- source
  invoke ci-register "$(ci_fields failed-build failure)"
  accepted
  before=$(sha256sum "$target")
  invoke begin-edit "$(evidence_fields '{"causal_edit":"next increment"}')"
  [ "$status" -ne 0 ]
  [ "$(sha256sum "$target")" = "$before" ]
  invoke ci-recovery "$(evidence_fields '{"causal_repair":"fix actual CI failure"}')"
  accepted
}

@test "remote delivery mode cannot weaken to local-only at successful commit" {
  remote_reviewed
  git -C "$repo" add source
  git -C "$repo" commit -m 'actual remote fixture' >"$evidence" 2>&1
  before=$(sha256sum "$target")
  invoke commit-success "$(evidence_fields '{"mode":"local-only"}')"
  [ "$status" -ne 0 ]
  [ "$(sha256sum "$target")" = "$before" ]
  [[ "$output" == *'cannot cross local-only and remote delivery gates'* ]]
}

@test "begin edit after terminal success retires its current CI credit without losing observations" {
  remote_committed
  git -C "$repo" push -q origin HEAD:refs/heads/main
  invoke push-readback '{"remote":"origin","ref":"refs/heads/main"}'
  accepted
  invoke ci-register "$(ci_fields completed-build success)"
  accepted
  invoke terminal-review-pass "$(evidence_fields)"
  accepted
  first=$(jq -c '.ci.runs' <<<"$record")
  invoke begin-edit "$(evidence_fields '{"causal_edit":"next approved increment"}')"
  accepted
  [ "$(jq -c '.ci.runs' <<<"$record")" = "$first" ]
  [ "$(jq -r '.ci.terminal_success_run_id' <<<"$record")" = null ]
  [ "$(jq -r '.state' <<<"$record")" = awaiting-causal-edit ]
}

@test "begin edit after local delivery preserves the local gate family" {
  local_delivery
  invoke begin-edit "$(evidence_fields '{"causal_edit":"next approved local increment"}')"
  accepted
  [ "$(jq -r '.state' <<<"$record")" = awaiting-causal-edit ]
  [ "$(jq '.ci.runs | length' <<<"$record")" = 0 ]
  printf 'next local increment\n' >"$repo/source"
  invoke edit-pass "$(evidence_fields)"
  accepted
  invoke lightweight-review-pass "$(evidence_fields '{"route":"local-snapshot"}')"
  accepted
  [ "$(jq -r '.next_action' <<<"$record")" = fast-gate ]
}

@test "signed commits ignore signature display configuration in machine-readable ancestry queries" {
  reviewed
  ssh-keygen -q -t ed25519 -N '' -f "$records/signing-key"
  printf 'test@example.invalid %s\n' "$(cat "$records/signing-key.pub")" >"$records/allowed-signers"
  git -C "$repo" config gpg.format ssh
  git -C "$repo" config user.signingkey "$records/signing-key"
  git -C "$repo" config gpg.ssh.allowedSignersFile "$records/allowed-signers"
  git -C "$repo" config log.showSignature true
  printf '#!/bin/sh\necho actual-hook-passed\n' >"$repo/.git/hooks/pre-commit"
  chmod +x "$repo/.git/hooks/pre-commit"
  git -C "$repo" add source
  git -C "$repo" commit -S -m 'actually signed fixture' >"$evidence" 2>&1
  git -C "$repo" verify-commit HEAD >>"$evidence" 2>&1
  invoke commit-success "$(evidence_fields '{"mode":"local-only"}')"
  accepted
  [ "$(jq -r '.next_action' <<<"$record")" = verify-exact-commit ]
  invoke exact-verify-fail "$(evidence_fields)"
  accepted
  git -C "$repo" commit --amend -S -m 'repaired signed commit metadata' >"$evidence" 2>&1
  git -C "$repo" verify-commit HEAD >>"$evidence" 2>&1
  invoke exact-verify-retry "$(evidence_fields)"
  accepted
  [ "$(jq -r '.next_action' <<<"$record")" = verify-exact-commit ]
  [ "$(git -C "$repo" config log.showSignature)" = true ]
}

@test "begin edit after verified push does not invent a CI run when registration awaits PR" {
  remote_committed
  git -C "$repo" push -q origin HEAD:refs/heads/feature
  invoke push-readback '{"remote":"origin","ref":"refs/heads/feature"}'
  accepted
  [ "$(jq -r '.next_action' <<<"$record")" = register-exact-sha-ci-monitor ]
  invoke begin-edit "$(evidence_fields '{"causal_edit":"fix delivery helper before opening PR"}')"
  accepted
  [ "$(jq -r '.state' <<<"$record")" = awaiting-causal-edit ]
  [ "$(jq '.ci.runs | length' <<<"$record")" = 0 ]
  [ "$(jq -r '.ci.terminal_success_run_id' <<<"$record")" = null ]
  [ "$(jq -r '.baseline_oid' <<<"$record")" = "$baseline" ]
}
