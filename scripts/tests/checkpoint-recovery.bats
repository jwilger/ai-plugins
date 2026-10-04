#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  writer="$ROOT/plugins/development-system/scripts/write-local-checkpoint.sh"
  recovery="$ROOT/plugins/development-system/scripts/record-checkpoint-failure.sh"
  repo="$BATS_TEST_TMPDIR/repo"
  records="$BATS_TEST_TMPDIR/records"
  mkdir -p "$repo" "$records"
  export GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
  git -C "$repo" init -q
  git -C "$repo" config user.name Test
  git -C "$repo" config user.email test@example.invalid
  git -C "$repo" config commit.gpgsign false
  git -C "$repo" config core.hooksPath "$repo/.git/hooks"
  printf 'baseline\n' >"$repo/tracked.txt"
  git -C "$repo" add tracked.txt
  git -C "$repo" commit -qm baseline
  baseline=$(git -C "$repo" rev-parse HEAD)
  target="$repo/.git/development-system/checkpoints/repair.latest"
  proposal="$records/proposal"
  evidence="$records/failure.log"
  empty=e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
  record=$(jq -cn --arg head "$baseline" --arg empty "$empty" '{generation:0,predecessor_sha256:null,baseline_oid:$head,snapshot:{head_oid:$head,tracked_sha256:$empty,untracked_sha256:$empty},state:"pushed-or-delivery-mode-equivalent",test:null,gates:{lightweight_review_receipt:null,fast_gate_receipt:null,exact_identity_verification_receipt:null},delivery:{mode:"local-only",commit_oid:null,pushed_oid:null,local_snapshot:"clean baseline"},ci:{runs:[],terminal_success_run_id:null},next_action:"causal-edit: implement regression fixture"}')
  printf 'checkpoint-v1 %s\n' "$record" >"$proposal"
  run bash -c 'cd "$1" && exec "$2" repair 0 null "$3"' _ "$repo" "$writer" "$proposal"
  [ "$status" -eq 0 ]
  read_checkpoint
}

read_checkpoint() {
  record=$(tail -c +15 "$target")
  generation=$(jq -r '.generation + 1' <<<"$record")
  predecessor=$(sha256sum "$target" | cut -d ' ' -f 1)
}

make_proposal() {
  head=$(git -C "$repo" rev-parse HEAD)
  tracked=$(git -C "$repo" diff --binary --full-index HEAD -- | sha256sum | cut -d ' ' -f 1)
  untracked=$(git -C "$repo" ls-files --others --exclude-standard -z | while IFS= read -r -d '' path; do
    mode=100644
    [ ! -x "$repo/$path" ] || mode=100755
    printf '%s\0%s\0%s\n' "$mode" "$path" "$(git -C "$repo" hash-object -- "$path")"
  done | sha256sum | cut -d ' ' -f 1)
  candidate=$(jq -c --argjson generation "$generation" --arg predecessor "$predecessor" --arg head "$head" --arg tracked "$tracked" --arg untracked "$untracked" --arg evidence "$evidence" \
    ".generation = \$generation | .predecessor_sha256 = \$predecessor | .snapshot = {head_oid:\$head,tracked_sha256:\$tracked,untracked_sha256:\$untracked} | $1" <<<"$record")
  printf 'checkpoint-v1 %s\n' "$candidate" >"$proposal"
}

publish() {
  run bash -c 'cd "$1" && exec "$2" repair "$3" "$4" "$5"' _ "$repo" "$writer" "$generation" "$predecessor" "$proposal"
}

accept() {
  publish
  if [ "$status" -ne 0 ]; then echo "$output"; return 1; fi
  read_checkpoint
}

reject() {
  before=$(sha256sum "$target")
  publish
  [ "$status" -ne 0 ]
  [ "$(sha256sum "$target")" = "$before" ]
}

