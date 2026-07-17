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
  [[ "$output" == *'ai-plugins-user-delegate.conf'* ]]
  [[ "$output" == *'/run/systemd/system/user@${uid}.service.d/ai-plugins-ci.conf'* ]]
  [[ "$output" == *"'[Service]'"* ]]
  [[ "$output" == *'Delegate=cpu memory pids'* ]]
  [[ "$output" == *'sudo install -D -m 0644 "$delegate_source" "$delegate_drop_in"'* ]]
  [[ "$output" == *'sudo systemctl daemon-reload'* ]]
  [[ "$output" == *'sudo loginctl enable-linger "$user"'* ]]
  [[ "$output" == *'for attempt in 1 2 3; do'* ]]
  [[ "$output" == *'sudo systemctl restart "$user_manager_unit"'* ]]
  [[ "$output" == *'sudo systemctl start "$user_manager_unit"'* ]]
  [[ "$output" == *'sudo systemctl status --no-pager --full'* ]]
  [[ "$output" == *'"$user_manager_unit" "$runtime_unit"'* ]]
  [[ "$output" == *'--property=ActiveState,SubState,Result,ExecMainStatus,ControlGroup,DelegateControllers'* ]]
  [[ "$output" == *'sudo journalctl --boot --no-pager --lines=100'* ]]
  [[ "$output" == *'--unit="$user_manager_unit" --unit="$runtime_unit"'* ]]
  [[ "$output" == *'sudo systemctl reset-failed "$user_manager_unit"'* ]]
  [[ "$output" == *'sleep "$attempt"'* ]]
  [[ "$output" == *'if [ "$activation_succeeded" != true ]; then'* ]]
  [[ "$output" != *'sudo systemctl set-property'* ]]
  [[ "$output" == *'sudo systemctl is-active --quiet "$user_manager_unit"'* ]]
  [[ "$output" == *'DelegateControllers'* ]]
  [[ "$output" == *'test -S "$runtime_dir/systemd/private"'* ]]
  [[ "$output" == *'XDG_RUNTIME_DIR="$runtime_dir" systemctl --user show-environment'* ]]
  [[ "$output" == *'XDG_RUNTIME_DIR=$runtime_dir'* ]]
  [[ "$output" == *'GITHUB_ENV'* ]]

  linger_line=""
  install_line=""
  reload_line=""
  restart_line=""
  activation_probe_line=""
  retry_loop_line=""
  probe_line=""
  for index in "${!lines[@]}"; do
    [[ "${lines[$index]}" == *'sudo loginctl enable-linger'* ]] && linger_line="$index"
    [[ "${lines[$index]}" == *'sudo install -D -m 0644'* ]] && install_line="$index"
    [[ "${lines[$index]}" == *'sudo systemctl daemon-reload'* ]] && reload_line="$index"
    [[ "${lines[$index]}" == *'sudo systemctl restart'* ]] && restart_line="$index"
    [[ "${lines[$index]}" == *'sudo systemctl is-active --quiet "$user_manager_unit"'* ]] && activation_probe_line="$index"
    [[ "${lines[$index]}" == *'for attempt in 1 2 3'* ]] && retry_loop_line="$index"
    [[ "${lines[$index]}" == *'systemctl --user show-environment'* ]] && probe_line="$index"
  done
  [ "$install_line" -lt "$reload_line" ]
  [ "$reload_line" -lt "$linger_line" ]
  [ "$linger_line" -lt "$restart_line" ]
  [ "$restart_line" -lt "$activation_probe_line" ]
  [ "$activation_probe_line" -lt "$retry_loop_line" ]
  [ "$restart_line" -lt "$probe_line" ]

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
