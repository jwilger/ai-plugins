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
for dependency in git jq flock sha256sum sed od tr sync mktemp cp mv chmod grep wc tail head cut node; do
  command -v "$dependency" >/dev/null 2>&1 || { echo "missing checkpoint runtime dependency: $dependency" >&2; exit 2; }
done
sync --help 2>&1 | grep -q -- ' -f' || { echo "checkpoint runtime requires sync -f support" >&2; exit 2; }
[[ $(wc -l < "$record_file") -eq 1 && $(tail -c 1 "$record_file" | od -An -t u1 | tr -d ' ') == 10 ]] || { echo "record must contain exactly one newline-terminated line" >&2; exit 2; }
cmp -s "$record_file" <(tr -d '\000' <"$record_file") || { echo "record must not contain NUL bytes" >&2; exit 2; }

[[ $(head -c 14 "$record_file") == "checkpoint-v1 " ]] || { echo "record must start with checkpoint-v1" >&2; exit 2; }

git_common_dir=$(git rev-parse --path-format=absolute --git-common-dir)
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

current_head=$(git rev-parse HEAD)
current_tracked=$(git diff --binary --full-index HEAD -- | sha256sum | cut -d ' ' -f 1)
untracked_stream=$(mktemp)
trap 'rm -f -- "$untracked_stream"' EXIT
while IFS= read -r -d '' path; do
  if [[ -L $path ]]; then mode=120000
  elif [[ -f $path && -x $path ]]; then mode=100755
  elif [[ -f $path ]]; then mode=100644
  else echo "unsupported untracked file type: $path" >&2; exit 2
  fi
  if [[ -L $path ]]; then
    oid=$(node -e 'process.stdout.write(require("node:fs").readlinkSync(process.argv[1], {encoding: "buffer"}))' "$path" | git hash-object --stdin)
  else
    oid=$(git hash-object -- "$path")
  fi
  printf '%s\0%s\0%s\n' "$mode" "$path" "$oid" >>"$untracked_stream"
done < <(git ls-files --others --exclude-standard -z)
current_untracked=$(sha256sum "$untracked_stream" | cut -d ' ' -f 1)

if ! tail -c +15 "$record_file" | jq -e --argjson generation "$expected_generation" --arg predecessor "$expected_predecessor" --arg current_head "$current_head" --arg current_tracked "$current_tracked" --arg current_untracked "$current_untracked" '
  def exact_keys($expected): (keys | sort) == ($expected | sort);
  def string_or_null: type == "string" or . == null;
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
  (.test == null or (.test | exact_keys(["command", "receipt_ref", "outcome", "failure_kind"]) and (.command | type == "string") and (.receipt_ref | type == "string") and (.outcome | IN("pass", "fail")) and (.failure_kind | string_or_null))) and
  (.gates | exact_keys(["lightweight_review_receipt", "fast_gate_receipt", "exact_identity_verification_receipt"]) and
    (.lightweight_review_receipt | string_or_null) and (.fast_gate_receipt | string_or_null) and
    (.exact_identity_verification_receipt == null or
      (.exact_identity_verification_receipt | exact_keys(["receipt_ref", "outcome"]) and
       (.receipt_ref | type == "string") and (.outcome | IN("pass", "fail"))))) and
  (.delivery == null or (.delivery | exact_keys(["mode", "commit_oid", "pushed_oid", "local_snapshot"]) and (.mode | IN("local-only", "direct-to-trunk", "pull-request")) and (.commit_oid | . == null or oid) and (.pushed_oid | . == null or oid) and (.local_snapshot | string_or_null))) and
  (.ci | exact_keys(["runs", "terminal_success_run_id"]) and (.runs | type == "array") and all(.runs[]; exact_keys(["provider", "run_id", "commit_oid", "status"]) and (.provider | type == "string") and (.run_id | type == "string") and (.commit_oid | oid) and (.status | IN("queued", "running", "success", "failure"))) and (.terminal_success_run_id | string_or_null)) and
  (.next_action | type == "string") and
  ($generation != 0 or .state == "pushed-or-delivery-mode-equivalent") and
  (.ci.terminal_success_run_id == null or any(.ci.runs[]; .run_id == $record.ci.terminal_success_run_id and .status == "success" and .commit_oid == $record.delivery.pushed_oid)) and
  (if .state == "failing" then
     .test != null and .test.outcome == "fail" and .delivery == null and all(.gates[]; . == null) and
     .next_action == "causal-edit"
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
        all(.ci.runs[]; .commit_oid == $record.delivery.pushed_oid) and
        (if (.ci.runs | length) == 0 then .next_action == "register-exact-sha-ci-monitor"
         elif any(.ci.runs[]; .status == "failure") then .next_action == "enter-ci-recovery"
         elif .ci.terminal_success_run_id != null then .next_action == "terminal-review"
         else .next_action == "monitor-exact-sha-ci" end)
      end)
   end)
  ' >/dev/null; then
  echo "checkpoint record failed schema, snapshot, or state validation" >&2
  exit 2
fi
proposed_baseline=$(tail -c +15 "$record_file" | jq -er '.baseline_oid')

if [[ -e $target ]]; then
  current_generation=$(sed -n 's/^checkpoint-v1 //p' "$target" | jq -er '.generation')
  current_baseline=$(sed -n 's/^checkpoint-v1 //p' "$target" | jq -er '.baseline_oid')
  current_predecessor=$(sha256sum "$target" | cut -d ' ' -f 1)
  [[ $expected_generation -eq $((current_generation + 1)) ]] || { echo "stale checkpoint generation" >&2; exit 3; }
  [[ $expected_predecessor == "$current_predecessor" ]] || { echo "stale checkpoint predecessor" >&2; exit 3; }
  [[ $proposed_baseline == "$current_baseline" ]] || { echo "checkpoint baseline does not match predecessor" >&2; exit 3; }
else
  [[ $expected_generation -eq 0 && $expected_predecessor == null ]] || { echo "missing checkpoint predecessor" >&2; exit 3; }
fi

temporary=$(mktemp "$checkpoint_dir/.$checkpoint_id.tmp.XXXXXX")
trap 'rm -f -- "$temporary" "$untracked_stream"' EXIT
cp -- "$record_file" "$temporary"
chmod 600 "$temporary"
sync -f "$temporary"
mv -f -- "$temporary" "$target"
sync -f "$checkpoint_dir"
rm -f -- "$untracked_stream"
trap - EXIT
