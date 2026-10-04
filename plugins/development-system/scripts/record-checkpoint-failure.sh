#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo 'usage: record-checkpoint-failure.sh CHECKPOINT_ID EXPECTED_GENERATION EXPECTED_PREDECESSOR FAILURE_KIND COMMAND RECEIPT_FILE CAUSAL_REPAIR' >&2
  exit 2
}

[[ $# -eq 7 ]] || usage
checkpoint_id=$1
expected_generation=$2
expected_predecessor=$3
failure_kind=$4
failed_command=$5
receipt_file=$6
causal_repair=$7
[[ $checkpoint_id =~ ^[A-Za-z0-9._-]+$ ]] || usage
[[ $expected_generation =~ ^[1-9][0-9]*$ ]] || usage
[[ $expected_predecessor =~ ^[0-9a-f]{64}$ ]] || usage
[[ $failed_command =~ [^[:space:]] && $causal_repair =~ [^[:space:]] ]] || usage
case "$failure_kind" in
  pre-commit-hook) predecessor_action=commit-through-pre-commit-hook ;;
  lightweight-review) predecessor_action=lightweight-review ;;
  *) usage ;;
esac
for dependency in git jq sha256sum realpath mktemp node; do
  command -v "$dependency" >/dev/null 2>&1 || { echo "missing checkpoint runtime dependency: $dependency" >&2; exit 2; }
done
[[ -f $receipt_file && -r $receipt_file && -s $receipt_file ]] || { echo 'failure evidence must be a readable, nonempty file' >&2; exit 2; }
worktree_root=$(git rev-parse --show-toplevel)
receipt_absolute=$(realpath -- "$receipt_file")
case "$receipt_absolute" in
  "$worktree_root"|"$worktree_root"/*)
    echo 'failure evidence must be outside the worktree' >&2
    exit 2
    ;;
esac
git_common_dir=$(git rev-parse --path-format=absolute --git-common-dir)
target="$git_common_dir/development-system/checkpoints/$checkpoint_id.latest"
[[ -f $target ]] || { echo 'missing checkpoint predecessor' >&2; exit 3; }
umask 077
scratch=$(mktemp -d)
trap 'rm -rf -- "$scratch"' EXIT
# Read one immutable copy. The writer checks the same digest under its lock.
cp -- "$target" "$scratch/predecessor"
[[ $(sha256sum "$scratch/predecessor" | cut -d ' ' -f 1) == "$expected_predecessor" ]] || { echo 'stale checkpoint predecessor' >&2; exit 3; }
[[ $(head -c 14 "$scratch/predecessor") == 'checkpoint-v1 ' ]] || { echo 'malformed checkpoint predecessor' >&2; exit 3; }
current_record=$(tail -c +15 "$scratch/predecessor")
head_oid=$(git rev-parse HEAD)
jq -e --argjson generation "$expected_generation" --arg action "$predecessor_action" --arg head "$head_oid" '
  def exact_keys($keys): (keys | sort) == ($keys | sort);
  def nonblank: type == "string" and test("\\S");
  def oid: type == "string" and test("^[0-9a-f]{40}([0-9a-f]{24})?$");
  def sha256: type == "string" and test("^[0-9a-f]{64}$");
  exact_keys(["generation", "predecessor_sha256", "baseline_oid", "snapshot", "state", "test", "gates", "delivery", "ci", "next_action"]) and
  .generation == ($generation - 1) and
  .generation >= 1 and (.predecessor_sha256 | sha256) and
  (.baseline_oid | oid) and
  (.snapshot | exact_keys(["head_oid", "tracked_sha256", "untracked_sha256"]) and
    (.head_oid | oid) and (.tracked_sha256 | sha256) and (.untracked_sha256 | sha256)) and
  (.test | exact_keys(["command", "receipt_ref", "outcome", "failure_kind"]) and
    (.command | nonblank) and (.receipt_ref | nonblank) and .outcome == "pass" and
    (.failure_kind == null or (.failure_kind | type == "string"))) and
  (.gates | exact_keys(["lightweight_review_receipt", "fast_gate_receipt", "exact_identity_verification_receipt"]) and
    .fast_gate_receipt == null and .exact_identity_verification_receipt == null and
    (if $action == "lightweight-review" then .lightweight_review_receipt == null
     else (.lightweight_review_receipt | nonblank) end)) and
  .delivery == null and
  (.ci | exact_keys(["runs", "terminal_success_run_id"]) and .terminal_success_run_id == null and
    (.runs | type == "array") and all(.runs[];
      exact_keys(["provider", "run_id", "commit_oid", "status"]) and
      (.provider | nonblank) and (.run_id | nonblank) and (.commit_oid | oid) and
      (.status | IN("queued", "running", "success", "failure")))) and
  .state == "passing-awaiting-gates-or-review" and
  .next_action == $action and .snapshot.head_oid == $head
' <<<"$current_record" >/dev/null || {
  echo 'failure recovery requires the expected pending gate and unchanged HEAD' >&2
  exit 3
}
tracked_sha256=$(git -C "$worktree_root" diff --binary --full-index HEAD -- | sha256sum | cut -d ' ' -f 1)
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
  printf '%s\0%s\0%s\n' "$mode" "$path" "$oid" >>"$scratch/untracked"
done < <(git -C "$worktree_root" ls-files --full-name --others --exclude-standard -z)
touch "$scratch/untracked"
untracked_sha256=$(sha256sum "$scratch/untracked" | cut -d ' ' -f 1)
receipt_digest=$(sha256sum "$receipt_absolute" | cut -d ' ' -f 1)
record=$(jq -c --argjson generation "$expected_generation" --arg predecessor "$expected_predecessor" \
  --arg head "$head_oid" --arg tracked "$tracked_sha256" --arg untracked "$untracked_sha256" \
  --arg kind "$failure_kind" --arg command "$failed_command" \
  --arg receipt "$receipt_absolute sha256=$receipt_digest" --arg repair "$causal_repair" '
  .generation = $generation | .predecessor_sha256 = $predecessor |
  .snapshot = {head_oid:$head,tracked_sha256:$tracked,untracked_sha256:$untracked} |
  .state = "failing" |
  .test = {command:$command,receipt_ref:$receipt,outcome:"fail",failure_kind:$kind} |
  .gates |= map_values(null) | .delivery = null |
  .next_action = ("causal-edit: " + $repair)
' <<<"$current_record")
printf 'checkpoint-v1 %s\n' "$record" >"$scratch/proposal"
script_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
"$script_root/write-local-checkpoint.sh" "$checkpoint_id" "$expected_generation" "$expected_predecessor" "$scratch/proposal"
printf 'Recorded %s failure at checkpoint %s generation %s; only the causal repair and fresh testing may follow.\n' "$failure_kind" "$checkpoint_id" "$expected_generation"
