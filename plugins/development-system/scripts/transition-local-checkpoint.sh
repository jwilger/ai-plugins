#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
  echo 'usage: transition-local-checkpoint.sh CHECKPOINT_ID OPERATION_ID OPERATION INPUT_FILE' >&2
  exit 2
fi
script_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
exec "$script_root/write-local-checkpoint.sh" --operation "$@"