await_review() {
  printf 'broken\n' >"$repo/tracked.txt"
  make_proposal '.state = "passing-awaiting-gates-or-review" | .test = {command:"focused check",receipt_ref:"fresh focused check passed",outcome:"pass",failure_kind:null} | .gates |= map_values(null) | .delivery = null | .next_action = "lightweight-review"'
  accept
}

await_commit() {
  await_review
  make_proposal '.gates.lightweight_review_receipt = "bounded review passed" | .next_action = "commit-through-pre-commit-hook"'
  accept
}

fail_commit() {
  cat >"$repo/.git/hooks/pre-commit" <<'HOOK'
#!/bin/sh
if ! grep -qx repaired tracked.txt; then
  echo 'required check failed: tracked.txt must contain repaired' >&2
  exit 1
fi
echo 'required check passed'
HOOK
  chmod +x "$repo/.git/hooks/pre-commit"
  git -C "$repo" add tracked.txt
  run git -C "$repo" commit -m 'fixture checkpoint'
  [ "$status" -eq 1 ]
  printf 'git commit exit=1\n%s\n' "$output" >"$evidence"
  [ "$(git -C "$repo" rev-parse HEAD)" = "$baseline" ]
  [ "$(git -C "$repo" diff --cached --name-only -- tracked.txt)" = tracked.txt ]
}

hook_failure() {
  make_proposal '.state = "failing" | .test = {command:"git commit -m fixture-checkpoint",receipt_ref:$evidence,outcome:"fail",failure_kind:"pre-commit-hook"} | .gates |= map_values(null) | .delivery = null | .next_action = "causal-edit: repair the hook-required content"'
}

@test "failed real pre-commit hook can recover through fresh tests review commit and verification" {
  await_commit
  fail_commit
  staged_before=$(git -C "$repo" diff --cached --binary | sha256sum)
  hook_failure
  accept
  [ "$(jq -r '.state' <<<"$record")" = failing ]
  [ "$(git -C "$repo" diff --cached --binary | sha256sum)" = "$staged_before" ]
  [ "$(jq -r '.test.receipt_ref' <<<"$record")" = "$evidence" ]

  make_proposal '.state = "passing-awaiting-gates-or-review" | .test = {command:"check",receipt_ref:"pass",outcome:"pass",failure_kind:null} | .gates.lightweight_review_receipt = "old review" | .next_action = "commit-through-pre-commit-hook"'
  reject
  make_proposal '.next_action = "push"'
  reject

  printf 'repaired\n' >"$repo/tracked.txt"
  run grep -qx repaired "$repo/tracked.txt"
  [ "$status" -eq 0 ]
  make_proposal '.state = "passing-awaiting-gates-or-review" | .test = {command:"grep -qx repaired tracked.txt",receipt_ref:"fresh repaired check passed",outcome:"pass",failure_kind:null} | .next_action = "lightweight-review"'
  accept
  make_proposal '.gates.lightweight_review_receipt = "fresh repaired review passed" | .next_action = "commit-through-pre-commit-hook"'
  accept
  git -C "$repo" add tracked.txt
  run git -C "$repo" commit -m 'fixture recovery'
  [ "$status" -eq 0 ]
  [[ "$output" == *"required check passed"* ]]
  new_head=$(git -C "$repo" rev-parse HEAD)
  [ "$new_head" != "$baseline" ]
  make_proposal '.state = "committed" | .gates.fast_gate_receipt = "real successful pre-commit receipt" | .delivery = {mode:"local-only",commit_oid:$head,pushed_oid:null,local_snapshot:null} | .next_action = "verify-exact-commit"'
  accept
  make_proposal '.next_action = "record-local-delivery"'
  reject
  make_proposal '.gates.exact_identity_verification_receipt = {receipt_ref:"verified fixture commit and source identity",outcome:"pass"} | .next_action = "record-local-delivery"'
  accept
  make_proposal '.state = "pushed-or-delivery-mode-equivalent" | .delivery.local_snapshot = "verified local commit" | .next_action = "terminal-review"'
  accept
  [ "$(jq -r '.baseline_oid' <<<"$record")" = "$baseline" ]
}

