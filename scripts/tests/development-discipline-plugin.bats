#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
}
@test "delivery-workflow benchmark rejects policy-invalid plans" {
  workspace="$ROOT/plugins/development-system/components/development-discipline/skills/delivery-workflow/.plugin-eval/workspace"

  run node "$workspace/verify-delivery-plan.mjs" "$workspace/fixtures/direct-to-trunk-valid.json"
  [ "$status" -eq 0 ]

  run node "$workspace/verify-delivery-plan.mjs" "$workspace/fixtures/local-only-invalid.json"
  [ "$status" -ne 0 ]

  run node "$workspace/verify-delivery-plan.mjs" "$workspace/fixtures/local-only-valid.json"
  [ "$status" -eq 0 ]

  run node "$workspace/verify-delivery-plan.mjs" "$workspace/fixtures/local-only-authorization-invalid.json"
  [ "$status" -ne 0 ]

  run node "$workspace/verify-delivery-plan.mjs" "$workspace/fixtures/direct-to-trunk-invalid.json"
  [ "$status" -ne 0 ]

  run node "$workspace/verify-delivery-plan.mjs" "$workspace/fixtures/content-identical-commit-valid.json"
  [ "$status" -eq 0 ]

  run node "$workspace/verify-delivery-plan.mjs" "$workspace/fixtures/source-change-rereview-valid.json"
  [ "$status" -eq 0 ]

  run node "$workspace/verify-delivery-plan.mjs" "$workspace/fixtures/content-identical-commit-rereview-invalid.json"
  [ "$status" -ne 0 ]

  run node "$workspace/verify-delivery-plan.mjs" "$workspace/fixtures/content-change-no-rereview-invalid.json"
  [ "$status" -ne 0 ]

  run node "$workspace/verify-delivery-plan.mjs" "$workspace/fixtures/content-identical-commit-gate-invalid.json"
  [ "$status" -ne 0 ]
}

@test "development-workflow benchmark verifies lifecycle routing and stop boundaries" {
  workspace="$ROOT/plugins/development-system/components/development-discipline/skills/development-workflow/.plugin-eval/workspace"

  run node "$workspace/verify-workflow-plan.mjs" "$workspace/fixtures/implementation-valid.json"
  [ "$status" -eq 0 ]

  run node "$workspace/verify-workflow-plan.mjs" "$workspace/fixtures/implementation-missing-verification.json"
  [ "$status" -ne 0 ]

  run node "$workspace/verify-workflow-plan.mjs" "$workspace/fixtures/implementation-delivery-late.json"
  [ "$status" -ne 0 ]

  run node "$workspace/verify-workflow-plan.mjs" "$workspace/fixtures/implementation-extra-specialist.json"
  [ "$status" -ne 0 ]

  run node "$workspace/verify-workflow-plan.mjs" "$workspace/fixtures/implementation-extra-phase.json"
  [ "$status" -ne 0 ]

  run node "$workspace/verify-workflow-plan.mjs" "$workspace/fixtures/ci-hold-valid.json"
  [ "$status" -eq 0 ]

  run node "$workspace/verify-workflow-plan.mjs" "$workspace/fixtures/ci-hold-with-unrelated-phases.json"
  [ "$status" -ne 0 ]

  run node "$workspace/verify-workflow-plan.mjs" "$workspace/fixtures/review-only-invalid.json"
  [ "$status" -ne 0 ]

  run node "$workspace/verify-workflow-plan.mjs" "$workspace/fixtures/review-only-valid.json"
  [ "$status" -eq 0 ]

  run node "$workspace/verify-workflow-plan.mjs" "$workspace/fixtures/review-only-hidden-specialists.json"
  [ "$status" -ne 0 ]

  run node "$workspace/verify-workflow-plan.mjs" "$workspace/fixtures/review-only-extra-phase.json"
  [ "$status" -ne 0 ]
}

