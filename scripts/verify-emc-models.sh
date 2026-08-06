#!/usr/bin/env bash
set -euo pipefail

repository_root="$(git rev-parse --show-toplevel)"
runtime_root="$(mktemp -d "${TMPDIR:-/tmp}/ai-plugins-emc.XXXXXX")"
trap 'rm -rf "$runtime_root"' EXIT
real_quint="$(command -v quint)"
emc_verify_binary="$(dirname "$(command -v emc)")/.emc-wrapped"

verify_model() {
  local relative_model_root="$1"
  shift
  local source_root="$repository_root/$relative_model_root"
  local verification_root="$runtime_root/$(basename "$(dirname "$relative_model_root")")"

  (
    cd "$source_root"
    emc check
    for workflow in "$@"; do
      emc review gate --workflow "$workflow"
    done
  )

  mkdir -p "$verification_root"
  cp -a "$source_root/." "$verification_root/"
  (
    cd "$verification_root"
    emc sync
    emc check
    XDG_STATE_HOME="$runtime_root/xdg-state" \
      QUINT_HOME="$runtime_root/quint-home" \
      EMC_VERIFY_PARALLELISM="${EMC_VERIFY_PARALLELISM:-1}" \
      EMC_REAL_QUINT="$real_quint" \
      JVM_ARGS="${JVM_ARGS:--Xmx8192m}" \
      PATH="$repository_root/scripts/emc-bin:$PATH" \
      "$emc_verify_binary" verify
  )
}

verify_model \
  plugins/development-system/components/tiber/event-model \
  manage-work \
  recover-pushed-ci
verify_model \
  plugins/development-system/components/development-discipline/event-model \
  conduct-final-review