@test "lightweight review findings permit only failure recording and fresh remediation" {
  await_review
  printf 'review failed: missing boundary case\n' >"$evidence"
  make_proposal '.state = "failing" | .test = {command:"lightweight-review",receipt_ref:$evidence,outcome:"fail",failure_kind:"lightweight-review"} | .next_action = "causal-edit: cover the missing boundary"'
  accept
  make_proposal '.state = "passing-awaiting-gates-or-review" | .test.outcome = "pass" | .test.failure_kind = null | .gates.lightweight_review_receipt = "stale review" | .next_action = "commit-through-pre-commit-hook"'
  reject
  printf 'boundary covered\n' >"$repo/tracked.txt"
  run grep -qx 'boundary covered' "$repo/tracked.txt"
  [ "$status" -eq 0 ]
  make_proposal '.state = "passing-awaiting-gates-or-review" | .test = {command:"boundary check",receipt_ref:"fresh boundary result",outcome:"pass",failure_kind:null} | .next_action = "lightweight-review"'
  accept
  make_proposal '.gates.lightweight_review_receipt = "fresh boundary review" | .next_action = "commit-through-pre-commit-hook"'
  accept
}

@test "failure successors reject invalid evidence identities and skipped gates without publication" {
  await_commit
  fail_commit
  for mutation in \
    '.test.failure_kind = "lightweight-review"' \
    '.test.receipt_ref = ""' \
    '.test.outcome = "pass"' \
    '.gates.lightweight_review_receipt = "stale pass"' \
    '.gates.fast_gate_receipt = "fabricated pass"' \
    '.next_action = "commit-through-pre-commit-hook"' \
    '.baseline_oid = ("a" * 40)' \
    '.snapshot.tracked_sha256 = ("a" * 64)' \
    '.generation += 1' \
    '.predecessor_sha256 = ("a" * 64)'; do
    hook_failure
    tail -c +15 "$proposal" | jq -c "$mutation" >"$records/mutated"
    printf 'checkpoint-v1 %s\n' "$(cat "$records/mutated")" >"$proposal"
    reject
  done
  git -C "$repo" -c core.hooksPath=/dev/null commit -qm 'separate fixture commit'
  hook_failure
  reject
}

@test "recovery helper preserves mixed staged unstaged and newly staged work" {
  await_commit
  printf 'new content\n' >"$repo/new file.txt"
  git -C "$repo" add 'new file.txt'
  fail_commit
  printf 'unstaged extra content\n' >>"$repo/tracked.txt"
  index_before=$(git -C "$repo" write-tree)
  worktree_before=$(sha256sum "$repo/tracked.txt" "$repo/new file.txt")
  run bash -c 'cd "$1" && exec "$2" repair "$3" "$4" pre-commit-hook "git commit -m fixture-checkpoint" "$5" "repair the failed required check"' _ "$repo" "$recovery" "$generation" "$predecessor" "$evidence"
  if [ "$status" -ne 0 ]; then echo "$output"; return 1; fi
  read_checkpoint
  [ "$(jq -r '.state' <<<"$record")" = failing ]
  [ "$(git -C "$repo" write-tree)" = "$index_before" ]
  [ "$(sha256sum "$repo/tracked.txt" "$repo/new file.txt")" = "$worktree_before" ]
  [[ "$(jq -r '.test.receipt_ref' <<<"$record")" == *"sha256="* ]]
  [ "$(jq -r '.baseline_oid' <<<"$record")" = "$baseline" ]
}

