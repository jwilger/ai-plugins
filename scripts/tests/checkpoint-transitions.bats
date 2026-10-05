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

@test "retained running CI failure can be observed after beginning the next increment" {
  remote_committed
  git -C "$repo" push -q origin HEAD:refs/heads/main
  invoke push-readback '{"remote":"origin","ref":"refs/heads/main"}'
  accepted
  invoke ci-register "$(ci_fields outstanding running)"
  accepted
  invoke begin-edit "$(evidence_fields '{"causal_edit":"next approved increment"}')"
  accepted
  invoke ci-observe "$(ci_fields outstanding failure)"
  accepted
  [ "$(jq -r '.ci.runs[-1].status' <<<"$record")" = failure ]
  [ "$(jq -r '.next_action' <<<"$record")" = enter-ci-recovery ]
  invoke ci-recovery "$(evidence_fields '{"causal_repair":"repair observed required CI failure"}')"
  accepted
}

@test "a failed required CI hold cannot be cleared by an undiagnosed retry" {
  remote_committed
  git -C "$repo" push -q origin HEAD:refs/heads/main
  invoke push-readback '{"remote":"origin","ref":"refs/heads/main"}'
  accepted
  invoke ci-register "$(ci_fields failed failure)"
  accepted
  invoke ci-register "$(ci_fields retry success)"
  [ "$status" -ne 0 ]
  [ "$(jq -r '.next_action' <<<"$(tail -c +15 "$target")")" = enter-ci-recovery ]
}

@test "terminal findings can enter causal repair while exact SHA CI registration is pending" {
  remote_committed
  git -C "$repo" push -q origin HEAD:refs/heads/main
  invoke push-readback '{"remote":"origin","ref":"refs/heads/main"}'
  accepted
  invoke terminal-review-remediation "$(evidence_fields '{"causal_repair":"repair independently confirmed terminal finding"}')"
  accepted
  [ "$(jq -r '.state' <<<"$record")" = failing ]
  [ "$(jq -r '.next_action' <<<"$record")" = 'causal-edit: repair independently confirmed terminal finding' ]
}

@test "causally repaired successor may complete CI while preserving historical failure" {
  remote_committed
  git -C "$repo" push -q origin HEAD:refs/heads/main
  invoke push-readback '{"remote":"origin","ref":"refs/heads/main"}'
  accepted
  failed_oid=$(git -C "$repo" rev-parse HEAD)
  invoke ci-register "$(ci_fields broken failure)"
  accepted
  invoke ci-recovery "$(evidence_fields '{"causal_repair":"fix actual source failure"}')"
  accepted
  printf 'causally repaired source\n' > "$repo/source"
  invoke edit-pass "$(evidence_fields)"
  accepted
  invoke lightweight-review-pass "$(evidence_fields '{"route":"commit"}')"
  accepted
  git -C "$repo" add source
  git -C "$repo" commit -m 'causal repair' > "$evidence" 2>&1
  invoke commit-success "$(evidence_fields '{"mode":"direct-to-trunk"}')"
  accepted
  invoke exact-verify-pass "$(evidence_fields)"
  accepted
  git -C "$repo" push -q origin HEAD:refs/heads/main
  invoke push-readback '{"remote":"origin","ref":"refs/heads/main"}'
  accepted
  invoke ci-register "$(ci_fields repaired success)"
  accepted
  [ "$(jq -r '.next_action' <<<"$record")" = terminal-review ]
  [ "$(jq -r '.ci.runs[0].commit_oid' <<<"$record")" = "$failed_oid" ]
  [ "$(jq -r '.ci.runs[0].status' <<<"$record")" = failure ]
}