@test "local durable checkpoints reject stale cooperative writers" {
  skill="$ROOT/plugins/development-system/skills/development-workflow/SKILL.md"
  writer="$ROOT/plugins/development-system/scripts/write-local-checkpoint.sh"

  repo=$(mktemp -d)
  git -C "$repo" init -q
  printf '%s\n' baseline >"$repo/tracked.txt"
  git -C "$repo" add tracked.txt
  git -C "$repo" -c user.name=Test -c user.email=test@example.invalid -c commit.gpgsign=false commit --allow-empty -m baseline -q
  records=$(mktemp -d)
  first="$records/first.record"
  second="$records/second.record"
  stale="$records/stale.record"
  invalid="$records/invalid.record"
  invalid_state="$records/invalid-state.record"
  invalid_framing="$records/invalid-framing.record"
  invalid_nul="$records/invalid-nul.record"
  invalid_local_ci="$records/invalid-local-ci.record"
  invalid_baseline="$records/invalid-baseline.record"
  invalid_committed_test="$records/invalid-committed-test.record"
  dirty_bootstrap="$records/dirty-bootstrap.record"
  empty_action="$records/empty-action.record"
  invalid_test="$records/invalid-test.record"
  head_oid=$(git -C "$repo" rev-parse HEAD)
  empty_sha=e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
  printf '%s\n' 'checkpoint-v1 {"generation":0,"predecessor_sha256":null}' >"$invalid"

  run bash -c 'cd "$1" && "$2" work-item 0 null "$3"' _ "$repo" "$writer" "$invalid"
  [ "$status" -ne 0 ]
  [ ! -e "$repo/.git/development-system/checkpoints/work-item.latest" ]

  invalid_state_json=$(jq -cn --arg head "$head_oid" --arg empty "$empty_sha" '{generation:0,predecessor_sha256:null,baseline_oid:$head,snapshot:{head_oid:$head,tracked_sha256:$empty,untracked_sha256:$empty},state:"pushed-or-delivery-mode-equivalent",test:null,gates:{lightweight_review_receipt:null,fast_gate_receipt:null,exact_identity_verification_receipt:null},delivery:null,ci:{runs:[],terminal_success_run_id:null},next_action:"edit"}')
  printf 'checkpoint-v1 %s\n' "$invalid_state_json" >"$invalid_state"
  run bash -c 'cd "$1" && "$2" work-item 0 null "$3"' _ "$repo" "$writer" "$invalid_state"
  [ "$status" -ne 0 ]
  [[ "$output" == *"checkpoint record failed schema, snapshot, or state validation"* ]]

  printf 'checkpoint-v1 %s\n%s' "$invalid_state_json" "$invalid_state_json" >"$invalid_framing"
  run bash -c 'cd "$1" && "$2" work-item 0 null "$3"' _ "$repo" "$writer" "$invalid_framing"
  [ "$status" -ne 0 ]

  printf 'checkpoint-v1 {"generation":0,"predecessor_sha256":null}\0\n' >"$invalid_nul"
  run bash -c 'cd "$1" && "$2" work-item 0 null "$3"' _ "$repo" "$writer" "$invalid_nul"
  [ "$status" -ne 0 ]

  invalid_local_ci_json=$(jq -cn --arg head "$head_oid" --arg empty "$empty_sha" '{generation:0,predecessor_sha256:null,baseline_oid:$head,snapshot:{head_oid:$head,tracked_sha256:$empty,untracked_sha256:$empty},state:"pushed-or-delivery-mode-equivalent",test:null,gates:{lightweight_review_receipt:null,fast_gate_receipt:null,exact_identity_verification_receipt:null},delivery:{mode:"local-only",commit_oid:null,pushed_oid:null,local_snapshot:"snapshot"},ci:{runs:[{provider:"ci",run_id:"run",commit_oid:$head,status:"success"}],terminal_success_run_id:"run"},next_action:"edit"}')
  printf 'checkpoint-v1 %s\n' "$invalid_local_ci_json" >"$invalid_local_ci"
  run bash -c 'cd "$1" && "$2" work-item 0 null "$3"' _ "$repo" "$writer" "$invalid_local_ci"
  [ "$status" -ne 0 ]

  generation_zero_failing_json=$(jq -cn --arg head "$head_oid" --arg empty "$empty_sha" '{generation:0,predecessor_sha256:null,baseline_oid:$head,snapshot:{head_oid:$head,tracked_sha256:$empty,untracked_sha256:$empty},state:"failing",test:{command:"test",receipt_ref:"receipt",outcome:"fail",failure_kind:"expected-red"},gates:{lightweight_review_receipt:null,fast_gate_receipt:null,exact_identity_verification_receipt:null},delivery:null,ci:{runs:[],terminal_success_run_id:null},next_action:"causal-edit"}')
  printf 'checkpoint-v1 %s\n' "$generation_zero_failing_json" >"$invalid_local_ci"
  run bash -c 'cd "$1" && "$2" generation-zero-failing 0 null "$3"' _ "$repo" "$writer" "$invalid_local_ci"
  [ "$status" -ne 0 ]

  contradictory_local_json=$(jq -cn --arg head "$head_oid" --arg empty "$empty_sha" '{generation:0,predecessor_sha256:null,baseline_oid:$head,snapshot:{head_oid:$head,tracked_sha256:$empty,untracked_sha256:$empty},state:"pushed-or-delivery-mode-equivalent",test:null,gates:{lightweight_review_receipt:null,fast_gate_receipt:null,exact_identity_verification_receipt:null},delivery:{mode:"local-only",commit_oid:null,pushed_oid:$head,local_snapshot:"snapshot"},ci:{runs:[],terminal_success_run_id:null},next_action:"edit"}')
  printf 'checkpoint-v1 %s\n' "$contradictory_local_json" >"$invalid_local_ci"
  run bash -c 'cd "$1" && "$2" contradictory-local 0 null "$3"' _ "$repo" "$writer" "$invalid_local_ci"
  [ "$status" -ne 0 ]
  [[ "$output" == *"checkpoint record failed schema, snapshot, or state validation"* ]]

  first_json=$(jq -cn --arg head "$head_oid" --arg empty "$empty_sha" '{generation:0,predecessor_sha256:null,baseline_oid:$head,snapshot:{head_oid:$head,tracked_sha256:$empty,untracked_sha256:$empty},state:"pushed-or-delivery-mode-equivalent",test:null,gates:{lightweight_review_receipt:null,fast_gate_receipt:null,exact_identity_verification_receipt:null},delivery:{mode:"local-only",commit_oid:null,pushed_oid:null,local_snapshot:"snapshot"},ci:{runs:[],terminal_success_run_id:null},next_action:"causal-edit: implement the first planned ticket increment"}')
  printf 'checkpoint-v1 %s\n' "$first_json" >"$first"

  in_worktree_record="$repo/proposal.record"
  printf 'checkpoint-v1 %s\n' "$first_json" >"$in_worktree_record"
  run bash -c 'cd "$1" && "$2" self-referential 0 null "$3"' _ "$repo" "$writer" "$in_worktree_record"
  [ "$status" -ne 0 ]
  [[ "$output" == *"record file must be outside the worktree"* ]]
  rm "$in_worktree_record"

  empty_action_json=$(printf '%s' "$first_json" | jq -c '.next_action = ""')
  printf 'checkpoint-v1 %s\n' "$empty_action_json" >"$empty_action"
  run bash -c 'cd "$1" && "$2" empty-action 0 null "$3"' _ "$repo" "$writer" "$empty_action"
  [ "$status" -ne 0 ]

  mkdir "$repo/nested"
  printf '%s\n' outside >"$repo/root-untracked"
  run bash -c 'cd "$1/nested" && "$2" nested-cwd 0 null "$3"' _ "$repo" "$writer" "$first"
  [ "$status" -ne 0 ]
  [ ! -e "$repo/.git/development-system/checkpoints/nested-cwd.latest" ]
  rm "$repo/root-untracked"

  run bash -c 'cd "$1" && "$2" work-item 0 null "$3"' _ "$repo" "$writer" "$first"
  [ "$status" -eq 0 ]
  target="$repo/.git/development-system/checkpoints/work-item.latest"
  predecessor=$(sha256sum "$target" | cut -d ' ' -f 1)

  run bash -c 'cd "$1" && "$2" legacy-upgrade 0 null "$3"' _ "$repo" "$writer" "$first"
  [ "$status" -eq 0 ]
  legacy_target="$repo/.git/development-system/checkpoints/legacy-upgrade.latest"
  legacy_zero_predecessor=$(sha256sum "$legacy_target" | cut -d ' ' -f 1)
  legacy_json=$(jq -cn --arg predecessor "$legacy_zero_predecessor" --arg head "$head_oid" --arg empty "$empty_sha" '{generation:1,predecessor_sha256:$predecessor,baseline_oid:$head,snapshot:{head_oid:$head,tracked_sha256:$empty,untracked_sha256:$empty},state:"failing",test:{command:"test",receipt_ref:"receipt",outcome:"fail",failure_kind:"expected-red"},gates:{lightweight_review_receipt:null,fast_gate_receipt:null,exact_identity_verification_receipt:null},delivery:null,ci:{runs:[],terminal_success_run_id:null},next_action:"legacy descriptive action"}')
  printf 'checkpoint-v1 %s\n' "$legacy_json" >"$legacy_target"
  legacy_predecessor=$(sha256sum "$legacy_target" | cut -d ' ' -f 1)
  legacy_successor_json=$(printf '%s' "$legacy_json" | jq -c --arg predecessor "$legacy_predecessor" '.generation = 2 | .predecessor_sha256 = $predecessor | .next_action = "causal-edit: implement the recorded RED behavior"')
  printf 'checkpoint-v1 %s\n' "$legacy_successor_json" >"$stale"
  run bash -c 'cd "$1" && "$2" legacy-upgrade 2 "$3" "$4"' _ "$repo" "$writer" "$legacy_predecessor" "$stale"
  [ "$status" -eq 0 ]

  for passing_case in lightweight fast commit; do
    case_id="passing-$passing_case"
    run bash -c 'cd "$1" && "$2" "$3" 0 null "$4"' _ "$repo" "$writer" "$case_id" "$first"
    [ "$status" -eq 0 ]
    case_target="$repo/.git/development-system/checkpoints/$case_id.latest"
    case_predecessor=$(sha256sum "$case_target" | cut -d ' ' -f 1)
    case "$passing_case" in
      lightweight) light=null; fast=null; action=lightweight-review ;;
      fast) light='"review"'; fast=null; action=fast-gate ;;
      commit) light='"review"'; fast='"gate"'; action=commit-or-record-local-snapshot ;;
    esac
    passing_json=$(jq -cn --arg predecessor "$case_predecessor" --arg head "$head_oid" --arg empty "$empty_sha" --arg action "$action" --argjson light "$light" --argjson fast "$fast" '{generation:1,predecessor_sha256:$predecessor,baseline_oid:$head,snapshot:{head_oid:$head,tracked_sha256:$empty,untracked_sha256:$empty},state:"passing-awaiting-gates-or-review",test:{command:"test",receipt_ref:"receipt",outcome:"pass",failure_kind:null},gates:{lightweight_review_receipt:$light,fast_gate_receipt:$fast,exact_identity_verification_receipt:null},delivery:null,ci:{runs:[],terminal_success_run_id:null},next_action:$action}')
    printf 'checkpoint-v1 %s\n' "$passing_json" >"$stale"
    run bash -c 'cd "$1" && "$2" "$3" 1 "$4" "$5"' _ "$repo" "$writer" "$case_id" "$case_predecessor" "$stale"
    [ "$status" -eq 0 ]
  done

  invalid_passing_json=$(jq -cn --arg predecessor "$predecessor" --arg head "$head_oid" --arg empty "$empty_sha" '{generation:1,predecessor_sha256:$predecessor,baseline_oid:$head,snapshot:{head_oid:$head,tracked_sha256:$empty,untracked_sha256:$empty},state:"passing-awaiting-gates-or-review",test:{command:"test",receipt_ref:"receipt",outcome:"pass",failure_kind:null},gates:{lightweight_review_receipt:null,fast_gate_receipt:"gate",exact_identity_verification_receipt:null},delivery:null,ci:{runs:[],terminal_success_run_id:null},next_action:"lightweight-review"}')
  printf 'checkpoint-v1 %s\n' "$invalid_passing_json" >"$stale"
  run bash -c 'cd "$1" && "$2" work-item 1 "$3" "$4"' _ "$repo" "$writer" "$predecessor" "$stale"
  [ "$status" -ne 0 ]

  invalid_test_json=$(jq -cn --arg predecessor "$predecessor" --arg head "$head_oid" --arg empty "$empty_sha" '{generation:1,predecessor_sha256:$predecessor,baseline_oid:$head,snapshot:{head_oid:$head,tracked_sha256:$empty,untracked_sha256:$empty},state:"failing",test:{command:"new-test",receipt_ref:"unexpected-pass",outcome:"pass",failure_kind:"invalid-test"},gates:{lightweight_review_receipt:null,fast_gate_receipt:null,exact_identity_verification_receipt:null},delivery:null,ci:{runs:[],terminal_success_run_id:null},next_action:"rewrite-invalid-test: make the new test prove the missing behavior"}')
  printf 'checkpoint-v1 %s\n' "$invalid_test_json" >"$invalid_test"
  run bash -c 'cd "$1" && "$2" invalid-test 0 null "$3"' _ "$repo" "$writer" "$first"
  [ "$status" -eq 0 ]
  run bash -c 'cd "$1" && "$2" invalid-test 1 "$3" "$4"' _ "$repo" "$writer" "$predecessor" "$invalid_test"
  [ "$status" -eq 0 ]

  invalid_committed_test_json=$(jq -cn --arg predecessor "$predecessor" --arg head "$head_oid" --arg empty "$empty_sha" '{generation:1,predecessor_sha256:$predecessor,baseline_oid:$head,snapshot:{head_oid:$head,tracked_sha256:$empty,untracked_sha256:$empty},state:"committed",test:{command:"test",receipt_ref:"receipt",outcome:"fail",failure_kind:"regression"},gates:{lightweight_review_receipt:"review",fast_gate_receipt:"gate",exact_identity_verification_receipt:{receipt_ref:"verified",outcome:"fail"}},delivery:{mode:"local-only",commit_oid:$head,pushed_oid:null,local_snapshot:null},ci:{runs:[],terminal_success_run_id:null},next_action:"local-complete"}')
  printf 'checkpoint-v1 %s\n' "$invalid_committed_test_json" >"$invalid_committed_test"
  run bash -c 'cd "$1" && "$2" work-item 1 "$3" "$4"' _ "$repo" "$writer" "$predecessor" "$invalid_committed_test"
  [ "$status" -ne 0 ]

  failed_verification_push_json=$(jq -cn --arg predecessor "$predecessor" --arg head "$head_oid" --arg empty "$empty_sha" '{generation:1,predecessor_sha256:$predecessor,baseline_oid:$head,snapshot:{head_oid:$head,tracked_sha256:$empty,untracked_sha256:$empty},state:"committed",test:{command:"test",receipt_ref:"receipt",outcome:"pass",failure_kind:null},gates:{lightweight_review_receipt:"review",fast_gate_receipt:"gate",exact_identity_verification_receipt:{receipt_ref:"verified",outcome:"fail"}},delivery:{mode:"direct-to-trunk",commit_oid:$head,pushed_oid:null,local_snapshot:null},ci:{runs:[],terminal_success_run_id:null},next_action:"push"}')
  printf 'checkpoint-v1 %s\n' "$failed_verification_push_json" >"$invalid_committed_test"
  run bash -c 'cd "$1" && "$2" failed-push 1 "$3" "$4"' _ "$repo" "$writer" "$predecessor" "$invalid_committed_test"
  [ "$status" -ne 0 ]

  verified_wrong_action_json=$(printf '%s' "$failed_verification_push_json" | jq -c '.gates.exact_identity_verification_receipt.outcome = "pass" | .next_action = "causal-edit"')
  printf 'checkpoint-v1 %s\n' "$verified_wrong_action_json" >"$invalid_committed_test"
  run bash -c 'cd "$1" && "$2" work-item 1 "$3" "$4"' _ "$repo" "$writer" "$predecessor" "$invalid_committed_test"
  [ "$status" -ne 0 ]

  empty_receipts_json=$(printf '%s' "$failed_verification_push_json" | jq -c '.gates.lightweight_review_receipt = "" | .gates.fast_gate_receipt = "" | .gates.exact_identity_verification_receipt = {receipt_ref:"",outcome:"pass"} | .next_action = "push"')
  printf 'checkpoint-v1 %s\n' "$empty_receipts_json" >"$invalid_committed_test"
  run bash -c 'cd "$1" && "$2" work-item 1 "$3" "$4"' _ "$repo" "$writer" "$predecessor" "$invalid_committed_test"
  [ "$status" -ne 0 ]

  failed_verification_delivery_json=$(jq -cn --arg predecessor "$predecessor" --arg head "$head_oid" --arg empty "$empty_sha" '{generation:1,predecessor_sha256:$predecessor,baseline_oid:$head,snapshot:{head_oid:$head,tracked_sha256:$empty,untracked_sha256:$empty},state:"pushed-or-delivery-mode-equivalent",test:{command:"test",receipt_ref:"receipt",outcome:"pass",failure_kind:null},gates:{lightweight_review_receipt:"review",fast_gate_receipt:"gate",exact_identity_verification_receipt:{receipt_ref:"verification",outcome:"fail"}},delivery:{mode:"local-only",commit_oid:null,pushed_oid:null,local_snapshot:"snapshot"},ci:{runs:[],terminal_success_run_id:null},next_action:"local-complete"}')
  printf 'checkpoint-v1 %s\n' "$failed_verification_delivery_json" >"$invalid_committed_test"
  run bash -c 'cd "$1" && "$2" failed-verification 1 "$3" "$4"' _ "$repo" "$writer" "$predecessor" "$invalid_committed_test"
  [ "$status" -ne 0 ]

  remote_bootstrap_json=$(printf '%s' "$first_json" | jq -c --arg head "$head_oid" '.delivery = {mode:"direct-to-trunk",commit_oid:null,pushed_oid:$head,local_snapshot:null}')
  printf 'checkpoint-v1 %s\n' "$remote_bootstrap_json" >"$stale"
  run bash -c 'cd "$1" && "$2" remote-ci 0 null "$3"' _ "$repo" "$writer" "$stale"
  [ "$status" -eq 0 ]
  remote_target="$repo/.git/development-system/checkpoints/remote-ci.latest"
  remote_predecessor=$(sha256sum "$remote_target" | cut -d ' ' -f 1)
  remote_failure_json=$(jq -cn --arg predecessor "$remote_predecessor" --arg head "$head_oid" --arg empty "$empty_sha" '{generation:1,predecessor_sha256:$predecessor,baseline_oid:$head,snapshot:{head_oid:$head,tracked_sha256:$empty,untracked_sha256:$empty},state:"pushed-or-delivery-mode-equivalent",test:{command:"test",receipt_ref:"receipt",outcome:"pass",failure_kind:null},gates:{lightweight_review_receipt:"review",fast_gate_receipt:"gate",exact_identity_verification_receipt:{receipt_ref:"verification",outcome:"pass"}},delivery:{mode:"direct-to-trunk",commit_oid:$head,pushed_oid:$head,local_snapshot:null},ci:{runs:[{provider:"ci",run_id:"failed",commit_oid:$head,status:"failure"}],terminal_success_run_id:null},next_action:"edit"}')
  printf 'checkpoint-v1 %s\n' "$remote_failure_json" >"$stale"
  run bash -c 'cd "$1" && "$2" remote-ci 1 "$3" "$4"' _ "$repo" "$writer" "$remote_predecessor" "$stale"
  [ "$status" -ne 0 ]
  remote_recovery_json=$(printf '%s' "$remote_failure_json" | jq -c '.next_action = "enter-ci-recovery"')
  printf 'checkpoint-v1 %s\n' "$remote_recovery_json" >"$stale"
  run bash -c 'cd "$1" && "$2" remote-ci 1 "$3" "$4"' _ "$repo" "$writer" "$remote_predecessor" "$stale"
  [ "$status" -eq 0 ]
  recovered_predecessor=$(sha256sum "$remote_target" | cut -d ' ' -f 1)
  recovered_json=$(printf '%s' "$remote_recovery_json" | jq -c --arg predecessor "$recovered_predecessor" '.generation = 2 | .predecessor_sha256 = $predecessor | .ci.runs += [{provider:"ci",run_id:"recovered",commit_oid:.snapshot.head_oid,status:"success"}] | .ci.terminal_success_run_id = "recovered" | .next_action = "terminal-review"')
  printf 'checkpoint-v1 %s\n' "$recovered_json" >"$stale"
  run bash -c 'cd "$1" && "$2" remote-ci 2 "$3" "$4"' _ "$repo" "$writer" "$recovered_predecessor" "$stale"
  [ "$status" -eq 0 ]
  recovered_terminal_predecessor=$(sha256sum "$remote_target" | cut -d ' ' -f 1)
  dropped_failure_json=$(printf '%s' "$recovered_json" | jq -c --arg predecessor "$recovered_terminal_predecessor" '.generation = 3 | .predecessor_sha256 = $predecessor | .ci.runs = [.ci.runs[-1]]')
  printf 'checkpoint-v1 %s\n' "$dropped_failure_json" >"$stale"
  run bash -c 'cd "$1" && "$2" remote-ci 3 "$3" "$4"' _ "$repo" "$writer" "$recovered_terminal_predecessor" "$stale"
  [ "$status" -ne 0 ]
  stale_success_json=$(printf '%s' "$recovered_json" | jq -c --arg predecessor "$recovered_terminal_predecessor" '.generation = 3 | .predecessor_sha256 = $predecessor | .ci.runs += [{provider:"ci",run_id:"newer-failure",commit_oid:.snapshot.head_oid,status:"failure"}]')
  printf 'checkpoint-v1 %s\n' "$stale_success_json" >"$stale"
  run bash -c 'cd "$1" && "$2" remote-ci 3 "$3" "$4"' _ "$repo" "$writer" "$recovered_terminal_predecessor" "$stale"
  [ "$status" -ne 0 ]

  printf 'checkpoint-v1 %s\n' "$remote_bootstrap_json" >"$stale"
  run bash -c 'cd "$1" && "$2" cross-commit-ci 0 null "$3"' _ "$repo" "$writer" "$stale"
  [ "$status" -eq 0 ]
  cross_commit_target="$repo/.git/development-system/checkpoints/cross-commit-ci.latest"
  cross_commit_predecessor=$(sha256sum "$cross_commit_target" | cut -d ' ' -f 1)
  retained_older_ci_json=$(jq -cn --arg predecessor "$cross_commit_predecessor" --arg head "$head_oid" --arg empty "$empty_sha" '{generation:1,predecessor_sha256:$predecessor,baseline_oid:$head,snapshot:{head_oid:$head,tracked_sha256:$empty,untracked_sha256:$empty},state:"pushed-or-delivery-mode-equivalent",test:{command:"test",receipt_ref:"receipt",outcome:"pass",failure_kind:null},gates:{lightweight_review_receipt:"review",fast_gate_receipt:"gate",exact_identity_verification_receipt:{receipt_ref:"verification",outcome:"pass"}},delivery:{mode:"direct-to-trunk",commit_oid:$head,pushed_oid:$head,local_snapshot:null},ci:{runs:[{provider:"ci",run_id:"older-success",commit_oid:"1111111111111111111111111111111111111111",status:"success"}],terminal_success_run_id:null},next_action:"register-exact-sha-ci-monitor"}')
  printf 'checkpoint-v1 %s\n' "$retained_older_ci_json" >"$stale"
  run bash -c 'cd "$1" && "$2" cross-commit-ci 1 "$3" "$4"' _ "$repo" "$writer" "$cross_commit_predecessor" "$stale"
  [ "$status" -eq 0 ]
  cross_commit_predecessor=$(sha256sum "$cross_commit_target" | cut -d ' ' -f 1)
  current_ci_json=$(printf '%s' "$retained_older_ci_json" | jq -c --arg predecessor "$cross_commit_predecessor" '.generation = 2 | .predecessor_sha256 = $predecessor | .ci.runs += [{provider:"ci",run_id:"current",commit_oid:.snapshot.head_oid,status:"running"}] | .next_action = "monitor-exact-sha-ci"')
  printf 'checkpoint-v1 %s\n' "$current_ci_json" >"$stale"
  run bash -c 'cd "$1" && "$2" cross-commit-ci 2 "$3" "$4"' _ "$repo" "$writer" "$cross_commit_predecessor" "$stale"
  [ "$status" -eq 0 ]

  invalid_baseline_json=$(jq -cn --arg predecessor "$predecessor" --arg head "$head_oid" --arg empty "$empty_sha" '{generation:1,predecessor_sha256:$predecessor,baseline_oid:"0000000000000000000000000000000000000000",snapshot:{head_oid:$head,tracked_sha256:$empty,untracked_sha256:$empty},state:"failing",test:{command:"test",receipt_ref:"receipt",outcome:"fail",failure_kind:"expected-red"},gates:{lightweight_review_receipt:null,fast_gate_receipt:null,exact_identity_verification_receipt:null},delivery:null,ci:{runs:[],terminal_success_run_id:null},next_action:"causal-edit: correct the mismatched baseline"}')
  printf 'checkpoint-v1 %s\n' "$invalid_baseline_json" >"$invalid_baseline"
  run bash -c 'cd "$1" && "$2" work-item 1 "$3" "$4"' _ "$repo" "$writer" "$predecessor" "$invalid_baseline"
  [ "$status" -eq 3 ]
  [[ "$output" == *"checkpoint baseline does not match predecessor"* ]]

  second_json=$(jq -cn --arg predecessor "$predecessor" --arg head "$head_oid" --arg empty "$empty_sha" '{generation:1,predecessor_sha256:$predecessor,baseline_oid:$head,snapshot:{head_oid:$head,tracked_sha256:$empty,untracked_sha256:$empty},state:"failing",test:{command:"test",receipt_ref:"receipt",outcome:"fail",failure_kind:"expected-red"},gates:{lightweight_review_receipt:null,fast_gate_receipt:null,exact_identity_verification_receipt:null},delivery:null,ci:{runs:[],terminal_success_run_id:null},next_action:"causal-edit: implement the recorded RED behavior"}')
  printf 'checkpoint-v1 %s\n' "$second_json" >"$second"

  failing_push_json=$(printf '%s' "$second_json" | jq -c '.next_action = "push"')
  printf 'checkpoint-v1 %s\n' "$failing_push_json" >"$stale"
  run bash -c 'cd "$1" && "$2" work-item 1 "$3" "$4"' _ "$repo" "$writer" "$predecessor" "$stale"
  [ "$status" -ne 0 ]

  run bash -c 'cd "$1" && "$2" work-item 1 "$3" "$4"' _ "$repo" "$writer" "$predecessor" "$second"
  [ "$status" -eq 0 ]
  current=$(<"$target")
  printf 'checkpoint-v1 %s\n' "$second_json" >"$stale"

  run bash -c 'cd "$1" && "$2" work-item 1 "$3" "$4"' _ "$repo" "$writer" "$predecessor" "$stale"
  [ "$status" -eq 3 ]
  [ "$(<"$target")" = "$current" ]

  lock_path="$repo/.git/development-system/checkpoints/lock-order.lock"
  exec {held_lock}>"$lock_path"
  flock -x "$held_lock"
  bash -c 'cd "$1" && "$2" lock-order 0 null "$3"' _ "$repo" "$writer" "$first" &
  blocked_writer=$!
  printf '%s\n' dirty >"$repo/tracked.txt"
  flock -u "$held_lock"
  wait "$blocked_writer" && false
  [ ! -e "$repo/.git/development-system/checkpoints/lock-order.latest" ]
  git -C "$repo" checkout -- tracked.txt

  git_shim_dir=$(mktemp -d)
  real_git=$(command -v git)
  printf '%s\n' '#!/usr/bin/env bash' \
    'if [[ " $* " == *" diff --binary "* ]]; then' \
    '  "$REAL_GIT" "$@"' \
    '  result=$?' \
    '  if [[ ! -e $MUTATION_MARKER ]]; then' \
    '    : >"$MUTATION_MARKER"' \
    '    printf "%s\n" dirty >>"$MUTATION_WORKTREE/tracked.txt"' \
    '  fi' \
    '  exit "$result"' \
    'fi' \
    'exec "$REAL_GIT" "$@"' >"$git_shim_dir/git"
  chmod +x "$git_shim_dir/git"
  run env PATH="$git_shim_dir:$PATH" REAL_GIT="$real_git" \
    MUTATION_MARKER="$repo/.git/mutated-after-snapshot" MUTATION_WORKTREE="$repo" \
    bash -c 'cd "$1" && "$2" post-snapshot-mutation 0 null "$3"' _ "$repo" "$writer" "$first"
  [ "$status" -eq 3 ]
  [[ "$output" == *"worktree changed while publishing checkpoint"* ]]
  [ ! -e "$repo/.git/development-system/checkpoints/post-snapshot-mutation.latest" ]
  git -C "$repo" checkout -- tracked.txt

  printf '%s\n' dirty >"$repo/tracked.txt"
  dirty_tracked=$(git -C "$repo" diff --binary --full-index HEAD -- | sha256sum | cut -d ' ' -f 1)
  dirty_json=$(jq -cn --arg head "$head_oid" --arg tracked "$dirty_tracked" --arg empty "$empty_sha" '{generation:0,predecessor_sha256:null,baseline_oid:$head,snapshot:{head_oid:$head,tracked_sha256:$tracked,untracked_sha256:$empty},state:"pushed-or-delivery-mode-equivalent",test:null,gates:{lightweight_review_receipt:null,fast_gate_receipt:null,exact_identity_verification_receipt:null},delivery:{mode:"local-only",commit_oid:null,pushed_oid:null,local_snapshot:"snapshot"},ci:{runs:[],terminal_success_run_id:null},next_action:"edit"}')
  printf 'checkpoint-v1 %s\n' "$dirty_json" >"$dirty_bootstrap"
  run bash -c 'cd "$1" && "$2" dirty-item 0 null "$3"' _ "$repo" "$writer" "$dirty_bootstrap"
  [ "$status" -ne 0 ]
  [ ! -e "$repo/.git/development-system/checkpoints/dirty-item.latest" ]

  git -C "$repo" checkout -- tracked.txt
  printf '%s\n' untracked >"$repo/untracked.txt"
  run bash -c 'cd "$1" && "$2" untracked-item 0 null "$3"' _ "$repo" "$writer" "$first"
  [ "$status" -ne 0 ]
  [ ! -e "$repo/.git/development-system/checkpoints/untracked-item.latest" ]

  rm "$repo/untracked.txt"
  ln -s missing-target "$repo/dangling-link"
  run bash -c 'cd "$1" && "$2" dangling-link-item 0 null "$3"' _ "$repo" "$writer" "$first"
  [ "$status" -ne 0 ]
  [[ "$output" == *"checkpoint record failed schema, snapshot, or state validation"* ]]
  [ ! -e "$repo/.git/development-system/checkpoints/dangling-link-item.latest" ]
}


