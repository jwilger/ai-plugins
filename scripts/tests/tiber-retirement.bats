#!/usr/bin/env bats

setup() {
  PLUGIN_ROOT="$(cd "$BATS_TEST_DIRNAME/../../plugins/development-system" && pwd)"
  CLI="$PLUGIN_ROOT/bin/development-system"
}

@test "installed development-system exposes its bundled Tiber tracker" {
  run "$CLI" tiber --help

  [ "$status" -eq 0 ]
  [[ "$output" == *"Repository-local task board"* ]]
  expected_commands=$'commands:\n  init\n  create --title <title>\n  list\n  claim <ticket-id> --owner <owner>\n  release <ticket-id> --owner <owner>\n  prioritize <ticket-id> --priority <0..4>\n  complete <ticket-id> --owner <owner>\n  next\n  depend <ticket-id> --on <dependency-id>\n  undepend <ticket-id> --on <dependency-id>'
  actual_commands="$(sed -n '/^commands:/,$p' <<<"$output")"
  [ "$actual_commands" = "$expected_commands" ]

  run find "$PLUGIN_ROOT" -path '*/components/tiber/bin/tiber' -type f -print

  [ "$status" -eq 0 ]
  [ -n "$output" ]
}

@test "Tiber initializes an Eventcore transaction store instead of ticket Markdown files" {
  project="$BATS_TEST_TMPDIR/project"
  mkdir -p "$project"

  run bash -c 'cd "$1" && "$2" tiber init' _ "$project" "$CLI"

  [ "$status" -eq 0 ]
  [ -d "$project/.development-system/tiber/store/events" ]
  [ -z "$(find "$project/.development-system/tiber/store/events" -name '*.md' -print -quit)" ]
}

@test "Tiber reports its initial deterministic workflow decision without recording an event" {
  project="$BATS_TEST_TMPDIR/project"
  mkdir -p "$project"

  run bash -c 'cd "$1" && "$2" tiber workflow status' _ "$project" "$CLI"
  [ "$status" -ne 0 ]
  [[ "$output" == *"tiber.workflow not_initialized"* ]]
  [ ! -e "$project/.development-system/tiber/store" ]

  run bash -c 'cd "$1" && "$2" tiber init' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  events_before="$(rg --fixed-strings '"record":"event"' "$project/.development-system/tiber/store/events" | wc -l)"

  run bash -c 'cd "$1" && "$2" tiber workflow status' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  [ "$output" = "tiber.workflow frontier=0 next=RecordDiscoveryEvidence evidence=artifact,observation,measurement events=DiscoveryEvidenceRecorded" ]
  events_after_first_status="$(rg --fixed-strings '"record":"event"' "$project/.development-system/tiber/store/events" | wc -l)"
  [ "$events_after_first_status" -eq "$events_before" ]

  run bash -c 'cd "$1" && "$2" tiber workflow status' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  [ "$output" = "tiber.workflow frontier=0 next=RecordDiscoveryEvidence evidence=artifact,observation,measurement events=DiscoveryEvidenceRecorded" ]
  events_after_second_status="$(rg --fixed-strings '"record":"event"' "$project/.development-system/tiber/store/events" | wc -l)"
  [ "$events_after_second_status" -eq "$events_before" ]
}

@test "Tiber records discovery evidence and advances its deterministic workflow" {
  project="$BATS_TEST_TMPDIR/project"
  mkdir -p "$project"

  run bash -c 'cd "$1" && "$2" tiber init' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  events_before="$(rg --fixed-strings '"record":"event"' "$project/.development-system/tiber/store/events" | wc -l)"

  run bash -c 'cd "$1" && "$2" tiber workflow execute RecordDiscoveryEvidence --event DiscoveryEvidenceRecorded --expected-frontier 0 --observation "Customer interviews show the manual handoff blocks delivery"' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  [ "$output" = "tiber.workflow recorded=DiscoveryEvidenceRecorded frontier=1 next=FormProductHypothesis evidence=derived events=ProductHypothesisFormed" ]
  events_after="$(rg --fixed-strings '"record":"event"' "$project/.development-system/tiber/store/events" | wc -l)"
  [ "$events_after" -eq $((events_before + 1)) ]

  run bash -c 'cd "$1" && "$2" tiber workflow status' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  [ "$output" = "tiber.workflow frontier=1 next=FormProductHypothesis evidence=derived events=ProductHypothesisFormed" ]
}