@test "transient failure after terminal readiness requires fresh CI before returning" {
  remote_committed
  git -C "$repo" push -q origin HEAD:refs/heads/main
  invoke push-readback '{"remote":"origin","ref":"refs/heads/main"}'
  accepted
  invoke ci-register "$(ci_fields initial success)"
  accepted
  invoke ci-register "$(ci_fields later failure)"
  accepted
  invoke ci-retry-diagnosis "$(ci_fields later failure | jq 'del(.status) + {classification:"transient",rationale:"isolated infrastructure fault"}')"
  accepted
  [ "$(jq -r '.ci.terminal_success_run_id' <<<"$record")" = null ]
  [ "$(jq -r '.next_action' <<<"$record")" = monitor-exact-sha-ci ]
  invoke ci-register "$(ci_fields fresh success)"
  accepted
  [ "$(jq -r '.next_action' <<<"$record")" = terminal-review ]
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
  invoke ci-retry-diagnosis "$(ci_fields run-failed failure | jq 'del(.status) + {classification:"caused",rationale:"fixture diagnosis"}')"
  [ "$status" -ne 0 ]
  invoke ci-retry-diagnosis "$(ci_fields run-failed failure | jq 'del(.status) + {classification:"transient",rationale:"isolated fixture proves infrastructure failed without source change"}')"
  accepted
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

compatibility_failure() {
  local kind=$1
  generation=$(tail -c +15 "$target" | jq '.generation + 1')
  predecessor=$(sha256sum "$target" | cut -d ' ' -f 1)
  run bash -c 'cd "$1" && exec "$2" task "$3" "$4" "$5" "actual failed gate" "$6" "repair failed gate"' _ "$repo" "$ROOT/plugins/development-system/scripts/record-checkpoint-failure.sh" "$generation" "$predecessor" "$kind" "$evidence"
  accepted
}

compatibility_recovery() {
  local mode=$1 kind=$2
  if [ "$mode" = local-only ]; then passing; else
    remote_reviewed
    if [ "$kind" = lightweight-review ]; then
      invoke hook-failure "$(evidence_fields '{"causal_repair":"prepare another reviewed attempt"}')"
      accepted
      invoke edit-pass "$(evidence_fields)"
      accepted
    fi
  fi
  if [ "$kind" = pre-commit-hook ]; then
    if [ "$mode" = local-only ]; then
      invoke lightweight-review-pass "$(evidence_fields '{"route":"commit"}')"
      accepted
    fi
    printf '#!/bin/sh\necho actual-hook-failure >&2\nexit 1\n' >"$repo/.git/hooks/pre-commit"
    chmod +x "$repo/.git/hooks/pre-commit"
    git -C "$repo" add source
    run bash -c 'git -C "$1" commit -m failed >"$2" 2>&1' _ "$repo" "$evidence"
    [ "$status" -ne 0 ]
  else
    printf 'actual review finding: repair fixture\n' >"$evidence"
  fi
  compatibility_failure "$kind"
  printf '#!/bin/sh\necho repaired-hook\n' >"$repo/.git/hooks/pre-commit"
  chmod +x "$repo/.git/hooks/pre-commit"
  printf 'repaired source\n' >"$repo/source"
  invoke edit-pass "$(evidence_fields)"
  accepted
  invoke lightweight-review-pass "$(evidence_fields '{"route":"commit"}')"
  accepted
  git -C "$repo" add source
  git -C "$repo" commit -m repaired >"$evidence" 2>&1
  before=$(sha256sum "$target")
  if [ "$mode" != local-only ]; then
    invoke commit-success "$(evidence_fields '{"mode":"local-only"}')"
    [ "$status" -ne 0 ]
    [ "$(sha256sum "$target")" = "$before" ]
  fi
  invoke commit-success "$(evidence_fields "$(jq -cn --arg mode "$mode" '{mode:$mode}')")"
  accepted
}

@test "repair compatibility hook failure retains remote gate family" { compatibility_recovery direct-to-trunk pre-commit-hook; }
@test "repair compatibility review failure retains remote gate family" { compatibility_recovery direct-to-trunk lightweight-review; }
@test "repair compatibility hook failure retains genuine local mode" { compatibility_recovery local-only pre-commit-hook; }
@test "repair compatibility review failure retains genuine local mode" { compatibility_recovery local-only lightweight-review; }

@test "repair unknown legacy mode holds typed continuation without reusing older receipts" {
  passing
  # Publish the supported compatibility failure as a legacy record: its original
  # typed receipt is deliberately absent, but the older initialize receipt stays.
  rm "$target.operations/op-1.json"
  compatibility_failure lightweight-review
  before=$(sha256sum "$target")
  invoke edit-pass "$(evidence_fields)"
  [ "$status" -ne 0 ]
  [[ "$output" == *'delivery mode'*'compatibility'* ]]
  [ "$(sha256sum "$target")" = "$before" ]
}

@test "repair filtered snapshot preserves raw reviewed bytes across staging and commit" {
  printf '*.txt text eol=lf\n*.clean filter=fixture\n' >"$repo/.gitattributes"
  git -C "$repo" config filter.fixture.clean 'tr -d X'
  git -C "$repo" add .gitattributes
  git -C "$repo" commit -qm attributes
  initialize
  printf 'one\r\ntwo\r\n' >"$repo/new.txt"
  printf 'aXb\n' >"$repo/new.clean"
  printf 'executable\n' >"$repo/odd"$'\n\377'
  chmod +x "$repo/odd"$'\n\377'
  ln -s $'target\n' "$repo/link"
  invoke edit-pass "$(evidence_fields)"
  accepted
  [ "$(jq -r '.snapshot.untracked_sha256' <<<"$record")" = "$(legacy_untracked_digest)" ]
  raw_source=$(jq -r '.source_sha256' <<<"$output")
  printf 'one\ntwo\n' >"$repo/new.txt"
  invoke lightweight-review-pass "$(evidence_fields '{"route":"commit"}')"
  [ "$status" -ne 0 ]
  printf 'one\r\ntwo\r\n' >"$repo/new.txt"
  invoke lightweight-review-pass "$(evidence_fields '{"route":"commit"}')"
  accepted
  git -C "$repo" add .
  git -C "$repo" commit -qm filtered
  # Raw drift hidden by the clean filter must not inherit review credit.
  printf 'abX\n' >"$repo/new.clean"
  invoke commit-success "$(evidence_fields '{"mode":"local-only"}')"
  [ "$status" -ne 0 ]
  printf 'aXb\n' >"$repo/new.clean"
  invoke commit-success "$(evidence_fields '{"mode":"local-only"}')"
  accepted
  [ "$(jq -r '.source_sha256' <<<"$output")" = "$raw_source" ]
}

submodule_fixture() {
  child="$records/child"
  git init -q "$child"
  git -C "$child" config user.name Test
  git -C "$child" config user.email test@example.invalid
  printf 'child baseline\n' >"$child/file"
  git -C "$child" add file
  git -C "$child" commit -qm child
  git -C "$repo" -c protocol.file.allow=always submodule add -q "$child" child
  git -C "$repo/child" config user.name Test
  git -C "$repo/child" config user.email test@example.invalid
  git -C "$repo" commit -qam submodule
}

@test "repair clean gitlink uses actual child HEAD and survives parent staging" {
  submodule_fixture
  initialize
  printf 'child successor\n' >"$repo/child/file"
  git -C "$repo/child" commit -qam successor
  invoke edit-pass "$(evidence_fields)"
  accepted
  source_identity=$(jq -r '.source_sha256' <<<"$output")
  invoke lightweight-review-pass "$(evidence_fields '{"route":"commit"}')"
  accepted
  git -C "$repo" add child
  git -C "$repo" commit -qm successor
  invoke commit-success "$(evidence_fields '{"mode":"local-only"}')"
  accepted
  [ "$(jq -r '.source_sha256' <<<"$output")" = "$source_identity" ]
}

@test "repair gitlink dirty untracked missing and different child HEAD hold without publishing" {
  submodule_fixture
  initialize
  printf 'parent edit\n' >"$repo/source"
  invoke edit-pass "$(evidence_fields)"
  accepted
  before=$(sha256sum "$target")
  git -C "$repo" config submodule.child.ignore all
  printf 'dirty child\n' >"$repo/child/file"
  invoke lightweight-review-pass "$(evidence_fields '{"route":"commit"}')"
  [ "$status" -ne 0 ]
  [[ "$output" == *'submodule'*'dirty'* ]]
  git -C "$repo/child" restore file
  touch "$repo/child/untracked"
  invoke lightweight-review-pass "$(evidence_fields '{"route":"commit"}')"
  [ "$status" -ne 0 ]
  rm "$repo/child/untracked"
  printf 'unreviewed child\n' >"$repo/child/file"
  git -C "$repo/child" commit -qam drift
  invoke lightweight-review-pass "$(evidence_fields '{"route":"commit"}')"
  [ "$status" -ne 0 ]
  git -C "$repo" submodule deinit -q -f child
  invoke lightweight-review-pass "$(evidence_fields '{"route":"commit"}')"
  [ "$status" -ne 0 ]
  [[ "$output" == *'submodule'*'initialize'* ]]
  [ "$(sha256sum "$target")" = "$before" ]
}

@test "repair source hashing bounds regular file reads and equals raw Git blobs" {
  initialize
  dd if=/dev/zero of="$repo/large" bs=1048576 count=3 status=none
  cat >"$records/bounded.cjs" <<'JS'
const fs = require('node:fs');
const original = fs.readFileSync;
fs.readFileSync = function (file, ...args) {
  if (String(file).endsWith('/large')) throw new Error('whole source file allocation forbidden');
  return original.call(this, file, ...args);
};
const read = fs.readSync;
fs.readSync = function (fd, buffer, offset, length, position) {
  if (fs.readlinkSync(`/proc/self/fd/${fd}`).endsWith('/large')) {
    if (buffer.length > 65536 || length > 65536) throw new Error('unbounded source chunk');
    fs.appendFileSync(process.env.CHUNK_LOG, `${length}\n`);
  }
  return read.call(this, fd, buffer, offset, length, position);
};
JS
  export NODE_OPTIONS="--require=$records/bounded.cjs" CHUNK_LOG="$records/chunks"
  invoke edit-pass "$(evidence_fields)"
  accepted
  [ "$(wc -l <"$CHUNK_LOG")" -ge 48 ]
  actual=$(jq -r '.source_sha256' <<<"$output")
  expected=$( { for name in large source; do
    printf '100644\0%s\0%s\n' "$name" "$(git -C "$repo" hash-object --no-filters -- "$name")"
  done; } | sha256sum | cut -d ' ' -f 1)
  [ "$actual" = "$expected" ]
}

@test "repair compatibility stable retry recovers interrupted receipt publication and preserves CAS" {
  remote_reviewed
  compatibility_failure pre-commit-hook
  failure_generation=$generation
  failure_predecessor=$predecessor
  failure_target=$(sha256sum "$target")
  receipt=$(find "$target.operations" -name 'compat-failure-*.json')
  [ -n "$receipt" ]
  # Reconstruct the real writer crash window after publishing .latest but before
  # moving its durable operation intent to the retained receipt directory.
  mv "$receipt" "$target.pending-operation"
  run bash -c 'cd "$1" && exec "$2" task "$3" "$4" pre-commit-hook "actual failed gate" "$5" "repair failed gate"' _ "$repo" "$ROOT/plugins/development-system/scripts/record-checkpoint-failure.sh" "$failure_generation" "$failure_predecessor" "$evidence"
  [ "$status" -eq 0 ]
  [ ! -e "$target.pending-operation" ]
  [ -e "$receipt" ]
  [ "$(sha256sum "$target")" = "$failure_target" ]
  invoke edit-pass "$(evidence_fields)"
  accepted
  before=$(sha256sum "$target")
  run bash -c 'cd "$1" && exec "$2" task "$3" "$4" pre-commit-hook "actual failed gate" "$5" "repair failed gate"' _ "$repo" "$ROOT/plugins/development-system/scripts/record-checkpoint-failure.sh" "$failure_generation" "$failure_predecessor" "$evidence"
  [ "$status" -eq 0 ]
  [ "$(sha256sum "$target")" = "$before" ]
  run bash -c 'cd "$1" && exec "$2" task "$3" "$4" pre-commit-hook "different request" "$5" "repair failed gate"' _ "$repo" "$ROOT/plugins/development-system/scripts/record-checkpoint-failure.sh" "$failure_generation" "$failure_predecessor" "$evidence"
  [ "$status" -ne 0 ]
  [ "$(sha256sum "$target")" = "$before" ]
}

@test "repair nested submodule dirt cannot hide behind ignore settings" {
  submodule_fixture
  git -C "$repo/child" -c protocol.file.allow=always submodule add -q "$child" nested
  git -C "$repo/child" commit -qam nested
  git -C "$repo" commit -qam nested
  initialize
  printf 'parent edit\n' >"$repo/source"
  invoke edit-pass "$(evidence_fields)"
  accepted
  git -C "$repo" config submodule.child.ignore all
  git -C "$repo/child" config submodule.nested.ignore all
  printf 'nested dirt\n' >"$repo/child/nested/file"
  before=$(sha256sum "$target")
  invoke lightweight-review-pass "$(evidence_fields '{"route":"commit"}')"
  [ "$status" -ne 0 ]
  [[ "$output" == *'submodule'*'dirty'* ]]
  [ "$(sha256sum "$target")" = "$before" ]
}

@test "repair SHA256 raw identities preserve executable symlink and odd path bytes" {
  repo="$BATS_TEST_TMPDIR/sha256"
  git init -q --object-format=sha256 "$repo"
  git -C "$repo" config user.name Test
  git -C "$repo" config user.email test@example.invalid
  printf 'baseline\n' >"$repo/source"
  git -C "$repo" add source
  git -C "$repo" commit -qm baseline
  target="$repo/.git/development-system/checkpoints/task.latest"
  initialize
  odd=$'odd\n\377'
  printf 'binary\0payload\n' >"$repo/$odd"
  chmod +x "$repo/$odd"
  ln -s $'target\n' "$repo/link"
  invoke edit-pass "$(evidence_fields)"
  accepted
  [ "$(jq -r '.snapshot.untracked_sha256' <<<"$record")" = "$(legacy_untracked_digest)" ]
  actual=$(jq -r '.source_sha256' <<<"$output")
  expected=$( {
    printf '120000\0link\0%s\n' "$(printf 'target\n' | git -C "$repo" hash-object --stdin)"
    printf '100755\0%s\0%s\n' "$odd" "$(git -C "$repo" hash-object --no-filters -- "$odd")"
    printf '100644\0source\0%s\n' "$(git -C "$repo" hash-object --no-filters -- source)"
  } | sha256sum | cut -d ' ' -f 1)
  [ "$actual" = "$expected" ]
}

@test "repair changing a file during chunk reading holds publication" {
  initialize
  dd if=/dev/zero of="$repo/large" bs=1048576 count=1 status=none
  cat >"$records/change-during-read.cjs" <<'JS'
const fs = require('node:fs');
const read = fs.readSync;
let changed = false;
fs.readSync = function (fd, ...args) {
  const count = read.call(this, fd, ...args);
  const name = fs.readlinkSync(`/proc/self/fd/${fd}`);
  if (!changed && name.endsWith('/large')) {
    changed = true;
    fs.truncateSync(name, 0);
  }
  return count;
};
JS
  export NODE_OPTIONS="--require=$records/change-during-read.cjs"
  before=$(sha256sum "$target")
  invoke edit-pass "$(evidence_fields)"
  [ "$status" -ne 0 ]
  [[ "$output" == *'source file changed while hashing'* ]]
  [ "$(sha256sum "$target")" = "$before" ]
}

# Independent checkpoint-v1 oracle: the publisher's original shell algorithm,
# using path-aware Git hashes and raw symlink targets rather than the JS helper.
legacy_untracked_digest() {
  git -C "$repo" ls-files --full-name --others --exclude-standard -z |
    while IFS= read -r -d '' name; do
      if [ -L "$repo/$name" ]; then
        mode=120000
        oid=$(node -e 'process.stdout.write(require("node:fs").readlinkSync(process.argv[1], {encoding:"buffer"}))' "$repo/$name" | git -C "$repo" hash-object --stdin)
      else
        mode=100644
        [ ! -x "$repo/$name" ] || mode=100755
        oid=$(git -C "$repo" hash-object -- "$name")
      fi
      printf '%s\0%s\0%s\n' "$mode" "$name" "$oid"
    done | sha256sum | cut -d ' ' -f 1
}

@test "repair canonical mode retains legacy execute access while raw mode binds execute bits" {
  initialize
  printf 'group execute only\n' >"$repo/group-only"
  chmod 0654 "$repo/group-only"
  [ ! -x "$repo/group-only" ]
  invoke edit-pass "$(evidence_fields)"
  accepted
  [ "$(jq -r '.snapshot.untracked_sha256' <<<"$record")" = "$(legacy_untracked_digest)" ]
  expected=$( {
    printf '100755\0group-only\0%s\n' "$(git -C "$repo" hash-object --no-filters -- group-only)"
    printf '100644\0source\0%s\n' "$(git -C "$repo" hash-object --no-filters -- source)"
  } | sha256sum | cut -d ' ' -f 1)
  [ "$(jq -r '.source_sha256' <<<"$output")" = "$expected" ]
}

@test "repair normalized raw edit during publication rejects stale review credit" {
  printf '*.txt text eol=lf\n' >"$repo/.gitattributes"
  git -C "$repo" add .gitattributes
  git -C "$repo" commit -qm attributes
  initialize
  printf 'text\n' >"$repo/new.txt"
  invoke edit-pass "$(evidence_fields)"
  accepted
  real_node=$(command -v node)
  mkdir "$records/wrappers"
  cat >"$records/wrappers/node" <<'SH'
#!/usr/bin/env bash
if [[ ${2:-} == stage && -e $RACE_MARKER ]]; then
  rm "$RACE_MARKER"
  printf 'text\r\n' >"$RACE_SOURCE"
fi
exec "$REAL_NODE" "$@"
SH
  chmod +x "$records/wrappers/node"
  export REAL_NODE="$real_node" RACE_MARKER="$records/race" RACE_SOURCE="$repo/new.txt"
  export PATH="$records/wrappers:$PATH"
  touch "$RACE_MARKER"
  before=$(sha256sum "$target")
  invoke lightweight-review-pass "$(evidence_fields '{"route":"commit"}')"
  [ "$status" -ne 0 ]
  [[ "$output" == *'raw source changed while publishing'* ]]
  [ "$(sha256sum "$target")" = "$before" ]
  [ ! -e "$target.pending-operation" ]
  # Record the real publication failure and rerun testing/review on the changed
  # bytes. A rejected review cannot silently credit the new content.
  printf 'publication rejected normalized concurrent source edit\n' >"$evidence"
  invoke lightweight-review-fail "$(evidence_fields '{"causal_repair":"retest changed raw source"}')"
  accepted
  invoke edit-pass "$(evidence_fields)"
  accepted
  invoke lightweight-review-pass "$(evidence_fields '{"route":"commit"}')"
  accepted
}

@test "repair invalid UTF8 gitlink paths retain clean child identity through nested review and staging" {
  export TMPDIR="$records/gitlink-temp"
  mkdir "$TMPDIR"
  child_name=$'child-\377'
  nested_name=$'nested-\376'
  child_path="$repo/$child_name"
  nested_path="$child_path/$nested_name"
  git init -q "$child_path"
  git -C "$child_path" config user.name Test
  git -C "$child_path" config user.email test@example.invalid
  printf 'child source\n' >"$child_path/file"
  git init -q "$nested_path"
  git -C "$nested_path" config user.name Test
  git -C "$nested_path" config user.email test@example.invalid
  printf 'nested source\n' >"$nested_path/file"
  git -C "$nested_path" add file
  git -C "$nested_path" commit -qm nested
  git -C "$child_path" add .
  git -C "$child_path" commit -qm child
  git -C "$repo" add -- "$child_name"
  git -C "$repo" commit -qm gitlink
  [ -z "$(git -C "$repo" status --porcelain)" ]
  [ -z "$(git -C "$child_path" status --porcelain)" ]
  initialize
  [ -z "$(find "$TMPDIR" -mindepth 1 -print -quit)" ]
  printf 'reviewed child successor\n' >"$child_path/file"
  git -C "$child_path" commit -qam successor
  invoke edit-pass "$(evidence_fields)"
  accepted
  reviewed_source=$(jq -r '.source_sha256' <<<"$output")
  before=$(sha256sum "$target")
  printf 'nested dirty\n' >"$nested_path/file"
  invoke lightweight-review-pass "$(evidence_fields '{"route":"commit"}')"
  [ "$status" -ne 0 ]
  [[ "$output" == *'submodule'*'dirty'* ]]
  [ "$(sha256sum "$target")" = "$before" ]
  [ -z "$(find "$TMPDIR" -mindepth 1 -print -quit)" ]
  git -C "$nested_path" restore file
  invoke lightweight-review-pass "$(evidence_fields '{"route":"commit"}')"
  accepted
  git -C "$repo" add -- "$child_name"
  git -C "$repo" commit -qm successor
  invoke commit-success "$(evidence_fields '{"mode":"local-only"}')"
  accepted
  [ "$(jq -r '.source_sha256' <<<"$output")" = "$reviewed_source" ]
  [ -z "$(find "$TMPDIR" -mindepth 1 -print -quit)" ]
}

ignored_gitlink_review() {
  if [ "${1:-sha1}" = sha256 ]; then
    export GIT_DEFAULT_HASH=sha256
    repo="$BATS_TEST_TMPDIR/sha256-repo"
    git init -q "$repo"
    git -C "$repo" config user.name Test
    git -C "$repo" config user.email test@example.invalid
    printf 'baseline\n' >"$repo/source"
    git -C "$repo" add source
    git -C "$repo" commit -qm baseline
    target="$repo/.git/development-system/checkpoints/task.latest"
  fi
  submodule_fixture
  gitlink_name=child
  if [ "${1:-sha1}" = sha256 ]; then
    gitlink_name=$'child-\377'
    git -C "$repo" mv child "$gitlink_name"
    git -C "$repo" commit -qam 'rename child'
  fi
  git -C "$repo" config submodule.child.ignore all
  initialize
  printf 'reviewed child successor\n' >"$repo/$gitlink_name/file"
  git -C "$repo/$gitlink_name" commit -qam successor
  child_head=$(git -C "$repo/$gitlink_name" rev-parse HEAD)
  printf 'parent change\n' >"$repo/source"
  invoke edit-pass "$(evidence_fields)"
  accepted
  invoke lightweight-review-pass "$(evidence_fields '{"route":"commit"}')"
  accepted
  git -C "$repo" add source
  git -C "$repo" commit -qm 'parent file only'
  [ -z "$(git -C "$repo" diff --binary --full-index HEAD --)" ]
}

ignored_gitlink_commit_control() {
  ignored_gitlink_review "$1"
  before=$(sha256sum "$target")
  invoke commit-success "$(evidence_fields '{"mode":"local-only"}')"
  [ "$status" -ne 0 ]
  [[ "$output" == *'committed gitlink'* ]]
  [ "$(sha256sum "$target")" = "$before" ]
  git -C "$repo" add -- "$gitlink_name"
  # Staging does not repair the already-created commit.
  invoke commit-success "$(evidence_fields '{"mode":"local-only"}')"
  [ "$status" -ne 0 ]
  [ "$(sha256sum "$target")" = "$before" ]
  # Amend only this disposable fixture's rejected attempt; the accepted commit
  # must still be a direct successor of the reviewed checkpoint's HEAD.
  git -C "$repo" commit --amend --no-edit -q
  invoke commit-success "$(evidence_fields '{"mode":"local-only"}')"
  accepted
  invoke exact-verify-pass "$(evidence_fields)"
  accepted
  invoke local-delivery '{}'
  accepted
}

@test "repair ignored reviewed gitlink omitted from parent commit rejects until actually committed" {
  ignored_gitlink_commit_control sha1
}

@test "repair ignored SHA256 odd-byte gitlink requires the authoritative committed child OID" {
  ignored_gitlink_commit_control sha256
}

@test "repair ignored child drift cannot initialize an uncommitted baseline" {
  submodule_fixture
  git -C "$repo" config submodule.child.ignore all
  printf 'uncommitted child reference\n' >"$repo/child/file"
  git -C "$repo/child" commit -qam successor
  invoke initialize '{"mode":"local-only","causal_edit":"new work"}'
  [ "$status" -ne 0 ]
  [[ "$output" == *'committed gitlink'* ]]
  [ ! -e "$target" ]
  git -C "$repo" add child
  git -C "$repo" commit -qm child
  initialize
}

@test "repair legacy omitted gitlink cannot gain exact verification but can record and repair failure" {
  ignored_gitlink_review sha1
  generation=$(jq '.generation + 1' <<<"$record")
  predecessor=$(sha256sum "$target" | cut -d ' ' -f 1)
  head=$(git -C "$repo" rev-parse HEAD)
  empty=e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
  # The compatibility API can hold records created before this repair. Publish
  # such a legacy committed record using the actual omitted-gitlink commit.
  legacy=$(jq -c --argjson generation "$generation" --arg predecessor "$predecessor" --arg head "$head" --arg empty "$empty" --arg evidence "$evidence" '
    .generation=$generation | .predecessor_sha256=$predecessor |
    .snapshot={head_oid:$head,tracked_sha256:$empty,untracked_sha256:$empty} |
    .state="committed" | .gates.fast_gate_receipt=$evidence |
    .delivery={mode:"local-only",commit_oid:$head,pushed_oid:null,local_snapshot:null} |
    .next_action="verify-exact-commit"' <<<"$record")
  printf 'checkpoint-v1 %s\n' "$legacy" >"$records/legacy"
  run bash -c 'cd "$1" && exec "$2" task "$3" "$4" "$5"' _ "$repo" "$ROOT/plugins/development-system/scripts/write-local-checkpoint.sh" "$generation" "$predecessor" "$records/legacy"
  accepted
  before=$(sha256sum "$target")
  invoke exact-verify-pass "$(evidence_fields)"
  [ "$status" -ne 0 ]
  [[ "$output" == *'committed gitlink'* ]]
  [ "$(sha256sum "$target")" = "$before" ]
  invoke exact-verify-fail "$(evidence_fields)"
  accepted
  before=$(sha256sum "$target")
  invoke exact-verify-retry "$(evidence_fields)"
  [ "$status" -ne 0 ]
  [ "$(sha256sum "$target")" = "$before" ]
  git -C "$repo" add child
  git -C "$repo" commit --amend --no-edit -q
  invoke exact-verify-retry "$(evidence_fields)"
  accepted
  invoke exact-verify-pass "$(evidence_fields)"
  accepted
}

committed_gitlink_shape() {
  submodule_fixture
  git -C "$repo" config submodule.child.ignore all
  initialize
  case "$1" in
    added)
      git -C "$repo" -c protocol.file.allow=always submodule add -q "$child" second
      git -C "$repo" config submodule.second.ignore all
      ;;
    removed) git -C "$repo" rm -q child ;;
    renamed) git -C "$repo" mv child $'renamed-\377' ;;
  esac
  invoke edit-pass "$(evidence_fields)"
  accepted
  invoke lightweight-review-pass "$(evidence_fields '{"route":"commit"}')"
  accepted
  reviewed_source=$(jq -r '.source_sha256' <<<"$output")
  git -C "$repo" commit -qam "$1 gitlink"
  invoke commit-success "$(evidence_fields '{"mode":"local-only"}')"
  accepted
  [ "$(jq -r '.source_sha256' <<<"$output")" = "$reviewed_source" ]
}

