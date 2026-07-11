#!/usr/bin/env bats

setup() {
  WORKFLOW="$BATS_TEST_DIRNAME/../../.github/workflows/ci.yml"
}

@test "runs CI for direct pushes to main" {
  run yq -e '.on.push.branches | any_c(. == "main")' "$WORKFLOW"

  [ "$status" -eq 0 ]
}

@test "keeps CI coverage for incoming pull requests" {
  run yq -e '.on.pull_request.branches | any_c(. == "main")' "$WORKFLOW"

  [ "$status" -eq 0 ]
}

@test "keeps CI coverage for merge queues" {
  run yq -e '.on.merge_group.types | any_c(. == "checks_requested")' "$WORKFLOW"

  [ "$status" -eq 0 ]
}

@test "does not replace pending main-push CI runs" {
  run yq -e \
    '. as $workflow | (($workflow | has("concurrency") | not) and ([$workflow.jobs[] | has("concurrency")] | all_c(. == false)))' \
    "$WORKFLOW"

  [ "$status" -eq 0 ]
}