@test "Tiber rejects malformed discovery workflow arguments without appending events" {
  project="$BATS_TEST_TMPDIR/project"
  mkdir -p "$project"
  run bash -c 'cd "$1" && "$2" tiber init' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  before="$(rg --fixed-strings '"record":"event"' "$project/.development-system/tiber/store/events" | wc -l)"

  run bash -c 'cd "$1" && "$2" tiber workflow execute RecordDiscoveryEvidence --event DiscoveryEvidenceRecorded --expected-frontier 0 --observation --mistyped value' _ "$project" "$CLI"
  [ "$status" -ne 0 ]
  [ "$output" = "tiber.workflow command_unknown" ]

  run bash -c 'cd "$1" && "$2" tiber workflow execute RecordDiscoveryEvidence --event DiscoveryEvidenceRecorded --event DiscoveryEvidenceRecorded --expected-frontier 0 --observation valid' _ "$project" "$CLI"
  [ "$status" -ne 0 ]
  [ "$output" = "tiber.workflow command_unknown" ]

  run bash -c 'cd "$1" && "$2" tiber workflow execute RecordDiscoveryEvidence --event DiscoveryEvidenceRecorded --expected-frontier 0 --observation valid --mistyped value' _ "$project" "$CLI"
  [ "$status" -ne 0 ]
  [ "$output" = "tiber.workflow command_unknown" ]

  run bash -c 'cd "$1" && "$2" tiber workflow execute RecordDiscoveryEvidence --event DiscoveryEvidenceRecorded --expected-frontier --observation valid' _ "$project" "$CLI"
  [ "$status" -ne 0 ]
  [ "$output" = "tiber.workflow command_unknown" ]
  after="$(rg --fixed-strings '"record":"event"' "$project/.development-system/tiber/store/events" | wc -l)"
  [ "$after" -eq "$before" ]
}

@test "Tiber workflow replay ignores ticket events in the same Eventcore store" {
  project="$BATS_TEST_TMPDIR/project"
  mkdir -p "$project"
  run bash -c 'cd "$1" && "$2" tiber init' _ "$project" "$CLI"
  [ "$status" -eq 0 ]

  run bash -c 'cd "$1" && "$2" tiber create --title "Ticket event beside workflow event"' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  run bash -c 'cd "$1" && "$2" tiber workflow execute RecordDiscoveryEvidence --event DiscoveryEvidenceRecorded --expected-frontier 0 --observation valid' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  run bash -c 'cd "$1" && "$2" tiber workflow status' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  [ "$output" = "tiber.workflow frontier=1 next=FormProductHypothesis evidence=derived events=ProductHypothesisFormed" ]
}

@test "Tiber rejects invalid discovery workflow executions without appending events" {
  project="$BATS_TEST_TMPDIR/project"
  mkdir -p "$project"
  run bash -c 'cd "$1" && "$2" tiber init' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  before="$(rg --fixed-strings '"record":"event"' "$project/.development-system/tiber/store/events" | wc -l)"
  run bash -c 'cd "$1" && "$2" tiber workflow execute FormProductHypothesis --event ProductHypothesisFormed --expected-frontier 0 --observation x' _ "$project" "$CLI"
  [ "$status" -ne 0 ]
  [ "$output" = "tiber.workflow command_not_eligible" ]
  after="$(rg --fixed-strings '"record":"event"' "$project/.development-system/tiber/store/events" | wc -l)"
  [ "$after" -eq "$before" ]
}

@test "Tiber rejects empty discovery observations without appending workflow events" {
  project="$BATS_TEST_TMPDIR/project"
  mkdir -p "$project"
  run bash -c 'cd "$1" && "$2" tiber init' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  before="$(rg --fixed-strings '"record":"event"' "$project/.development-system/tiber/store/events" | wc -l)"
  run bash -c 'cd "$1" && "$2" tiber workflow execute RecordDiscoveryEvidence --event DiscoveryEvidenceRecorded --expected-frontier 0 --observation ""' _ "$project" "$CLI"
  [ "$status" -ne 0 ]
  [[ "$output" == *"invalid_observation"* ]]
  after="$(rg --fixed-strings '"record":"event"' "$project/.development-system/tiber/store/events" | wc -l)"
  [ "$after" -eq "$before" ]
}

@test "Tiber rejects unknown workflow commands and overlong observations without appending events" {
  project="$BATS_TEST_TMPDIR/project"
  mkdir -p "$project"
  run bash -c 'cd "$1" && "$2" tiber init' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  before="$(rg --fixed-strings '"record":"event"' "$project/.development-system/tiber/store/events" | wc -l)"

  run bash -c 'cd "$1" && "$2" tiber workflow execute UnknownWorkflowCommand --event UnknownWorkflowEvent --expected-frontier 0 --observation x' _ "$project" "$CLI"
  [ "$status" -ne 0 ]
  [ "$output" = "tiber.workflow command_unknown" ]

  overlong="$(printf '%1025s' '' | tr ' ' x)"
  run bash -c 'cd "$1" && "$2" tiber workflow execute RecordDiscoveryEvidence --event DiscoveryEvidenceRecorded --expected-frontier 0 --observation "$3"' _ "$project" "$CLI" "$overlong"
  [ "$status" -ne 0 ]
  [ "$output" = "tiber.workflow invalid_observation" ]
  after="$(rg --fixed-strings '"record":"event"' "$project/.development-system/tiber/store/events" | wc -l)"
  [ "$after" -eq "$before" ]
}

