#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "usage: write-local-checkpoint.sh CHECKPOINT_ID EXPECTED_GENERATION EXPECTED_PREDECESSOR RECORD_FILE" >&2
  exit 2
}

[[ $# -eq 4 ]] || usage
checkpoint_id=$1
expected_generation=$2
expected_predecessor=$3
record_file=$4

[[ $checkpoint_id =~ ^[A-Za-z0-9._-]+$ ]] || { echo "invalid checkpoint id" >&2; exit 2; }
[[ $expected_generation =~ ^(0|[1-9][0-9]*)$ ]] || usage
[[ -f $record_file ]] || usage
for dependency in git jq flock sha256sum sed od tr sync mktemp cp mv chmod grep wc tail head cut node realpath; do
  command -v "$dependency" >/dev/null 2>&1 || { echo "missing checkpoint runtime dependency: $dependency" >&2; exit 2; }
done
sync --help 2>&1 | grep -q -- ' -f' || { echo "checkpoint runtime requires sync -f support" >&2; exit 2; }

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

candidate=$(mktemp "$checkpoint_dir/.$checkpoint_id.candidate.XXXXXX")
untracked_stream=
final_untracked_stream=
cleanup() {
  [[ -z $candidate ]] || rm -f -- "$candidate"
  [[ -z $untracked_stream ]] || rm -f -- "$untracked_stream"
  [[ -z $final_untracked_stream ]] || rm -f -- "$final_untracked_stream"
}
trap cleanup EXIT
cp -- "$record_file" "$candidate"
chmod 600 "$candidate"
sync -f "$candidate"

[[ $(wc -l < "$candidate") -eq 1 && $(tail -c 1 "$candidate" | od -An -t u1 | tr -d ' ') == 10 ]] || { echo "record must contain exactly one newline-terminated line" >&2; exit 2; }
cmp -s "$candidate" <(tr -d '\000' <"$candidate") || { echo "record must not contain NUL bytes" >&2; exit 2; }
[[ $(head -c 14 "$candidate") == "checkpoint-v1 " ]] || { echo "record must start with checkpoint-v1" >&2; exit 2; }

current_head=$(git -C "$worktree_root" rev-parse HEAD)
current_tracked=$(git -C "$worktree_root" diff --binary --full-index HEAD -- | sha256sum | cut -d ' ' -f 1)
untracked_stream=$(mktemp)
while IFS= read -r -d '' path; do
  absolute_path="$worktree_root/$path"
  if [[ -L $absolute_path ]]; then mode=120000
  elif [[ -f $absolute_path && -x $absolute_path ]]; then mode=100755
  elif [[ -f $absolute_path ]]; then mode=100644
  else echo "unsupported untracked file type: $path" >&2; exit 2
  fi
  if [[ -L $absolute_path ]]; then
    oid=$(node -e 'process.stdout.write(require("node:fs").readlinkSync(process.argv[1], {encoding: "buffer"}))' "$absolute_path" | git -C "$worktree_root" hash-object --stdin)
  else
    oid=$(git -C "$worktree_root" hash-object -- "$path")
  fi
  printf '%s\0%s\0%s\n' "$mode" "$path" "$oid" >>"$untracked_stream"
done < <(git -C "$worktree_root" ls-files --full-name --others --exclude-standard -z)
current_untracked=$(sha256sum "$untracked_stream" | cut -d ' ' -f 1)

if ! tail -c +15 "$candidate" | jq -e --argjson generation "$expected_generation" --arg predecessor "$expected_predecessor" --arg current_head "$current_head" --arg current_tracked "$current_tracked" --arg current_untracked "$current_untracked" '
  def exact_keys($expected): (keys | sort) == ($expected | sort);
  def string_or_null: type == "string" or . == null;
  def nonblank: type == "string" and test("\\S");
  def nonblank_or_null: . == null or nonblank;
  def oid: type == "string" and test("^[0-9a-f]{40}([0-9a-f]{24})?$");
  def sha256: type == "string" and test("^[0-9a-f]{64}$");
  . as $record |
  exact_keys(["generation", "predecessor_sha256", "baseline_oid", "snapshot", "state", "test", "gates", "delivery", "ci", "next_action"]) and
  .generation == $generation and
  (if $generation == 0 then .predecessor_sha256 == null else .predecessor_sha256 == $predecessor end) and
  (.baseline_oid | oid) and
  (.snapshot | exact_keys(["head_oid", "tracked_sha256", "untracked_sha256"]) and (.head_oid | oid) and (.tracked_sha256 | sha256) and (.untracked_sha256 | sha256)) and
  .snapshot.head_oid == $current_head and .snapshot.tracked_sha256 == $current_tracked and .snapshot.untracked_sha256 == $current_untracked and
  (.state | IN("failing", "passing-awaiting-gates-or-review", "committed", "pushed-or-delivery-mode-equivalent")) and
  (.test == null or (.test | exact_keys(["command", "receipt_ref", "outcome", "failure_kind"]) and (.command | nonblank) and (.receipt_ref | nonblank) and (.outcome | IN("pass", "fail")) and (.failure_kind | string_or_null))) and
  (.gates | exact_keys(["lightweight_review_receipt", "fast_gate_receipt", "exact_identity_verification_receipt"]) and
    (.lightweight_review_receipt | nonblank_or_null) and (.fast_gate_receipt | nonblank_or_null) and
    (.exact_identity_verification_receipt == null or
      (.exact_identity_verification_receipt | exact_keys(["receipt_ref", "outcome"]) and
       (.receipt_ref | nonblank) and (.outcome | IN("pass", "fail"))))) and
  (.delivery == null or (.delivery | exact_keys(["mode", "commit_oid", "pushed_oid", "local_snapshot"]) and (.mode | IN("local-only", "direct-to-trunk", "pull-request")) and (.commit_oid | . == null or oid) and (.pushed_oid | . == null or oid) and (.local_snapshot | nonblank_or_null))) and
  (.ci | exact_keys(["runs", "terminal_success_run_id"]) and (.runs | type == "array") and all(.runs[]; exact_keys(["provider", "run_id", "commit_oid", "status"]) and (.provider | nonblank) and (.run_id | nonblank) and (.commit_oid | oid) and (.status | IN("queued", "running", "success", "failure"))) and (.terminal_success_run_id | nonblank_or_null)) and
  (.next_action | nonblank) and
  ($generation != 0 or .state == "pushed-or-delivery-mode-equivalent") and
  (.ci.terminal_success_run_id == null or
   ((.ci.runs | length) > 0 and .ci.runs[-1].run_id == .ci.terminal_success_run_id and
    .ci.runs[-1].status == "success" and .ci.runs[-1].commit_oid == $record.delivery.pushed_oid)) and
  (if .state == "failing" then
     .test != null and .delivery == null and all(.gates[]; . == null) and
     (if .test.outcome == "pass" then
        .test.failure_kind == "invalid-test" and (.next_action | test("^rewrite-invalid-test: \\S"))
      else
        (.test.failure_kind | nonblank) and (.next_action | test("^causal-edit: \\S"))
      end)
   elif .state == "passing-awaiting-gates-or-review" then
     .test != null and .test.outcome == "pass" and .delivery == null and .gates.exact_identity_verification_receipt == null and
     (if .gates.lightweight_review_receipt == null then
        .gates.fast_gate_receipt == null and .next_action == "lightweight-review"
      elif .gates.fast_gate_receipt == null then
        .next_action == "fast-gate"
      else .next_action == "commit-or-record-local-snapshot" end)
   elif .state == "committed" then
     .test != null and .test.outcome == "pass" and .delivery != null and .delivery.commit_oid == .snapshot.head_oid and
     (.gates.lightweight_review_receipt | type == "string") and
     (.gates.fast_gate_receipt | type == "string") and
     ((.gates.exact_identity_verification_receipt == null and .next_action == "verify-exact-commit") or
      (.gates.exact_identity_verification_receipt.outcome == "pass" and
       (if .delivery.mode == "local-only" then .next_action == "record-local-delivery" else .next_action == "push" end)) or
      (.gates.exact_identity_verification_receipt.outcome == "fail" and .next_action == "repair-exact-identity-verification"))
   elif .state == "pushed-or-delivery-mode-equivalent" and .generation == 0 then
     .baseline_oid == .snapshot.head_oid and
     .snapshot.tracked_sha256 == "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855" and
     .snapshot.untracked_sha256 == "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855" and
     .test == null and all(.gates[]; . == null) and .delivery != null and
     (.next_action | test("^causal-edit: \\S")) and
     (if .delivery.mode == "local-only" then
        .delivery.pushed_oid == null and (.delivery.local_snapshot | type == "string") and
        (.ci.runs | length) == 0 and .ci.terminal_success_run_id == null
      else .delivery.local_snapshot == null and .delivery.pushed_oid == .snapshot.head_oid end)
   else
     .test != null and .test.outcome == "pass" and .delivery != null and
     (.gates.lightweight_review_receipt | type == "string") and
     (.gates.fast_gate_receipt | type == "string") and
     .gates.exact_identity_verification_receipt.outcome == "pass" and
     (if .delivery.mode == "local-only" then
        .delivery.pushed_oid == null and (.delivery.local_snapshot | type == "string") and
        (.ci.runs | length) == 0 and .ci.terminal_success_run_id == null and .next_action == "terminal-review"
      else
        .delivery.local_snapshot == null and .delivery.commit_oid == .snapshot.head_oid and
        .delivery.pushed_oid == .snapshot.head_oid and
        ((.ci.runs | map(select(.commit_oid == $record.delivery.pushed_oid))) as $current_runs |
         if ($current_runs | length) == 0 then .next_action == "register-exact-sha-ci-monitor"
         elif .ci.terminal_success_run_id != null then .next_action == "terminal-review"
         elif any($current_runs[]; .status == "failure") then .next_action == "enter-ci-recovery"
         else .next_action == "monitor-exact-sha-ci" end)
      end)
   end)
  ' >/dev/null; then
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
  jq -en --argjson current "$current_record" --argjson proposed "$proposed_record" '
    def passing($action):
      $proposed.state == "passing-awaiting-gates-or-review" and
      $proposed.next_action == $action;
    def remediation_result:
      $proposed.state == "failing" or
      (passing("lightweight-review") and
       $proposed.gates.lightweight_review_receipt == null and
       $proposed.gates.fast_gate_receipt == null);
    if ($current.next_action | test("^(causal-edit|rewrite-invalid-test): \\S")) then
      remediation_result
    elif $current.next_action == "lightweight-review" then
      passing("fast-gate") and
      $proposed.test == $current.test and
      ($proposed.gates.lightweight_review_receipt | type == "string") and
      $proposed.gates.fast_gate_receipt == null
    elif $current.next_action == "fast-gate" then
      passing("commit-or-record-local-snapshot") and
      $proposed.test == $current.test and
      $proposed.gates.lightweight_review_receipt == $current.gates.lightweight_review_receipt and
      ($proposed.gates.fast_gate_receipt | type == "string")
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
      remediation_result
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

final_head=$(git -C "$worktree_root" rev-parse HEAD)
final_tracked=$(git -C "$worktree_root" diff --binary --full-index HEAD -- | sha256sum | cut -d ' ' -f 1)
final_untracked_stream=$(mktemp)
while IFS= read -r -d '' path; do
  absolute_path="$worktree_root/$path"
  if [[ -L $absolute_path ]]; then mode=120000
  elif [[ -f $absolute_path && -x $absolute_path ]]; then mode=100755
  elif [[ -f $absolute_path ]]; then mode=100644
  else echo "unsupported untracked file type: $path" >&2; exit 2
  fi
  if [[ -L $absolute_path ]]; then
    oid=$(node -e 'process.stdout.write(require("node:fs").readlinkSync(process.argv[1], {encoding: "buffer"}))' "$absolute_path" | git -C "$worktree_root" hash-object --stdin)
  else
    oid=$(git -C "$worktree_root" hash-object -- "$path")
  fi
  printf '%s\0%s\0%s\n' "$mode" "$path" "$oid" >>"$final_untracked_stream"
done < <(git -C "$worktree_root" ls-files --full-name --others --exclude-standard -z)
final_untracked=$(sha256sum "$final_untracked_stream" | cut -d ' ' -f 1)
if [[ $final_head != "$current_head" || $final_tracked != "$current_tracked" || $final_untracked != "$current_untracked" ]]; then
  echo "worktree changed while publishing checkpoint" >&2
  exit 3
fi

mv -f -- "$candidate" "$target"
candidate=
sync -f "$checkpoint_dir"
cleanup
trap - EXIT