@test "repair committed gitlink check accepts actually added references" { committed_gitlink_shape added; }
@test "repair committed gitlink check accepts actually removed references" { committed_gitlink_shape removed; }
@test "repair committed gitlink check accepts actually renamed byte paths" { committed_gitlink_shape renamed; }

@test "repair local snapshot may deliver reviewed child before a parent commit" {
  submodule_fixture
  git -C "$repo" config submodule.child.ignore all
  initialize
  printf 'local child successor\n' >"$repo/child/file"
  git -C "$repo/child" commit -qam successor
  invoke edit-pass "$(evidence_fields)"
  accepted
  invoke lightweight-review-pass "$(evidence_fields '{"route":"local-snapshot"}')"
  accepted
  invoke fast-gate-pass "$(evidence_fields)"
  accepted
  invoke local-snapshot-delivery "$(evidence_fields)"
  accepted
  invoke terminal-review-pass "$(evidence_fields)"
  accepted
  [ "$(jq -r '.delivery.commit_oid' <<<"$record")" = null ]
  [ "$(jq -r '.next_action' <<<"$record")" = complete ]
}

@test "repair committed gitlink check catches ignored index drift during publication" {
  ignored_gitlink_review sha1
  old_child=$(git -C "$repo" rev-parse HEAD:child)
  git -C "$repo" add child
  git -C "$repo" commit --amend --no-edit -q
  mkdir "$records/wrappers"
  real_node=$(command -v node)
  cat >"$records/wrappers/node" <<'SH'
#!/usr/bin/env bash
if [[ ${2:-} == stage && -e $GITLINK_RACE_MARKER ]]; then
  rm "$GITLINK_RACE_MARKER"
  git -C "$GITLINK_RACE_REPO" update-index --cacheinfo "160000,$GITLINK_RACE_OLD,child"
fi
exec "$REAL_NODE" "$@"
SH
  chmod +x "$records/wrappers/node"
  export REAL_NODE="$real_node" GITLINK_RACE_MARKER="$records/race"
  export GITLINK_RACE_REPO="$repo" GITLINK_RACE_OLD="$old_child"
  export PATH="$records/wrappers:$PATH"
  touch "$GITLINK_RACE_MARKER"
  before=$(sha256sum "$target")
  invoke commit-success "$(evidence_fields '{"mode":"local-only"}')"
  [ "$status" -ne 0 ]
  [[ "$output" == *'committed gitlinks'* ]]
  [ "$(sha256sum "$target")" = "$before" ]
  [ ! -e "$target.pending-operation" ]
  git -C "$repo" reset -q -- child
  invoke commit-success "$(evidence_fields '{"mode":"local-only"}')"
  accepted
}