@test "Tiber rejects a stale discovery workflow frontier without appending an event" {
  project="$BATS_TEST_TMPDIR/project"
  mkdir -p "$project"
  run bash -c 'cd "$1" && "$2" tiber init' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  run bash -c 'cd "$1" && "$2" tiber workflow execute RecordDiscoveryEvidence --event DiscoveryEvidenceRecorded --expected-frontier 0 --observation first' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  before="$(rg --fixed-strings '"record":"event"' "$project/.development-system/tiber/store/events" | wc -l)"
  run bash -c 'cd "$1" && "$2" tiber workflow execute RecordDiscoveryEvidence --event DiscoveryEvidenceRecorded --expected-frontier 0 --observation retry' _ "$project" "$CLI"
  [ "$status" -ne 0 ]
  [[ "$output" == *"stale_frontier expected=0 actual=1"* ]]
  after="$(rg --fixed-strings '"record":"event"' "$project/.development-system/tiber/store/events" | wc -l)"
  [ "$after" -eq "$before" ]
}

@test "Tiber rejects a workflow event that the selected command cannot emit" {
  project="$BATS_TEST_TMPDIR/project"
  mkdir -p "$project"
  run bash -c 'cd "$1" && "$2" tiber init' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  before="$(rg --fixed-strings '"record":"event"' "$project/.development-system/tiber/store/events" | wc -l)"
  run bash -c 'cd "$1" && "$2" tiber workflow execute RecordDiscoveryEvidence --event ProductHypothesisFormed --expected-frontier 0 --observation x' _ "$project" "$CLI"
  [ "$status" -ne 0 ]
  [[ "$output" == *"event_not_emitted_by_command"* ]]
  after="$(rg --fixed-strings '"record":"event"' "$project/.development-system/tiber/store/events" | wc -l)"
  [ "$after" -eq "$before" ]
}

@test "Tiber fails closed when a well-formed replayed workflow event has an unknown semantic event" {
  project="$BATS_TEST_TMPDIR/project"
  mkdir -p "$project"
  run bash -c 'cd "$1" && "$2" tiber init' _ "$project" "$CLI"
  [ "$status" -eq 0 ]

  run bash -c 'cd "$1" && TIBER_WORKFLOW_TEST_MODE=1 "$2" tiber workflow test-append --command RecordDiscoveryEvidence --event UnknownWorkflowEvent --observation fixture' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  before_status="$(rg --fixed-strings '"record":"event"' "$project/.development-system/tiber/store/events" | wc -l)"

  run bash -c 'cd "$1" && "$2" tiber workflow status' _ "$project" "$CLI"
  [ "$status" -ne 0 ]
  [ "$output" = "tiber.workflow replay_failed reason=event_unknown" ]
  after_status="$(rg --fixed-strings '"record":"event"' "$project/.development-system/tiber/store/events" | wc -l)"
  [ "$after_status" -eq "$before_status" ]
}

@test "Tiber fails closed when a well-formed replayed workflow event is impossible at its frontier" {
  project="$BATS_TEST_TMPDIR/project"
  mkdir -p "$project"
  run bash -c 'cd "$1" && "$2" tiber init' _ "$project" "$CLI"
  [ "$status" -eq 0 ]

  run bash -c 'cd "$1" && TIBER_WORKFLOW_TEST_MODE=1 "$2" tiber workflow test-append --command FormProductHypothesis --event ProductHypothesisFormed --observation fixture' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  before_status="$(rg --fixed-strings '"record":"event"' "$project/.development-system/tiber/store/events" | wc -l)"

  run bash -c 'cd "$1" && "$2" tiber workflow status' _ "$project" "$CLI"
  [ "$status" -ne 0 ]
  [ "$output" = "tiber.workflow replay_failed reason=command_not_eligible" ]
  after_status="$(rg --fixed-strings '"record":"event"' "$project/.development-system/tiber/store/events" | wc -l)"
  [ "$after_status" -eq "$before_status" ]
}

@test "Tiber creates a functional ticket as an immutable Eventcore transaction" {
  project="$BATS_TEST_TMPDIR/project"
  mkdir -p "$project"

  run bash -c 'cd "$1" && "$2" tiber create --title "Restore deterministic workflow"' _ "$project" "$CLI"

  [ "$status" -eq 0 ]
  [[ "$output" == *"Restore deterministic workflow"* ]]
  event_file="$(find "$project/.development-system/tiber/store/events" -name '*.jsonl' -print -quit)"
  [ -n "$event_file" ]
  run rg --fixed-strings 'Restore deterministic workflow' "$event_file"
  [ "$status" -eq 0 ]
  [ -z "$(find "$project" -name '*.md' -print -quit)" ]
}

@test "Tiber lists ticket titles by replaying Eventcore transactions" {
  project="$BATS_TEST_TMPDIR/project"
  mkdir -p "$project"

  run bash -c 'cd "$1" && "$2" tiber create --title "Replay this ticket"' _ "$project" "$CLI"
  [ "$status" -eq 0 ]

  run bash -c 'cd "$1" && "$2" tiber list' _ "$project" "$CLI"

  [ "$status" -eq 0 ]
  [[ "$output" == *"Replay this ticket"* ]]
}

