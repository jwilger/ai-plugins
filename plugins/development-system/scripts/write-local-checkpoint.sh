#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "usage: write-local-checkpoint.sh CHECKPOINT_ID EXPECTED_GENERATION EXPECTED_PREDECESSOR RECORD_FILE" >&2
  exit 2
}

script_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
operation_mode=false
if [[ ${1:-} == --operation ]]; then
  [[ $# -eq 5 ]] || usage
  operation_mode=true
  checkpoint_id=$2
  operation_id=$3
  operation=$4
  record_file=$5
  expected_generation=0
  expected_predecessor=null
else
  [[ $# -eq 4 ]] || usage
  checkpoint_id=$1
  expected_generation=$2
  expected_predecessor=$3
  record_file=$4
fi

[[ $checkpoint_id =~ ^[A-Za-z0-9._-]+$ ]] || { echo "invalid checkpoint id" >&2; exit 2; }
[[ $expected_generation =~ ^(0|[1-9][0-9]*)$ ]] || usage
[[ -f $record_file ]] || usage
for dependency in git jq flock sha256sum sed od tr sync mktemp cp mv chmod grep wc tail head cut node realpath; do
  command -v "$dependency" >/dev/null 2>&1 || { echo "missing checkpoint runtime dependency: $dependency" >&2; exit 2; }
done
TERM=dumb sync --help 2>&1 | grep -q -- ' -f' || { echo "checkpoint runtime requires sync -f support" >&2; exit 2; }

worktree_root=$(git rev-parse --show-toplevel)
record_absolute=$(realpath -- "$record_file")
case "$record_absolute" in
  "$worktree_root"|"$worktree_root"/*)
    echo "record file must be outside the worktree" >&2
    exit 2
    ;;
esac
git_common_dir=$(git -C "$worktree_root" rev-parse --path-format=absolute --git-common-dir)
checkpoint_dir="$git_common_dir/development-system/checkpoints"
target="$checkpoint_dir/$checkpoint_id.latest"
lock="$checkpoint_dir/$checkpoint_id.lock"
umask 077
mkdir -p "$checkpoint_dir"
chmod 700 "$checkpoint_dir"
exec {lock_fd}>"$lock"
if ! flock -x -w 30 "$lock_fd"; then
  echo "checkpoint lock timed out; retry after the active writer completes" >&2
  exit 3
fi

# Reconcile an interrupted operation before either public entry point advances.
node "$script_root/checkpoint-operations.mjs" recover "$target"
if $operation_mode; then
  operation_result=$(node "$script_root/checkpoint-operations.mjs" prepare "$target" "$operation_id" "$operation" "$record_file")
  if [[ $(jq -r '.replayed' <<<"$operation_result") == true ]]; then
    jq -c '.receipt' <<<"$operation_result"
    exit 0
  fi
  expected_generation=$(jq -r '.record.generation' <<<"$operation_result")
  expected_predecessor=$(jq -r '.record.predecessor_sha256' <<<"$operation_result")
fi

candidate=$(mktemp "$checkpoint_dir/.$checkpoint_id.candidate.XXXXXX")
cleanup() {
  [[ -z $candidate ]] || rm -f -- "$candidate"
}
trap cleanup EXIT
if $operation_mode; then
  printf 'checkpoint-v1 %s\n' "$(jq -c '.record' <<<"$operation_result")" >"$candidate"
else
  cp -- "$record_file" "$candidate"
fi
chmod 600 "$candidate"
sync -f "$candidate"

[[ $(wc -l < "$candidate") -eq 1 && $(tail -c 1 "$candidate" | od -An -t u1 | tr -d ' ') == 10 ]] || { echo "record must contain exactly one newline-terminated line" >&2; exit 2; }
cmp -s "$candidate" <(tr -d '\000' <"$candidate") || { echo "record must not contain NUL bytes" >&2; exit 2; }
[[ $(head -c 14 "$candidate") == "checkpoint-v1 " ]] || { echo "record must start with checkpoint-v1" >&2; exit 2; }

current_snapshot=$(node "$script_root/checkpoint-operations.mjs" snapshot "$target")
current_head=$(jq -r '.head_oid' <<<"$current_snapshot")
current_tracked=$(jq -r '.tracked_sha256' <<<"$current_snapshot")
current_untracked=$(jq -r '.untracked_sha256' <<<"$current_snapshot")

if ! tail -c +15 "$candidate" | jq -e --argjson generation "$expected_generation" --arg predecessor "$expected_predecessor" --arg current_head "$current_head" --arg current_tracked "$current_tracked" --arg current_untracked "$current_untracked" -f "$script_root/checkpoint-record.jq" >/dev/null; then
  echo "checkpoint record failed schema, snapshot, or state validation" >&2
  exit 2
fi
proposed_baseline=$(tail -c +15 "$candidate" | jq -er '.baseline_oid')

if [[ -e $target ]]; then
  current_generation=$(sed -n 's/^checkpoint-v1 //p' "$target" | jq -er '.generation')
  current_baseline=$(sed -n 's/^checkpoint-v1 //p' "$target" | jq -er '.baseline_oid')
  current_predecessor=$(sha256sum "$target" | cut -d ' ' -f 1)
  [[ $expected_generation -eq $((current_generation + 1)) ]] || { echo "stale checkpoint generation" >&2; exit 3; }
  [[ $expected_predecessor == "$current_predecessor" ]] || { echo "stale checkpoint predecessor" >&2; exit 3; }
  [[ $proposed_baseline == "$current_baseline" ]] || { echo "checkpoint baseline does not match predecessor" >&2; exit 3; }
  current_record=$(sed -n 's/^checkpoint-v1 //p' "$target")
  proposed_record=$(tail -c +15 "$candidate")
  current_ci=$(jq -c '.ci.runs' <<<"$current_record")
  proposed_ci=$(jq -c '.ci.runs' <<<"$proposed_record")
  jq -en --argjson current "$current_ci" --argjson proposed "$proposed_ci" \
    '$proposed[0:($current | length)] == $current' >/dev/null || {
      echo "checkpoint CI observations do not preserve predecessor history" >&2
      exit 3
    }
  jq -en --argjson current "$current_record" --argjson proposed "$proposed_record" \
    '($proposed.ci.recoveries // [])[0:(($current.ci.recoveries // []) | length)] == ($current.ci.recoveries // [])' >/dev/null || {
      echo "checkpoint CI diagnosis history does not preserve predecessor" >&2
      exit 3
    }
  jq -en --argjson current "$current_record" --argjson proposed "$proposed_record" '
    def passing($action):
      $proposed.state == "passing-awaiting-gates-or-review" and
      $proposed.next_action == $action;
    def remediation_result:
      $proposed.state == "failing" or
      (passing("lightweight-review") and
       $proposed.gates.lightweight_review_receipt == null and
       $proposed.gates.fast_gate_receipt == null);
    def gate_failure($kind):
      $proposed.state == "failing" and
      $proposed.snapshot.head_oid == $current.snapshot.head_oid and
      $proposed.test.outcome == "fail" and
      $proposed.test.failure_kind == $kind;
    if ($proposed.next_action != "complete" or $current.next_action == "complete") and
       (($proposed | del(.generation, .predecessor_sha256, .next_action, .ci)) == ($current | del(.generation, .predecessor_sha256, .next_action, .ci))) and
       (($proposed.ci.runs | length) > ($current.ci.runs | length) or
        ($current.next_action == "enter-ci-recovery" and
         (($proposed.ci.recoveries // []) | length) > (($current.ci.recoveries // []) | length))) and
       (($proposed.ci.recoveries // [])[0:(($current.ci.recoveries // []) | length)] == ($current.ci.recoveries // [])) then true
    elif $proposed.state == "awaiting-causal-edit" then
      $current.state == "pushed-or-delivery-mode-equivalent" and
      ($current.next_action | IN("register-exact-sha-ci-monitor", "monitor-exact-sha-ci", "terminal-review", "complete")) and
      $current.gates.exact_identity_verification_receipt.outcome == "pass" and
      $proposed.snapshot == $current.snapshot and
      $proposed.ci.runs == $current.ci.runs
    elif ($current.next_action | test("^(causal-edit|rewrite-invalid-test): \\S")) then
      remediation_result
    elif $current.next_action == "lightweight-review" then
      gate_failure("lightweight-review") or
      ((passing("fast-gate") or passing("commit-through-pre-commit-hook")) and
       $proposed.test == $current.test and
       ($proposed.gates.lightweight_review_receipt | type == "string") and
       $proposed.gates.fast_gate_receipt == null)
    elif $current.next_action == "commit-through-pre-commit-hook" then
      gate_failure("pre-commit-hook") or
      ($proposed.state == "committed" and
       $proposed.next_action == "verify-exact-commit" and
       $proposed.test == $current.test and
       $proposed.gates.lightweight_review_receipt == $current.gates.lightweight_review_receipt and
       ($proposed.gates.fast_gate_receipt | type == "string"))
    elif $current.next_action == "fast-gate" then
      gate_failure("fast-gate") or
      (passing("commit-or-record-local-snapshot") and
      $proposed.test == $current.test and
      $proposed.gates.lightweight_review_receipt == $current.gates.lightweight_review_receipt and
      ($proposed.gates.fast_gate_receipt | type == "string"))
    elif $current.next_action == "commit-or-record-local-snapshot" then
      $proposed.test == $current.test and
      $proposed.gates.lightweight_review_receipt == $current.gates.lightweight_review_receipt and
      $proposed.gates.fast_gate_receipt == $current.gates.fast_gate_receipt and
      (($proposed.state == "committed" and $proposed.next_action == "verify-exact-commit") or
       ($proposed.state == "pushed-or-delivery-mode-equivalent" and
        $proposed.delivery.mode == "local-only" and $proposed.next_action == "terminal-review"))
    elif $current.next_action == "verify-exact-commit" then
      $proposed.state == "committed" and
      ($proposed.next_action | IN("repair-exact-identity-verification", "push", "record-local-delivery"))
    elif $current.next_action == "repair-exact-identity-verification" then
      $proposed.state == "committed" and $proposed.next_action == "verify-exact-commit"
    elif $current.next_action == "push" then
      $proposed.state == "pushed-or-delivery-mode-equivalent" and
      $proposed.next_action == "register-exact-sha-ci-monitor"
    elif $current.next_action == "record-local-delivery" then
      $proposed.state == "pushed-or-delivery-mode-equivalent" and
      $proposed.delivery.mode == "local-only" and $proposed.next_action == "terminal-review"
    elif ($current.next_action | IN("register-exact-sha-ci-monitor", "monitor-exact-sha-ci", "enter-ci-recovery")) then
      (($proposed.state == "pushed-or-delivery-mode-equivalent" and
        ($proposed.next_action | IN("register-exact-sha-ci-monitor", "monitor-exact-sha-ci", "enter-ci-recovery", "terminal-review"))) or
       remediation_result)
    elif $current.next_action == "terminal-review" then
      gate_failure("terminal-review") or
      (($proposed.next_action | IN("monitor-exact-sha-ci", "enter-ci-recovery", "terminal-review")) and
       ($proposed.ci.runs | length) > ($current.ci.runs | length) and
       ($proposed | del(.generation, .predecessor_sha256, .next_action, .ci)) ==
       ($current | del(.generation, .predecessor_sha256, .next_action, .ci))) or
      ($proposed.next_action == "complete" and
       ($proposed | del(.generation, .predecessor_sha256, .next_action)) ==
       ($current | del(.generation, .predecessor_sha256, .next_action)))
    elif $current.next_action == "complete" then false
    else
      ($proposed | del(.generation, .predecessor_sha256, .next_action)) ==
      ($current | del(.generation, .predecessor_sha256, .next_action)) and
      ($proposed.next_action | test("^(causal-edit|rewrite-invalid-test): \\S"))
    end
  ' >/dev/null || {
    echo "successor does not perform predecessor next_action" >&2
    exit 3
  }
else
  [[ $expected_generation -eq 0 && $expected_predecessor == null ]] || { echo "missing checkpoint predecessor" >&2; exit 3; }
fi

final_snapshot=$(node "$script_root/checkpoint-operations.mjs" snapshot "$target")
final_head=$(jq -r '.head_oid' <<<"$final_snapshot")
final_tracked=$(jq -r '.tracked_sha256' <<<"$final_snapshot")
final_untracked=$(jq -r '.untracked_sha256' <<<"$final_snapshot")

if [[ $final_head != "$current_head" || $final_tracked != "$current_tracked" || $final_untracked != "$current_untracked" ]]; then
  echo "worktree changed while publishing checkpoint" >&2
  exit 3
fi

if $operation_mode; then
  printf '%s\n' "$operation_result" | node "$script_root/checkpoint-operations.mjs" stage "$target" "$candidate"
fi
mv -f -- "$candidate" "$target"
candidate=
sync -f "$checkpoint_dir"
if $operation_mode; then
  node "$script_root/checkpoint-operations.mjs" recover "$target"
  node "$script_root/checkpoint-operations.mjs" read "$target" "$operation_id"
fi
cleanup
trap - EXIT