@test "historical failure diagnosis restores registration for an unregistered successor" {
  remote_committed
  git -C "$repo" push -q origin HEAD:refs/heads/main
  invoke push-readback '{"remote":"origin","ref":"refs/heads/main"}'
  accepted
  old_oid=$(git -C "$repo" rev-parse HEAD)
  invoke ci-register "$(ci_fields old running)"
  accepted
  invoke begin-edit "$(evidence_fields '{"causal_edit":"next increment"}')"
  accepted
  printf 'new successor source\n' > "$repo/source"
  invoke edit-pass "$(evidence_fields)"
  accepted
  invoke lightweight-review-pass "$(evidence_fields '{"route":"commit"}')"
  accepted
  git -C "$repo" add source
  git -C "$repo" commit -m successor > "$evidence" 2>&1
  invoke commit-success "$(evidence_fields '{"mode":"direct-to-trunk"}')"
  accepted
  invoke exact-verify-pass "$(evidence_fields)"
  accepted
  git -C "$repo" push -q origin HEAD:refs/heads/main
  invoke push-readback '{"remote":"origin","ref":"refs/heads/main"}'
  accepted
  old_fields=$(ci_fields old failure | jq --arg oid "$old_oid" '.commit_oid=$oid')
  invoke ci-observe "$old_fields"
  accepted
  invoke ci-retry-diagnosis "$(jq 'del(.status) + {classification:"unrelated",rationale:"old run infrastructure failure"}' <<< "$old_fields")"
  accepted
  [ "$(jq -r '.next_action' <<< "$record")" = register-exact-sha-ci-monitor ]
}