@test "Tiber list exposes stable ticket identifiers from Eventcore streams" {
  project="$BATS_TEST_TMPDIR/project"
  mkdir -p "$project"

  run bash -c 'cd "$1" && "$2" tiber create --title "Addressable ticket"' _ "$project" "$CLI"
  [ "$status" -eq 0 ]

  run bash -c 'cd "$1" && "$2" tiber list' _ "$project" "$CLI"

  [ "$status" -eq 0 ]
  [[ "$output" == *"id=ticket-"* ]]
  [[ "$output" == *"title=Addressable ticket"* ]]
}

@test "Tiber claims an addressable ticket through Eventcore" {
  project="$BATS_TEST_TMPDIR/project"
  mkdir -p "$project"

  run bash -c 'cd "$1" && "$2" tiber create --title "Claimable ticket"' _ "$project" "$CLI"
  [ "$status" -eq 0 ]

  run bash -c 'cd "$1" && "$2" tiber list' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  ticket_id="$(sed -n 's/.*id=\([^ ]*\).*/\1/p' <<<"$output")"
  [ -n "$ticket_id" ]

  run bash -c 'cd "$1" && "$2" tiber claim "$3" --owner alice' _ "$project" "$CLI" "$ticket_id"
  [ "$status" -eq 0 ]

  run bash -c 'cd "$1" && "$2" tiber list' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  [[ "$output" == *"id=$ticket_id"* ]]
  [[ "$output" == *"owner=alice"* ]]
}

@test "Tiber preserves the first owner when a second claim is rejected" {
  project="$BATS_TEST_TMPDIR/project"
  mkdir -p "$project"

  run bash -c 'cd "$1" && "$2" tiber create --title "Exclusively claimable"' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  run bash -c 'cd "$1" && "$2" tiber list' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  ticket_id="$(sed -n 's/.*id=\([^ ]*\).*/\1/p' <<<"$output")"

  run bash -c 'cd "$1" && "$2" tiber claim "$3" --owner alice' _ "$project" "$CLI" "$ticket_id"
  [ "$status" -eq 0 ]
  run bash -c 'cd "$1" && "$2" tiber claim "$3" --owner bob' _ "$project" "$CLI" "$ticket_id"
  [ "$status" -ne 0 ]

  run bash -c 'cd "$1" && "$2" tiber list' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  [[ "$output" == *"id=$ticket_id"* ]]
  [[ "$output" == *"owner=alice"* ]]
  [[ "$output" != *"owner=bob"* ]]
}

@test "Tiber releases only the current owner's claim and permits a new claim" {
  project="$BATS_TEST_TMPDIR/project"
  mkdir -p "$project"

  run bash -c 'cd "$1" && "$2" tiber create --title "Releasable ticket"' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  run bash -c 'cd "$1" && "$2" tiber list' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  ticket_id="$(sed -n 's/.*id=\([^ ]*\).*/\1/p' <<<"$output")"
  [ -n "$ticket_id" ]

  run bash -c 'cd "$1" && "$2" tiber claim "$3" --owner alice' _ "$project" "$CLI" "$ticket_id"
  [ "$status" -eq 0 ]
  run bash -c 'cd "$1" && "$2" tiber release "$3" --owner alice' _ "$project" "$CLI" "$ticket_id"
  [ "$status" -eq 0 ]
  run bash -c 'cd "$1" && "$2" tiber claim "$3" --owner bob' _ "$project" "$CLI" "$ticket_id"
  [ "$status" -eq 0 ]

  run bash -c 'cd "$1" && "$2" tiber release "$3" --owner alice' _ "$project" "$CLI" "$ticket_id"
  [ "$status" -ne 0 ]
  run bash -c 'cd "$1" && "$2" tiber list' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  [[ "$output" == *"id=$ticket_id"* ]]
  [[ "$output" == *"owner=bob"* ]]
}

@test "Tiber records ticket and board release events in one Eventcore transaction" {
  project="$BATS_TEST_TMPDIR/project"
  mkdir -p "$project"

  run bash -c 'cd "$1" && "$2" tiber create --title "Atomically releasable"' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  run bash -c 'cd "$1" && "$2" tiber list' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  ticket_id="$(sed -n 's/.*id=\([^ ]*\).*/\1/p' <<<"$output")"

  run bash -c 'cd "$1" && "$2" tiber claim "$3" --owner alice' _ "$project" "$CLI" "$ticket_id"
  [ "$status" -eq 0 ]
  run bash -c 'cd "$1" && "$2" tiber release "$3" --owner alice' _ "$project" "$CLI" "$ticket_id"
  [ "$status" -eq 0 ]

  release_transaction="$(rg -l --fixed-strings '"TicketClaimReleasedV1"' "$project/.development-system/tiber/store/events")"
  [ -n "$release_transaction" ]
  [ "$(wc -l <<<"$release_transaction")" -eq 1 ]
  run rg --fixed-strings '"BoardTicketClaimReleasedV1"' "$release_transaction"
  [ "$status" -eq 0 ]
  run rg --fixed-strings '"stream_id":"board-local"' "$release_transaction"
  [ "$status" -eq 0 ]
}