@test "recovery helper rejects missing evidence stale callers and the wrong pending gate" {
  await_commit
  fail_commit
  before=$(sha256sum "$target")
  for kind in lightweight-review unknown; do
    run bash -c 'cd "$1" && exec "$2" repair "$3" "$4" "$5" "git commit" "$6" "repair the failure"' _ "$repo" "$recovery" "$generation" "$predecessor" "$kind" "$evidence"
    [ "$status" -ne 0 ]
    [ "$(sha256sum "$target")" = "$before" ]
  done
  for receipt in "$records/missing" "$records/empty" "$repo/in-worktree-receipt"; do
    : >"$records/empty"
    printf 'actual hook failed\n' >"$repo/in-worktree-receipt"
    run bash -c 'cd "$1" && exec "$2" repair "$3" "$4" pre-commit-hook "git commit" "$5" "repair the failure"' _ "$repo" "$recovery" "$generation" "$predecessor" "$receipt"
    [ "$status" -ne 0 ]
    [ "$(sha256sum "$target")" = "$before" ]
  done
  rm "$repo/in-worktree-receipt"
  run bash -c 'cd "$1" && exec "$2" repair "$3" "$4" pre-commit-hook "git commit" "$5" "repair the failure"' _ "$repo" "$recovery" "$((generation + 1))" "$predecessor" "$evidence"
  [ "$status" -ne 0 ]
  [ "$(sha256sum "$target")" = "$before" ]
  run bash -c 'cd "$1" && exec "$2" repair "$3" "$4" pre-commit-hook "git commit" "$5" "repair the failure"' _ "$repo" "$recovery" "$generation" "$(printf '%064d' 0)" "$evidence"
  [ "$status" -ne 0 ]
  [ "$(sha256sum "$target")" = "$before" ]
  git -C "$repo" -c core.hooksPath=/dev/null commit -qm 'separate fixture commit'
  run bash -c 'cd "$1" && exec "$2" repair "$3" "$4" pre-commit-hook "git commit" "$5" "repair the failure"' _ "$repo" "$recovery" "$generation" "$predecessor" "$evidence"
  [ "$status" -ne 0 ]
  [ "$(sha256sum "$target")" = "$before" ]
}

@test "recovery helper records a review failure without running a commit or changing source" {
  await_review
  printf 'review failed: missing boundary coverage\n' >"$evidence"
  source_before=$(sha256sum "$repo/tracked.txt")
  run bash -c 'cd "$1" && exec "$2" repair "$3" "$4" lightweight-review "bounded lightweight review" "$5" "cover the missing boundary"' _ "$repo" "$recovery" "$generation" "$predecessor" "$evidence"
  [ "$status" -eq 0 ]
  read_checkpoint
  [ "$(jq -r '.test.failure_kind' <<<"$record")" = lightweight-review ]
  [ "$(jq -r '.next_action' <<<"$record")" = 'causal-edit: cover the missing boundary' ]
  [ "$(sha256sum "$repo/tracked.txt")" = "$source_before" ]
  [ "$(git -C "$repo" rev-parse HEAD)" = "$baseline" ]
}

@test "recovery helper does not normalize malformed predecessor evidence into a valid failure" {
  await_commit
  fail_commit
  for mutation in '.test = null' '.snapshot.tracked_sha256 = "invalid"' '.gates.fast_gate_receipt = "unexpected pass"'; do
    # Deliberately corrupt the disposable authority to simulate damaged evidence.
    malformed=$(jq -c "$mutation" <<<"$record")
    printf 'checkpoint-v1 %s\n' "$malformed" >"$target"
    damaged_digest=$(sha256sum "$target" | cut -d ' ' -f 1)
    before=$(sha256sum "$target")
    run bash -c 'cd "$1" && exec "$2" repair "$3" "$4" pre-commit-hook "git commit" "$5" "repair the failure"' _ "$repo" "$recovery" "$generation" "$damaged_digest" "$evidence"
    [ "$status" -ne 0 ]
    [ "$(sha256sum "$target")" = "$before" ]
  done
}