@test "compatibility gate progress cannot delete existing CI diagnoses" {
  remote_committed
  git -C "$repo" push -q origin HEAD:refs/heads/main
  invoke push-readback '{"remote":"origin","ref":"refs/heads/main"}'
  accepted
  invoke ci-register "$(ci_fields failed failure)"
  accepted
  invoke ci-recovery "$(evidence_fields '{"causal_repair":"repair diagnosed source failure"}')"
  accepted
  invoke edit-pass "$(evidence_fields)"
  accepted
  generation=$(jq '.generation+1' <<< "$record")
  predecessor=$(sha256sum "$target" | cut -d ' ' -f 1)
  { printf 'checkpoint-v1 '; jq -c --argjson gen "$generation" --arg pred "$predecessor" --arg receipt "$evidence" 'del(.ci.recoveries) | .generation=$gen | .predecessor_sha256=$pred | .gates.lightweight_review_receipt=$receipt | .next_action="commit-through-pre-commit-hook"' <<< "$record"; } > "$records/drop-diagnosis.json"
  run bash -c 'cd "$1" && exec "$2" task "$3" "$4" "$5"' _ "$repo" "${DIAGNOSIS_REPRO_WRITER:-$ROOT/plugins/development-system/scripts/write-local-checkpoint.sh}" "$generation" "$predecessor" "$records/drop-diagnosis.json"
  [ "$status" -ne 0 ]
  [ "$(sha256sum "$target" | cut -d ' ' -f 1)" = "$predecessor" ]
}