@test "Tiber list replays claim changes beyond the first Eventcore page" {
  project="$BATS_TEST_TMPDIR/project"
  mkdir -p "$project"

  run bash -c 'cd "$1" && "$2" tiber create --title "Paged ticket"' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  run bash -c 'cd "$1" && "$2" tiber list' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  ticket_id="$(sed -n 's/.*id=\([^ ]*\).*/\1/p' <<<"$output")"
  [ -n "$ticket_id" ]

  run bash -c 'cd "$1" && "$2" tiber claim "$3" --owner alice' _ "$project" "$CLI" "$ticket_id"
  [ "$status" -eq 0 ]
  run bash -c '
    set -euo pipefail
    cd "$1"
    for ((cycle = 0; cycle < 24; cycle++)); do
      "$2" tiber release "$3" --owner alice >/dev/null
      "$2" tiber claim "$3" --owner alice >/dev/null
    done
  ' _ "$project" "$CLI" "$ticket_id"
  [ "$status" -eq 0 ]

  run bash -c 'cd "$1" && "$2" tiber create --title "Boundary filler"' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  run bash -c 'cd "$1" && "$2" tiber release "$3" --owner alice' _ "$project" "$CLI" "$ticket_id"
  [ "$status" -eq 0 ]
  run bash -c 'cd "$1" && "$2" tiber claim "$3" --owner bob' _ "$project" "$CLI" "$ticket_id"
  [ "$status" -eq 0 ]

  event_count="$(rg --fixed-strings '"record":"event"' "$project/.development-system/tiber/store/events" | wc -l)"
  [ "$event_count" -eq 104 ]
  run bash -c 'cd "$1" && "$2" tiber list' _ "$project" "$CLI"

  [ "$status" -eq 0 ]
  [[ "$output" == *"id=$ticket_id title=Paged ticket owner=bob"* ]]
}

@test "Tiber persists board priorities with backward-compatible defaults" {
  project="$BATS_TEST_TMPDIR/project"
  mkdir -p "$project"

  run bash -c 'cd "$1" && "$2" tiber create --title "Earlier ticket"' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  run bash -c 'cd "$1" && "$2" tiber list' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  earlier_ticket_id="$(sed -n 's/.*id=\([^ ]*\).*title=Earlier ticket.*/\1/p' <<<"$output")"
  [ -n "$earlier_ticket_id" ]

  run bash -c 'cd "$1" && "$2" tiber create --title "Later ticket"' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  run bash -c 'cd "$1" && "$2" tiber list' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  later_ticket_id="$(sed -n 's/.*id=\([^ ]*\).*title=Later ticket.*/\1/p' <<<"$output")"
  [ -n "$later_ticket_id" ]

  run bash -c 'cd "$1" && "$2" tiber prioritize "$3" --priority 0' _ "$project" "$CLI" "$later_ticket_id"
  [ "$status" -eq 0 ]

  run bash -c 'cd "$1" && "$2" tiber list' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  [[ "$(head -n 1 <<<"$output")" == *"id=$later_ticket_id title=Later ticket owner=unclaimed priority=0 completed=false blocked_by=none"* ]]
  [[ "$output" == *"id=$earlier_ticket_id title=Earlier ticket owner=unclaimed priority=2 completed=false blocked_by=none"* ]]

  event_count_before_unknown="$(rg --fixed-strings 'BoardTicketPrioritySetV1' "$project/.development-system/tiber/store/events" | wc -l)"
  run bash -c 'cd "$1" && "$2" tiber prioritize ticket-missing --priority 0' _ "$project" "$CLI"
  [ "$status" -ne 0 ]
  event_count_after_unknown="$(rg --fixed-strings 'BoardTicketPrioritySetV1' "$project/.development-system/tiber/store/events" | wc -l)"
  [ "$event_count_after_unknown" -eq "$event_count_before_unknown" ]
}

