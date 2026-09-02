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
[[ $(wc -l < "$record_file") -eq 1 ]] || { echo "record must contain exactly one newline-terminated line" >&2; exit 2; }

record=$(<"$record_file")
[[ $record == checkpoint-v1\ * ]] || { echo "record must start with checkpoint-v1" >&2; exit 2; }
json=${record#checkpoint-v1 }
jq -e --argjson generation "$expected_generation" --arg predecessor "$expected_predecessor" '
  .generation == $generation and
  (if $generation == 0 then .predecessor_sha256 == null else .predecessor_sha256 == $predecessor end)
' <<<"$json" >/dev/null

git_common_dir=$(git rev-parse --path-format=absolute --git-common-dir)
checkpoint_dir="$git_common_dir/development-system/checkpoints"
target="$checkpoint_dir/$checkpoint_id.latest"
lock="$checkpoint_dir/$checkpoint_id.lock"
umask 077
mkdir -p "$checkpoint_dir"
chmod 700 "$checkpoint_dir"
exec {lock_fd}>"$lock"
flock -x "$lock_fd"

if [[ -e $target ]]; then
  current_generation=$(sed -n 's/^checkpoint-v1 //p' "$target" | jq -er '.generation')
  current_predecessor=$(sha256sum "$target" | cut -d ' ' -f 1)
  [[ $expected_generation -eq $((current_generation + 1)) ]] || { echo "stale checkpoint generation" >&2; exit 3; }
  [[ $expected_predecessor == "$current_predecessor" ]] || { echo "stale checkpoint predecessor" >&2; exit 3; }
else
  [[ $expected_generation -eq 0 && $expected_predecessor == null ]] || { echo "missing checkpoint predecessor" >&2; exit 3; }
fi

temporary=$(mktemp "$checkpoint_dir/.$checkpoint_id.tmp.XXXXXX")
trap 'rm -f -- "$temporary"' EXIT
cp -- "$record_file" "$temporary"
chmod 600 "$temporary"
sync -f "$temporary"
mv -f -- "$temporary" "$target"
sync -f "$checkpoint_dir"
trap - EXIT