@test "compatibility CI observation cannot skip the distinct terminal review" {
  remote_committed
  git -C "$repo" push -q origin HEAD:refs/heads/main
  invoke push-readback '{"remote":"origin","ref":"refs/heads/main"}'
  accepted
  invoke ci-register "$(ci_fields held running)"
  accepted
  generation=$(jq '.generation+1' <<< "$record")
  predecessor=$(sha256sum "$target" | cut -d ' ' -f 1)
  { printf 'checkpoint-v1 '; jq -c --argjson gen "$generation" --arg pred "$predecessor" '.generation=$gen | .predecessor_sha256=$pred | .ci.runs += [(.ci.runs[-1] | .status="success")] | .ci.terminal_success_run_id="held" | .next_action="complete"' <<< "$record"; } > "$records/skip-terminal.json"
  run bash -c 'cd "$1" && exec "$2" task "$3" "$4" "$5"' _ "$repo" "$ROOT/plugins/development-system/scripts/write-local-checkpoint.sh" "$generation" "$predecessor" "$records/skip-terminal.json"
  [ "$status" -ne 0 ]
}

compatibility_stale_ci_success() {
  remote_committed
  git -C "$repo" push -q origin HEAD:refs/heads/main
  invoke push-readback '{"remote":"origin","ref":"refs/heads/main"}'
  accepted
  invoke ci-register "$(ci_fields prior success)"
  accepted
  generation=$(jq '.generation+1' <<< "$record")
  predecessor=$(sha256sum "$target" | cut -d ' ' -f 1)
  before=$(sha256sum "$target")
  { printf 'checkpoint-v1 '; jq -c --argjson gen "$generation" --arg pred "$predecessor" --arg status "$1" '.generation=$gen | .predecessor_sha256=$pred | .ci.runs += [(.ci.runs[-1] | .run_id="newer" | .status=$status)]' <<< "$record"; } > "$records/stale-ci-success.json"
  run bash -c 'cd "$1" && exec "$2" task "$3" "$4" "$5"' _ "$repo" "$ROOT/plugins/development-system/scripts/write-local-checkpoint.sh" "$generation" "$predecessor" "$records/stale-ci-success.json"
  [ "$status" -ne 0 ]
  [ "$(sha256sum "$target")" = "$before" ]
}

@test "compatibility newer running CI invalidates older success credit" {
  compatibility_stale_ci_success running
}

@test "compatibility newer failed CI preempts older success credit" {
  compatibility_stale_ci_success failure
}