@test "Tiber completes only the current owner's ticket atomically" {
  project="$BATS_TEST_TMPDIR/project"
  mkdir -p "$project"

  run bash -c 'cd "$1" && "$2" tiber create --title "Completable ticket"' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  run bash -c 'cd "$1" && "$2" tiber list' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  ticket_id="$(sed -n 's/.*id=\([^ ]*\).*title=Completable ticket.*/\1/p' <<<"$output")"
  [ -n "$ticket_id" ]

  run bash -c 'cd "$1" && "$2" tiber claim "$3" --owner alice' _ "$project" "$CLI" "$ticket_id"
  [ "$status" -eq 0 ]

  completion_events_before_wrong_owner="$(rg --fixed-strings 'TicketCompletedV1' "$project/.development-system/tiber/store/events" | wc -l)"
  run bash -c 'cd "$1" && "$2" tiber complete "$3" --owner bob' _ "$project" "$CLI" "$ticket_id"
  [ "$status" -ne 0 ]
  completion_events_after_wrong_owner="$(rg --fixed-strings 'TicketCompletedV1' "$project/.development-system/tiber/store/events" | wc -l)"
  [ "$completion_events_after_wrong_owner" -eq "$completion_events_before_wrong_owner" ]
  run bash -c 'cd "$1" && "$2" tiber list' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  [[ "$output" == *"id=$ticket_id"* ]]
  [[ "$output" == *"owner=alice"* ]]

  run bash -c 'cd "$1" && "$2" tiber complete "$3" --owner alice' _ "$project" "$CLI" "$ticket_id"
  [ "$status" -eq 0 ]
  completion_transaction="$(rg -l --fixed-strings '"TicketCompletedV1"' "$project/.development-system/tiber/store/events")"
  [ -n "$completion_transaction" ]
  [ "$(wc -l <<<"$completion_transaction")" -eq 1 ]
  run rg --fixed-strings '"BoardTicketCompletedV1"' "$completion_transaction"
  [ "$status" -eq 0 ]

  run bash -c 'cd "$1" && "$2" tiber list' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  [[ "$output" == *"id=$ticket_id"* ]]
  [[ "$output" == *"owner=unclaimed"* ]]
  [[ "$output" == *"completed=true"* ]]

  completion_events_before_repeated="$(rg --fixed-strings 'TicketCompletedV1' "$project/.development-system/tiber/store/events" | wc -l)"
  run bash -c 'cd "$1" && "$2" tiber complete "$3" --owner alice' _ "$project" "$CLI" "$ticket_id"
  [ "$status" -ne 0 ]
  run bash -c 'cd "$1" && "$2" tiber complete ticket-missing --owner alice' _ "$project" "$CLI"
  [ "$status" -ne 0 ]
  run bash -c 'cd "$1" && "$2" tiber complete "$3"' _ "$project" "$CLI" "$ticket_id"
  [ "$status" -ne 0 ]
  completion_events_after_invalid="$(rg --fixed-strings 'TicketCompletedV1' "$project/.development-system/tiber/store/events" | wc -l)"
  [ "$completion_events_after_invalid" -eq "$completion_events_before_repeated" ]

  claim_events_before_reclaim="$(rg --fixed-strings 'TicketClaimedV1' "$project/.development-system/tiber/store/events" | wc -l)"
  run bash -c 'cd "$1" && "$2" tiber claim "$3" --owner bob' _ "$project" "$CLI" "$ticket_id"
  [ "$status" -ne 0 ]
  claim_events_after_reclaim="$(rg --fixed-strings 'TicketClaimedV1' "$project/.development-system/tiber/store/events" | wc -l)"
  [ "$claim_events_after_reclaim" -eq "$claim_events_before_reclaim" ]
}

@test "Tiber selects the next unclaimed incomplete ticket without mutation" {
  project="$BATS_TEST_TMPDIR/project"
  mkdir -p "$project"

  run bash -c 'cd "$1" && "$2" tiber create --title "Earlier eligible ticket"' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  run bash -c 'cd "$1" && "$2" tiber list' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  earlier_ticket_id="$(sed -n 's/.*id=\([^ ]*\).*title=Earlier eligible ticket.*/\1/p' <<<"$output")"
  [ -n "$earlier_ticket_id" ]

  run bash -c 'cd "$1" && "$2" tiber create --title "Higher priority ticket"' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  run bash -c 'cd "$1" && "$2" tiber list' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  higher_ticket_id="$(sed -n 's/.*id=\([^ ]*\).*title=Higher priority ticket.*/\1/p' <<<"$output")"
  [ -n "$higher_ticket_id" ]
  run bash -c 'cd "$1" && "$2" tiber prioritize "$3" --priority 0' _ "$project" "$CLI" "$higher_ticket_id"
  [ "$status" -eq 0 ]

  events_before_next="$(rg --fixed-strings '"record":"event"' "$project/.development-system/tiber/store/events" | wc -l)"
  run bash -c 'cd "$1" && "$2" tiber next' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  [[ "$output" == "tiber.ticket id=$higher_ticket_id title=Higher priority ticket owner=unclaimed priority=0 completed=false blocked_by=none" ]]
  events_after_next="$(rg --fixed-strings '"record":"event"' "$project/.development-system/tiber/store/events" | wc -l)"
  [ "$events_after_next" -eq "$events_before_next" ]

  run bash -c 'cd "$1" && "$2" tiber claim "$3" --owner alice' _ "$project" "$CLI" "$higher_ticket_id"
  [ "$status" -eq 0 ]
  run bash -c 'cd "$1" && "$2" tiber next' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  [[ "$output" == "tiber.ticket id=$earlier_ticket_id title=Earlier eligible ticket owner=unclaimed priority=2 completed=false blocked_by=none" ]]

  run bash -c 'cd "$1" && "$2" tiber complete "$3" --owner alice' _ "$project" "$CLI" "$higher_ticket_id"
  [ "$status" -eq 0 ]
  run bash -c 'cd "$1" && "$2" tiber claim "$3" --owner bob' _ "$project" "$CLI" "$earlier_ticket_id"
  [ "$status" -eq 0 ]
  run bash -c 'cd "$1" && "$2" tiber next' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  [ -z "$output" ]
}

