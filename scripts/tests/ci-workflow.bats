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

@test "runs Bubblewrap-backed quality tests on a user-namespace-capable image" {
  run yq -e '.jobs.quality."runs-on" == "ubuntu-22.04"' "$WORKFLOW"

  [ "$status" -eq 0 ]
}

@test "runs quality tests inside a controller-delegated user manager" {
  run yq -r '.jobs.quality.steps[].name' "$WORKFLOW"

  [ "$status" -eq 0 ]

  provision_index=""
  gate_index=""
  for index in "${!lines[@]}"; do
    case "${lines[$index]}" in
      "Provision delegated user systemd") provision_index="$index" ;;
      "Full gate") gate_index="$index" ;;
    esac
  done
  [ -n "$provision_index" ]
  [ -n "$gate_index" ]
  [ "$provision_index" -lt "$gate_index" ]

  run yq -r \
    '.jobs.quality.steps[] | select(.name == "Provision delegated user systemd") | .run' \
    "$WORKFLOW"

  [ "$status" -eq 0 ]
  [[ "$output" == *'sudo systemctl is-active --quiet systemd-logind.service'* ]]
  [[ "$output" == *'sudo loginctl enable-linger "$user"'* ]]
  [[ "$output" == *'sudo systemctl set-property --runtime "user@$uid.service"'* ]]
  [[ "$output" == *'Delegate=cpu memory pids'* ]]
  [[ "$output" == *'sudo systemctl start "user@$uid.service"'* ]]
  [[ "$output" == *'sudo systemctl is-active --quiet "user@$uid.service"'* ]]
  [[ "$output" == *'DelegateControllers'* ]]
  [[ "$output" == *'test -S "$runtime_dir/systemd/private"'* ]]
  [[ "$output" == *'XDG_RUNTIME_DIR="$runtime_dir" systemctl --user show-environment'* ]]
  [[ "$output" == *'XDG_RUNTIME_DIR=$runtime_dir'* ]]
  [[ "$output" == *'GITHUB_ENV'* ]]

  run yq -r \
    '.jobs.quality.steps[] | select(.name == "Full gate") | .run' \
    "$WORKFLOW"

  [ "$status" -eq 0 ]
  [[ "$output" == *'systemd-run'* ]]
  [[ "$output" == *'--user'* ]]
  [[ "$output" == *'--wait'* ]]
  [[ "$output" == *'--pipe'* ]]
  [[ "$output" == *'--collect'* ]]
  [[ "$output" == *'--working-directory="$GITHUB_WORKSPACE"'* ]]
  [[ "$output" == *'--setenv=CI=true'* ]]
  [[ "$output" == *'scorer contains its full process tree in the fixed aggregate systemd scope'* ]]
  [[ "$output" == *'direct public verifier uses the fixed aggregate scope without ambient secrets'* ]]
  [[ "$output" == *'just ci'* ]]
}
