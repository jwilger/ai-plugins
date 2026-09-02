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

  run grep -F "owner-only parent and serializes transitions with an exclusive task-scoped" "$skill"
  [ "$status" -eq 0 ]

  run grep -F 'generation` to equal the current generation plus one' "$skill"
  [ "$status" -eq 0 ]

  run grep -F 'predecessor_sha256` to equal SHA-256' "$skill"
  [ "$status" -eq 0 ]

  run grep -F "stale predecessor" "$skill"
  [ "$status" -eq 0 ]

  repo=$(mktemp -d)
  git -C "$repo" init -q
  git -C "$repo" -c user.name=Test -c user.email=test@example.invalid commit --allow-empty -m baseline -q
  records=$(mktemp -d)
  first="$records/first.record"
  second="$records/second.record"
  stale="$records/stale.record"
  invalid="$records/invalid.record"
  invalid_state="$records/invalid-state.record"
  invalid_framing="$records/invalid-framing.record"
  invalid_nul="$records/invalid-nul.record"
  invalid_local_ci="$records/invalid-local-ci.record"
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

  first_json=$(jq -cn --arg head "$head_oid" --arg empty "$empty_sha" '{generation:0,predecessor_sha256:null,baseline_oid:$head,snapshot:{head_oid:$head,tracked_sha256:$empty,untracked_sha256:$empty},state:"pushed-or-delivery-mode-equivalent",test:null,gates:{lightweight_review_receipt:null,fast_gate_receipt:null,exact_identity_verification_receipt:null},delivery:{mode:"local-only",commit_oid:null,pushed_oid:null,local_snapshot:"snapshot"},ci:{runs:[],terminal_success_run_id:null},next_action:"edit"}')
  printf 'checkpoint-v1 %s\n' "$first_json" >"$first"

  run bash -c 'cd "$1" && "$2" work-item 0 null "$3"' _ "$repo" "$writer" "$first"
  [ "$status" -eq 0 ]
  target="$repo/.git/development-system/checkpoints/work-item.latest"
  predecessor=$(sha256sum "$target" | cut -d ' ' -f 1)
  second_json=$(jq -cn --arg predecessor "$predecessor" --arg head "$head_oid" --arg empty "$empty_sha" '{generation:1,predecessor_sha256:$predecessor,baseline_oid:$head,snapshot:{head_oid:$head,tracked_sha256:$empty,untracked_sha256:$empty},state:"failing",test:{command:"test",receipt_ref:"receipt",outcome:"fail",failure_kind:"expected-red"},gates:{lightweight_review_receipt:null,fast_gate_receipt:null,exact_identity_verification_receipt:null},delivery:null,ci:{runs:[],terminal_success_run_id:null},next_action:"causal-edit"}')
  printf 'checkpoint-v1 %s\n' "$second_json" >"$second"

  run bash -c 'cd "$1" && "$2" work-item 1 "$3" "$4"' _ "$repo" "$writer" "$predecessor" "$second"
  [ "$status" -eq 0 ]
  current=$(<"$target")
  printf 'checkpoint-v1 %s\n' "$second_json" >"$stale"

  run bash -c 'cd "$1" && "$2" work-item 1 "$3" "$4"' _ "$repo" "$writer" "$predecessor" "$stale"
  [ "$status" -eq 3 ]
  [ "$(<"$target")" = "$current" ]
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