@test "Tiber blocks ticket selection and claims on unresolved dependencies" {
  project="$BATS_TEST_TMPDIR/project"
  mkdir -p "$project"

  run bash -c 'cd "$1" && "$2" tiber create --title "Dependent P0 ticket"' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  run bash -c 'cd "$1" && "$2" tiber list' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  dependent_ticket_id="$(sed -n 's/.*id=\([^ ]*\).*title=Dependent P0 ticket.*/\1/p' <<<"$output")"
  [ -n "$dependent_ticket_id" ]
  run bash -c 'cd "$1" && "$2" tiber prioritize "$3" --priority 0' _ "$project" "$CLI" "$dependent_ticket_id"
  [ "$status" -eq 0 ]

  run bash -c 'cd "$1" && "$2" tiber create --title "Blocking P2 ticket"' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  run bash -c 'cd "$1" && "$2" tiber list' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  blocker_ticket_id="$(sed -n 's/.*id=\([^ ]*\).*title=Blocking P2 ticket.*/\1/p' <<<"$output")"
  [ -n "$blocker_ticket_id" ]

  dependency_events_before="$(rg --fixed-strings 'BoardTicketDependencyAddedV1' "$project/.development-system/tiber/store/events" | wc -l)"
  run bash -c 'cd "$1" && "$2" tiber depend "$3" --on "$4"' _ "$project" "$CLI" "$dependent_ticket_id" "$blocker_ticket_id"
  [ "$status" -eq 0 ]
  dependency_events_after="$(rg --fixed-strings 'BoardTicketDependencyAddedV1' "$project/.development-system/tiber/store/events" | wc -l)"
  [ "$dependency_events_after" -eq $((dependency_events_before + 1)) ]

  run bash -c 'cd "$1" && "$2" tiber list' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  [[ "$output" == *"id=$dependent_ticket_id"*"blocked_by=$blocker_ticket_id"* ]]
  run bash -c 'cd "$1" && "$2" tiber next' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  [[ "$output" == "tiber.ticket id=$blocker_ticket_id title=Blocking P2 ticket owner=unclaimed priority=2 completed=false blocked_by=none" ]]

  claim_events_before="$(rg --fixed-strings 'TicketClaimedV1' "$project/.development-system/tiber/store/events" | wc -l)"
  run bash -c 'cd "$1" && "$2" tiber claim "$3" --owner alice' _ "$project" "$CLI" "$dependent_ticket_id"
  [ "$status" -ne 0 ]
  claim_events_after="$(rg --fixed-strings 'TicketClaimedV1' "$project/.development-system/tiber/store/events" | wc -l)"
  [ "$claim_events_after" -eq "$claim_events_before" ]

  run bash -c 'cd "$1" && "$2" tiber claim "$3" --owner alice' _ "$project" "$CLI" "$blocker_ticket_id"
  [ "$status" -eq 0 ]
  run bash -c 'cd "$1" && "$2" tiber complete "$3" --owner alice' _ "$project" "$CLI" "$blocker_ticket_id"
  [ "$status" -eq 0 ]
  run bash -c 'cd "$1" && "$2" tiber next' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  [[ "$output" == "tiber.ticket id=$dependent_ticket_id title=Dependent P0 ticket owner=unclaimed priority=0 completed=false blocked_by=none" ]]

  run bash -c 'cd "$1" && "$2" tiber claim "$3" --owner bob' _ "$project" "$CLI" "$dependent_ticket_id"
  [ "$status" -eq 0 ]
  dependency_events_before_cycle="$(rg --fixed-strings 'BoardTicketDependencyAddedV1' "$project/.development-system/tiber/store/events" | wc -l)"
  run bash -c 'cd "$1" && "$2" tiber depend "$3" --on "$4"' _ "$project" "$CLI" "$blocker_ticket_id" "$dependent_ticket_id"
  [ "$status" -ne 0 ]
  dependency_events_after_cycle="$(rg --fixed-strings 'BoardTicketDependencyAddedV1' "$project/.development-system/tiber/store/events" | wc -l)"
  [ "$dependency_events_after_cycle" -eq "$dependency_events_before_cycle" ]
}