@test "change-preflight benchmark rejects incomplete or speculative classifications" {
  workspace="$ROOT/evals/benchmarks/change-preflight/workspace"
  test_support="$ROOT/evals/benchmarks/change-preflight/test-support"

  changed_workspace="$(mktemp -d)"
  cp -R "$workspace/." "$changed_workspace/"
  printf '%s\n' 'implementation started early' >"$changed_workspace/project/implementation-target.txt"
  run node "$workspace/verify-change-preflight.mjs" "$test_support/fixtures/feature-valid.json" "$changed_workspace" feature
  [ "$status" -ne 0 ]

  changed_evidence_workspace="$(mktemp -d)"
  cp -R "$workspace/." "$changed_evidence_workspace/"
  printf '%s\n' 'implementation started early' >>"$changed_evidence_workspace/feature/request.md"
  run node "$workspace/verify-change-preflight.mjs" "$test_support/fixtures/feature-valid.json" "$changed_evidence_workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"representative repository changed before preflight completed"* ]]

  run node "$workspace/verify-change-preflight.mjs" "$test_support/fixtures/feature-valid.json" "$workspace" feature
  [ "$status" -eq 0 ]

  wrong_scenario="$(mktemp)"
  jq '.scenario = "docs-config"' "$test_support/fixtures/feature-valid.json" >"$wrong_scenario"
  run node "$workspace/verify-change-preflight.mjs" "$wrong_scenario" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"record does not match trusted scenario"* ]]

  run node "$workspace/verify-change-preflight.mjs" "$test_support/fixtures/docs-config-valid.json" "$workspace" docs-config
  [ "$status" -eq 0 ]

  run node "$workspace/verify-change-preflight.mjs" "$test_support/fixtures/migration-valid.json" "$workspace" migration
  [ "$status" -eq 0 ]

  live_root="$(mktemp -d -p /tmp plugin-eval-feature-XXXXXXXX)"
  live_workspace="$live_root/workspace"
  mkdir "$live_workspace"
  cp -R "$workspace/." "$live_workspace/"
  cp "$test_support/fixtures/feature-valid.json" "$live_workspace/change-preflight.json"
  verifier_command="node verify-change-preflight.mjs"
  pushd "$live_workspace" >/dev/null
  run /usr/bin/env bash -lc "$verifier_command"
  popd >/dev/null
  [ "$status" -eq 0 ]

  cross_case_root="$(mktemp -d -p /tmp plugin-eval-docs-config-XXXXXXXX)"
  cross_case_workspace="$cross_case_root/workspace"
  mkdir "$cross_case_workspace"
  cp -R "$workspace/." "$cross_case_workspace/"
  cp "$test_support/fixtures/feature-valid.json" "$cross_case_workspace/change-preflight.json"
  pushd "$cross_case_workspace" >/dev/null
  run /usr/bin/env bash -lc "$verifier_command"
  popd >/dev/null
  [ "$status" -ne 0 ]

  printf '%s\n' 'implementation started early' >"$live_workspace/project/implementation-target.txt"
  pushd "$live_workspace" >/dev/null
  run /usr/bin/env bash -lc "$verifier_command"
  popd >/dev/null
  [ "$status" -ne 0 ]

  live_plan_root="$(mktemp -d -p /tmp plugin-eval-feature-XXXXXXXX)"
  live_plan_workspace="$live_plan_root/workspace"
  mkdir "$live_plan_workspace"
  cp -R "$workspace/." "$live_plan_workspace/"
  jq '.surfaces.behavior.plan = ["edit source"]' "$test_support/fixtures/feature-valid.json" >"$live_plan_workspace/change-preflight.json"
  pushd "$live_plan_workspace" >/dev/null
  run /usr/bin/env bash -lc "$verifier_command"
  popd >/dev/null
  [ "$status" -ne 0 ]

  live_evidence_root="$(mktemp -d -p /tmp plugin-eval-feature-XXXXXXXX)"
  live_evidence_workspace="$live_evidence_root/workspace"
  mkdir "$live_evidence_workspace"
  cp -R "$workspace/." "$live_evidence_workspace/"
  cp "$test_support/fixtures/feature-valid.json" "$live_evidence_workspace/change-preflight.json"
  printf '%s\n' 'implementation started early' >>"$live_evidence_workspace/feature/request.md"
  pushd "$live_evidence_workspace" >/dev/null
  run /usr/bin/env bash -lc "$verifier_command"
  popd >/dev/null
  [ "$status" -ne 0 ]

  live_reason_root="$(mktemp -d -p /tmp plugin-eval-feature-XXXXXXXX)"
  live_reason_workspace="$live_reason_root/workspace"
  mkdir "$live_reason_workspace"
  cp -R "$workspace/." "$live_reason_workspace/"
  jq '.surfaces.behavior.reason = "Not applicable because behavior is unchanged."' "$test_support/fixtures/feature-valid.json" >"$live_reason_workspace/change-preflight.json"
  pushd "$live_reason_workspace" >/dev/null
  run /usr/bin/env bash -lc "$verifier_command"
  popd >/dev/null
  [ "$status" -ne 0 ]

  valid="$test_support/fixtures/feature-valid.json"
  invalid="$(mktemp)"

  jq 'del(.surfaces.evaluations)' "$valid" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"all and only the ten required surfaces"* ]]

  jq '.surfaces.behavior = {status:"not-applicable", evidence:["feature/src/commands.md"], reason:"No behavior changes are requested."}' "$valid" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"behavior must be applicable for feature"* ]]

  jq '.implementationPlan = ["edit source"]' "$valid" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"unexpected top-level fields: implementationPlan"* ]]

  jq '.surfaces.behavior.evidence = ["nearby code"]' "$valid" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"behavior evidence is not grounded"* ]]

  jq '.steps = []' "$valid" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"unexpected top-level fields: steps"* ]]

  jq '.surfaces.configuration.evidence = ["feature/src/commands.md"]' "$valid" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"configuration evidence is not grounded"* ]]

  jq '.surfaces.behavior.plan = ["edit source"]' "$valid" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"behavior has unexpected fields: plan"* ]]

  jq '.repositoryPolicyEvidence = ["repository policy"]' "$valid" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"repositoryPolicyEvidence must cite repository facts"* ]]

  jq '.repositoryPolicyEvidence = ["README.md"]' "$valid" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"repositoryPolicyEvidence must cite repository facts"* ]]

  jq '.surfaces.behavior.evidence = ["feature/request.md"]' "$valid" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"behavior evidence is not grounded"* ]]

  jq '.surfaces.configuration.reason = "Not applicable."' "$valid" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"configuration not-applicable decision needs a scenario-specific reason"* ]]

  jq '.surfaces.behavior.reason = "Not applicable because behavior is unchanged."' "$valid" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"behavior has unexpected fields: reason"* ]]

  jq '.repositoryPolicyEvidence = ["AGENTS.md", "README.md"]' "$valid" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"repositoryPolicyEvidence must cite repository facts"* ]]

  jq 'del(.surfaces.behavior.effect)' "$valid" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"behavior applicable decision needs a concrete effect"* ]]

  jq '.surfaces.configuration.reason = "This unrelated sentence is comfortably long enough."' "$valid" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"scenario-specific reason"* ]]

  jq '.surfaces.behavior.evidence += ["feature/request.md"]' "$valid" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"behavior evidence is not grounded"* ]]

  jq '.surfaces.behavior.effect = "This sentence is long but says nothing useful."' "$valid" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"behavior applicable decision needs a concrete effect"* ]]

  jq '.surfaces.behavior.effect = "No command or behavior changes are needed."' "$valid" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"behavior applicable decision needs a concrete effect"* ]]

  jq '.surfaces.behavior.effect = "Runtime behavior remains unchanged."' "$valid" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"behavior applicable decision needs a concrete effect"* ]]

  jq '.surfaces.configuration.reason = "Runtime configuration defaults absolutely change."' "$valid" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" feature
  [ "$status" -ne 0 ]
  [[ "$output" == *"configuration not-applicable decision needs a scenario-specific reason"* ]]

  jq 'del(.surfaces.migrations.decisions.backfill)' "$test_support/fixtures/migration-valid.json" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" migration
  [ "$status" -ne 0 ]
  [[ "$output" == *"compatibility, rollback, recovery, and backfill"* ]]

  jq '.surfaces.migrations.decisions.backfill = "This sentence is long but says nothing useful."' "$test_support/fixtures/migration-valid.json" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" migration
  [ "$status" -ne 0 ]
  [[ "$output" == *"compatibility, rollback, recovery, and backfill"* ]]

  jq '.surfaces.migrations.decisions.compatibility = "Compatibility will be broken and old repositories stop working." | .surfaces.migrations.decisions.rollback = "Rollback is impossible and unsupported for operators."' "$test_support/fixtures/migration-valid.json" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" migration
  [ "$status" -ne 0 ]
  [[ "$output" == *"compatibility, rollback, recovery, and backfill"* ]]

  jq '.surfaces.migrations.decisions.backfill = "Existing repositories will never receive backfill."' "$test_support/fixtures/migration-valid.json" >"$invalid"
  run node "$workspace/verify-change-preflight.mjs" "$invalid" "$workspace" migration
  [ "$status" -ne 0 ]
  [[ "$output" == *"compatibility, rollback, recovery, and backfill"* ]]
}