@test "Tiber rejects invalid dependency graph changes without writing events" {
  project="$BATS_TEST_TMPDIR/project"
  mkdir -p "$project"

  for title in "Ticket A" "Ticket B" "Ticket C"; do
    run bash -c 'cd "$1" && "$2" tiber create --title "$3"' _ "$project" "$CLI" "$title"
    [ "$status" -eq 0 ]
  done
  run bash -c 'cd "$1" && "$2" tiber list' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  ticket_a="$(sed -n 's/.*id=\([^ ]*\).*title=Ticket A.*/\1/p' <<<"$output")"
  ticket_b="$(sed -n 's/.*id=\([^ ]*\).*title=Ticket B.*/\1/p' <<<"$output")"
  ticket_c="$(sed -n 's/.*id=\([^ ]*\).*title=Ticket C.*/\1/p' <<<"$output")"
  [ -n "$ticket_a" ]
  [ -n "$ticket_b" ]
  [ -n "$ticket_c" ]

  run bash -c 'cd "$1" && "$2" tiber depend "$3" --on "$4"' _ "$project" "$CLI" "$ticket_a" "$ticket_b"
  [ "$status" -eq 0 ]
  dependency_events_before_invalid="$(rg --fixed-strings 'BoardTicketDependencyAddedV1' "$project/.development-system/tiber/store/events" | wc -l)"

  run bash -c 'cd "$1" && "$2" tiber depend "$3" --on "$4"' _ "$project" "$CLI" "$ticket_a" ticket-missing
  [ "$status" -ne 0 ]
  run bash -c 'cd "$1" && "$2" tiber depend "$3" --on "$4"' _ "$project" "$CLI" "$ticket_a" "$ticket_a"
  [ "$status" -ne 0 ]
  run bash -c 'cd "$1" && "$2" tiber depend "$3" --on "$4"' _ "$project" "$CLI" "$ticket_a" "$ticket_b"
  [ "$status" -ne 0 ]
  run bash -c 'cd "$1" && "$2" tiber depend "$3" --on "$4"' _ "$project" "$CLI" "$ticket_b" "$ticket_a"
  [ "$status" -ne 0 ]

  run bash -c 'cd "$1" && "$2" tiber depend "$3" --on "$4"' _ "$project" "$CLI" "$ticket_b" "$ticket_c"
  [ "$status" -eq 0 ]
  dependency_events_before_transitive_cycle="$(rg --fixed-strings 'BoardTicketDependencyAddedV1' "$project/.development-system/tiber/store/events" | wc -l)"
  run bash -c 'cd "$1" && "$2" tiber depend "$3" --on "$4"' _ "$project" "$CLI" "$ticket_c" "$ticket_a"
  [ "$status" -ne 0 ]
  dependency_events_after_invalid="$(rg --fixed-strings 'BoardTicketDependencyAddedV1' "$project/.development-system/tiber/store/events" | wc -l)"
  [ "$dependency_events_after_invalid" -eq "$dependency_events_before_transitive_cycle" ]
  [ "$dependency_events_before_transitive_cycle" -eq $((dependency_events_before_invalid + 1)) ]
}

@test "Tiber removes a direct dependency and restores ticket eligibility" {
  project="$BATS_TEST_TMPDIR/project"
  mkdir -p "$project"

  run bash -c 'cd "$1" && "$2" tiber create --title "Dependent ticket"' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  run bash -c 'cd "$1" && "$2" tiber list' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  dependent_ticket_id="$(sed -n 's/.*id=\([^ ]*\).*title=Dependent ticket.*/\1/p' <<<"$output")"
  [ -n "$dependent_ticket_id" ]
  run bash -c 'cd "$1" && "$2" tiber prioritize "$3" --priority 0' _ "$project" "$CLI" "$dependent_ticket_id"
  [ "$status" -eq 0 ]

  run bash -c 'cd "$1" && "$2" tiber create --title "Blocking ticket"' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  run bash -c 'cd "$1" && "$2" tiber list' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  blocker_ticket_id="$(sed -n 's/.*id=\([^ ]*\).*title=Blocking ticket.*/\1/p' <<<"$output")"
  [ -n "$blocker_ticket_id" ]
  run bash -c 'cd "$1" && "$2" tiber depend "$3" --on "$4"' _ "$project" "$CLI" "$dependent_ticket_id" "$blocker_ticket_id"
  [ "$status" -eq 0 ]

  run bash -c 'cd "$1" && "$2" tiber next' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  [[ "$output" == *"id=$blocker_ticket_id"* ]]
  run bash -c 'cd "$1" && "$2" tiber claim "$3" --owner alice' _ "$project" "$CLI" "$dependent_ticket_id"
  [ "$status" -ne 0 ]

  removal_events_before="$(rg --fixed-strings 'BoardTicketDependencyRemovedV1' "$project/.development-system/tiber/store/events" | wc -l)"
  run bash -c 'cd "$1" && "$2" tiber undepend "$3" --on "$4"' _ "$project" "$CLI" "$dependent_ticket_id" "$blocker_ticket_id"
  [ "$status" -eq 0 ]
  removal_events_after="$(rg --fixed-strings 'BoardTicketDependencyRemovedV1' "$project/.development-system/tiber/store/events" | wc -l)"
  [ "$removal_events_after" -eq $((removal_events_before + 1)) ]
  run bash -c 'cd "$1" && "$2" tiber list' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  [[ "$output" == *"id=$dependent_ticket_id"*"blocked_by=none"* ]]
  run bash -c 'cd "$1" && "$2" tiber next' _ "$project" "$CLI"
  [ "$status" -eq 0 ]
  [[ "$output" == *"id=$dependent_ticket_id"* ]]
  run bash -c 'cd "$1" && "$2" tiber claim "$3" --owner alice' _ "$project" "$CLI" "$dependent_ticket_id"
  [ "$status" -eq 0 ]
  run bash -c 'cd "$1" && "$2" tiber undepend "$3" --on "$4"' _ "$project" "$CLI" "$dependent_ticket_id" "$blocker_ticket_id"
  [ "$status" -ne 0 ]
  removal_events_final="$(rg --fixed-strings 'BoardTicketDependencyRemovedV1' "$project/.development-system/tiber/store/events" | wc -l)"
  [ "$removal_events_final" -eq "$removal_events_after" ]
}
