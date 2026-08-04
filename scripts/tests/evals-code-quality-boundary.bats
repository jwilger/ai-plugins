#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  LAUNCHER="$ROOT/scripts/evals/code-quality-codex-boundary"
  FIXTURE_ROOT="$BATS_TEST_TMPDIR/boundary"
  WORKSPACE="$FIXTURE_ROOT/workspace"
  CODEX_HOME_FIXTURE="$FIXTURE_ROOT/codex-home"
  PRIVATE_TMP="$FIXTURE_ROOT/private-tmp"
  REAL_HOME="$FIXTURE_ROOT/real-home"
  SIBLING_WORKSPACE="$FIXTURE_ROOT/sibling-workspace"
  RUNTIME_ROOT="$FIXTURE_ROOT/codex-runtime"
  TOOL_ROOT="$FIXTURE_ROOT/tools"

  mkdir -p \
    "$WORKSPACE" \
    "$CODEX_HOME_FIXTURE" \
    "$PRIVATE_TMP" \
    "$REAL_HOME" \
    "$RUNTIME_ROOT/bin" \
    "$RUNTIME_ROOT/codex-path" \
    "$RUNTIME_ROOT/codex-resources" \
    "$SIBLING_WORKSPACE" \
    "$TOOL_ROOT"
  chmod 700 "$CODEX_HOME_FIXTURE" "$PRIVATE_TMP"
  mkdir -p "$WORKSPACE/.git"
  printf 'ai-plugins downstream code-quality workspace\n' \
    >"$WORKSPACE/.git/.ai-plugins-code-quality-workspace"
  printf 'ai-plugins Codex eval home\n' \
    >"$CODEX_HOME_FIXTURE/.ai-plugins-eval-home"
  printf '%s\n' '[marketplaces.ai-plugins]' 'source_type = "local"' \
    >"$CODEX_HOME_FIXTURE/config.toml"
  printf '%s\n' '{"auth_mode":"chatgpt","tokens":{"access_token":"fixture-chatgpt-access","refresh_token":"fixture-chatgpt-refresh"}}' \
    >"$CODEX_HOME_FIXTURE/auth.json"
  chmod 600 "$CODEX_HOME_FIXTURE/auth.json"
  mkdir -p \
    "$CODEX_HOME_FIXTURE/marketplace/.agents/plugins" \
    "$CODEX_HOME_FIXTURE/plugins/cache/ai-plugins/fixture/0.1.0/skills/fixture" \
    "$CODEX_HOME_FIXTURE/skills/.system/fixture-system"
  printf '%s\n' '{"plugins":[]}' \
    >"$CODEX_HOME_FIXTURE/marketplace/.agents/plugins/marketplace.json"
  printf '%s\n' '# Fixture skill' \
    >"$CODEX_HOME_FIXTURE/plugins/cache/ai-plugins/fixture/0.1.0/skills/fixture/SKILL.md"
  printf '%s\n' '# Fixture system skill' \
    >"$CODEX_HOME_FIXTURE/skills/.system/fixture-system/SKILL.md"
  printf '%s\n' "$REAL_HOME" "$SIBLING_WORKSPACE" "$ROOT" \
    >"$CODEX_HOME_FIXTURE/protected-paths"

  write_fake_codex
  printf '%s\n' 'codex-cli 0.144.5' >"$RUNTIME_ROOT/version"
  printf '%s\n' \
    '{"layoutVersion":1,"version":"0.144.5","target":"fixture-linux-musl","variant":"codex","entrypoint":"bin/codex","resourcesDir":"codex-resources","pathDir":"codex-path"}' \
    >"$RUNTIME_ROOT/codex-package.json"
  printf '%s\n' '#!/bin/sh' 'exit 0' >"$RUNTIME_ROOT/codex-path/rg"
  printf '%s\n' '#!/bin/sh' 'exit 0' >"$RUNTIME_ROOT/codex-resources/bwrap"
  chmod 755 "$RUNTIME_ROOT/codex-path/rg" "$RUNTIME_ROOT/codex-resources/bwrap"
  write_fake_bwrap
  write_fake_timeout
  write_fake_prlimit
  write_fake_systemd_run

  CODEX_SHA256="$(sha256sum "$RUNTIME_ROOT/bin/codex" | cut -d' ' -f1)"
  write_execution_surface fixture-model medium
  CODEX_RESOURCE_BWRAP_SHA256="$(sha256sum "$RUNTIME_ROOT/codex-resources/bwrap" | cut -d' ' -f1)"
  CODEX_RG_SHA256="$(sha256sum "$RUNTIME_ROOT/codex-path/rg" | cut -d' ' -f1)"
  BWRAP_SHA256="$(sha256sum "$TOOL_ROOT/bwrap" | cut -d' ' -f1)"
  PRLIMIT_SHA256="$(sha256sum "$TOOL_ROOT/prlimit" | cut -d' ' -f1)"
  TIMEOUT_SHA256="$(sha256sum "$TOOL_ROOT/timeout" | cut -d' ' -f1)"
  SYSTEMD_RUN_SHA256="$(sha256sum "$TOOL_ROOT/systemd-run" | cut -d' ' -f1)"
  SAFE_TOOL_PATH="$(
    printf '%s\n' \
      "$(dirname "$(realpath "$(command -v awk)")")" \
      "$(dirname "$(realpath "$(command -v bash)")")" \
      "$(dirname "$(realpath "$(command -v env)")")" \
      "$(dirname "$(realpath "$(command -v find)")")" \
      "$(dirname "$(realpath "$(command -v tar)")")" |
      sort -u |
      paste -sd:
  )"
  NIX_STORE_CLOSURE="$FIXTURE_ROOT/nix-store-closure"
  printf '%s\n' ${SAFE_TOOL_PATH//:/ } |
    sed 's#/bin$##' |
    sort -u >"$NIX_STORE_CLOSURE"
  chmod 400 "$NIX_STORE_CLOSURE"
  NIX_STORE_CLOSURE_SHA256="$(sha256sum "$NIX_STORE_CLOSURE" | cut -d' ' -f1)"
}

write_fake_codex() {
  cat >"$RUNTIME_ROOT/bin/codex" <<'SH'
#!@HOST_BASH@
set -eu

fixture_root='@FIXTURE_ROOT@'
runtime_root="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
if [ "${1:-}" = "--version" ]; then
  [ "$HOME" = "$CODEX_HOME" ]
  [ "$HOME" != '@REAL_HOME@' ]
  [ "$TMPDIR" != '@PRIVATE_TMP@' ]
  [ -z "${OPENAI_API_KEY:-}" ]
  [ -z "${CODEX_API_KEY:-}" ]
  [ -z "${HOST_ONLY_SECRET:-}" ]
  printf '%s\n' "$HOME" "$TMPDIR" >"$fixture_root/version-probe-paths"
  printf '%s\n' "$*" >>"$fixture_root/codex-runtime/invocations"
  cat "$fixture_root/codex-runtime/version"
  exit 0
fi

printf '%s\n' "$*" >>"$CODEX_HOME/invocations"
printf '%s\n' "$@" >"$CODEX_HOME/received-args"
case " $* " in
  *' --model fixture-leak-probe '*)
    [ -z "${OPENAI_API_KEY:-}" ]
    [ -z "${CODEX_API_KEY:-}" ]
    [ -r "$CODEX_HOME/auth.json" ]
    [ -z "${HOST_ONLY_SECRET:-}" ]
    LEAK_PROBE_FILE="$CODEX_HOME/leak-probe" \
      "$FAKE_SAFE_SHELL_SOURCE" -c '
        [ -z "${OPENAI_API_KEY:-}" ]
        [ -z "${CODEX_API_KEY:-}" ]
        [ -z "${HOST_ONLY_SECRET:-}" ]
        printf "%s\n" model-shell-clean >"$LEAK_PROBE_FILE"
      '
    ;;
  *' --model fixture-containment-probe '*)
    [ ! -e '@REAL_HOME@' ]
    [ ! -e '@SIBLING_WORKSPACE@' ]
    [ ! -e '@REPOSITORY_ROOT@' ]
    [ -r /proc/self/environ ]
    printf '%s\n' contained
    ;;
  *' --model fixture-home-overlay-probe '*)
    [ -r "$CODEX_HOME/skills/.system/fixture-system/SKILL.md" ]
    [ -r /runtime/marketplace/.agents/plugins/marketplace.json ]
    [ ! -e "$CODEX_HOME/.ai-plugins-eval-home" ]
    [ ! -e "$CODEX_HOME/.ai-plugins-execution-surface.json" ]
    [ ! -e /workspace/.git/.ai-plugins-code-quality-workspace ]
    [ ! -e /runtime/workspace-input/.git/.ai-plugins-code-quality-workspace ]
    [ ! -e /runtime/workspace-output/.git/.ai-plugins-code-quality-workspace ]
    [ ! -s /runtime/workspace-exporter ]
    [ "$(cat /proc/sys/kernel/hostname)" = workspace ]
    ! grep -Eiq 'eval|benchmark|ai-plugins' /etc/passwd
    mountinfo="$(cat /proc/self/mountinfo)"
    for forbidden_mount_source in \
      '@FIXTURE_ROOT@' \
      '@REPOSITORY_ROOT@' \
      '@RUNTIME_ROOT@' \
      'ai-plugins-code-quality' \
      'no-marketplace-skills' \
      'targeted-quality-skills' \
      'all-marketplace-skills'; do
      case "$mountinfo" in
        *"$forbidden_mount_source"*) exit 78 ;;
      esac
    done
    case "$mountinfo" in
      *sample-[0-9]* | *[Ee][Vv][Aa][Ll]* | *[Bb][Ee][Nn][Cc][Hh][Mm][Aa][Rr][Kk]*)
        exit 78
        ;;
    esac
    printf '%s\n' mutable >"$CODEX_HOME/ephemeral-state"
    if { printf '%s\n' tampered >"$CODEX_HOME/config.toml"; } 2>/dev/null; then
      exit 73
    fi
    if {
      printf '%s\n' tampered \
        >"$CODEX_HOME/plugins/cache/ai-plugins/fixture/0.1.0/skills/fixture/SKILL.md"
    } 2>/dev/null; then
      exit 74
    fi
    if {
      printf '%s\n' tampered \
        >"$CODEX_HOME/skills/.system/fixture-system/SKILL.md"
    } 2>/dev/null; then
      exit 75
    fi
    if {
      printf '%s\n' tampered \
        >/runtime/marketplace/.agents/plugins/marketplace.json
    } 2>/dev/null; then
      exit 76
    fi
    printf '%s\n' immutable-inputs-protected
    ;;
  *' --model fixture-inner-sandbox-probe '*)
    [ -z "${OPENAI_API_KEY:-}" ]
    [ -z "${CODEX_API_KEY:-}" ]
    [ -r "$CODEX_HOME/auth.json" ]
    printf '%s\n' codex-retained-chatgpt-auth
    checker="$CODEX_HOME/inner-sandbox-check"
    cat >"$checker" <<'CHECK'
#!/bin/bash
set -eu
for environment in /proc/[0-9]*/environ; do
  [ -r "$environment" ] || continue
  while IFS= read -r -d '' entry; do
    case "$entry" in
      *fixture-api-key*) exit 91 ;;
    esac
  done <"$environment"
done
printf '%s\n' inner-sandbox-clean
CHECK
    chmod 700 "$checker"
    env -i \
      CODEX_HOME="$CODEX_HOME" \
      HOME="$HOME" \
      INNER_BWRAP=/runtime/codex-package/codex-resources/bwrap \
      INNER_CHECKER="$checker" \
      PATH="$PATH" \
      TMPDIR="$TMPDIR" \
      @HOST_BASH@ -c '
        exec "$INNER_BWRAP" \
          --unshare-user \
          --unshare-pid \
          --die-with-parent \
          --new-session \
          --ro-bind / / \
          --proc /proc \
          --dev /dev \
          -- /bin/bash "$INNER_CHECKER"
      '
    ;;
  *' --model fixture-auth-refresh-probe '*)
    printf '%s\n' '{"auth_mode":"chatgpt","tokens":{"access_token":"refreshed-access","refresh_token":"rotated-refresh"}}' \
      >"$CODEX_HOME/auth.json.refresh"
    mv "$CODEX_HOME/auth.json.refresh" "$CODEX_HOME/auth.json"
    ;;
esac
case " $* " in
  *' --model fixture-sleep '*) sleep 10 ;;
  *' --model fixture-cancel '*)
    printf '%s\n' "$$" >"$CODEX_HOME/cancel-pid"
    trap 'printf "%s\n" cancelled >"$CODEX_HOME/cancelled"; exit 143' HUP INT TERM
    sleep 10
    printf '%s\n' survived >"$CODEX_HOME/survived-cancellation"
    ;;
  *' --model fixture-output-flood '*)
    line=0123456789abcdef0123456789abcdef
    count=0
    while [ "$count" -lt 256 ]; do
      printf '%s\n' "$line"
      count=$((count + 1))
    done
    ;;
  *' --model fixture-workspace-flood '*)
    line=0123456789abcdef0123456789abcdef
    count=0
    while [ "$count" -lt 256 ]; do
      printf '%s\n' "$line" >>generated-output
      count=$((count + 1))
    done
    ;;
  *' --model fixture-live-workspace-flood '*)
    trap 'printf "%s\n" stopped-by-live-limit >"$CODEX_HOME/live-limit-stop"; exit 143' TERM
    line=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
    while :; do
      printf '%s\n' "$line" >>live-generated-output
      sleep 0.02
    done
    ;;
  *' --model fixture-workspace-symlink '*)
    ln -s /etc/passwd escaped-link
    ;;
  *' --model fixture-unlinked-disk-probe '*)
    exec 9>hidden-unlinked-output
    rm hidden-unlinked-output
    if head -c 2097152 /dev/zero >&9; then
      exit 74
    fi
    exec 9>&-
    printf '%s\n' unlinked-disk-bounded
    ;;
  *' --model fixture-success-output '*)
    line=0123456789abcdef0123456789abcdef
    count=0
    while [ "$count" -lt 32768 ]; do
      printf '%s\n' "$line"
      count=$((count + 1))
    done
    printf '%s\n' delivery-complete
    ;;
  *' --model fixture-store-closure-probe '*)
    forbidden_store_path="$(cat .forbidden-store-path)"
    [ ! -e "$forbidden_store_path" ]
    printf '%s\n' undeclared-store-source-hidden
    ;;
  *' --model fixture-git-preservation-probe '*)
    printf '%s\n' tampered >.git/.ai-plugins-code-quality-workspace
    printf '%s\n' hostile >.git/config
    rm original-working-tree-file
    printf '%s\n' candidate >candidate-working-tree-file
    ln candidate-working-tree-file candidate-hardlink
    ;;
esac
case " $* " in
  *' --ephemeral '*) ;;
  *)
    mkdir -p "$CODEX_HOME/sessions"
    printf '%s\n' leaked >"$CODEX_HOME/sessions/rollout.jsonl"
    ;;
esac
SH
  sed -i \
    -e "s|@FIXTURE_ROOT@|$FIXTURE_ROOT|g" \
    -e "s|@HOST_BASH@|$(realpath "$(command -v bash)")|g" \
    -e "s|@REAL_HOME@|$REAL_HOME|g" \
    -e "s|@PRIVATE_TMP@|$PRIVATE_TMP|g" \
    -e "s|@SIBLING_WORKSPACE@|$SIBLING_WORKSPACE|g" \
    -e "s|@REPOSITORY_ROOT@|$ROOT|g" \
    -e "s|@RUNTIME_ROOT@|$RUNTIME_ROOT|g" \
    "$RUNTIME_ROOT/bin/codex"
  chmod 755 "$RUNTIME_ROOT/bin/codex"
}

write_fake_bwrap() {
  cat >"$TOOL_ROOT/bwrap" <<'SH'
#!/bin/sh
set -eu

fixture_root='@FIXTURE_ROOT@'
printf '%s\n' "$@" >"$fixture_root/bwrap-args"
safe_shell_source=
workspace_input_source=
workspace_output_source=
auth_input_source=
auth_output_source=

while [ "$#" -gt 0 ]; do
  case "$1" in
    --clearenv)
      for variable in $(env | sed 's/=.*//'); do
        unset "$variable"
      done
      shift
      ;;
    --setenv)
      export "$2=$3"
      shift 3
      ;;
    --)
      shift
      break
      ;;
    --ro-bind)
      if [ "$3" = /bin/bash ]; then
        safe_shell_source="$2"
      fi
      if [ "$3" = /runtime/workspace-input ]; then
        workspace_input_source="$2"
      fi
      if [ "$3" = /runtime/auth-input/auth.json ]; then
        auth_input_source="$2"
      fi
      shift 3
      ;;
    --bind)
      if [ "$3" = /runtime/workspace-output ]; then
        workspace_output_source="$2"
      fi
      if [ "$3" = /runtime/auth-output/auth.json ]; then
        auth_output_source="$2"
      fi
      shift 3
      ;;
    --symlink)
      shift 3
      ;;
    --chdir)
      working_directory="$2"
      shift 2
      ;;
    --dir | --tmpfs | --proc | --dev | --hostname | --size | --preserve-fds)
      shift 2
      ;;
    *)
      shift
      ;;
  esac
done

[ "$#" -gt 4 ] || exit 70
[ -n "$workspace_input_source" ] || exit 70
[ -n "$workspace_output_source" ] || exit 70
[ -n "$auth_input_source" ] || exit 70
[ -n "$auth_output_source" ] || exit 70
shift
workspace_argument="$1"
workspace_max_bytes="$2"
workspace_max_entries="$3"
shift 3
export FAKE_SAFE_SHELL_SOURCE="$safe_shell_source"
if [ "${CODEX_HOME:-}" = /runtime/codex-home ]; then
  export CODEX_HOME="$fixture_root/codex-home"
  export HOME="$CODEX_HOME"
fi
if [ "${TMPDIR:-}" = /runtime/tmp ]; then
  export TMPDIR="$fixture_root/private-tmp"
fi
if [ "${working_directory:-}" = /workspace ]; then
  working_directory="$fixture_root/workspace"
fi
[ -n "$working_directory" ] && cd "$working_directory"
auth_original="$fixture_root/original-auth.json"
cp "$fixture_root/codex-home/auth.json" "$auth_original"
cp "$auth_input_source" "$fixture_root/codex-home/auth.json"
set +e
"$fixture_root/codex-runtime/bin/codex" "$@"
codex_status=$?
set -e
cp "$fixture_root/codex-home/auth.json" "$auth_output_source"
cp "$auth_original" "$fixture_root/codex-home/auth.json"
chmod 600 "$fixture_root/codex-home/auth.json"

unsafe_entry="$(
  find "$working_directory" -xdev \
    -path "$working_directory/.git" -prune -o \
    ! -type d ! -type f -print -quit
)"
unsafe_hardlink="$(
  find "$working_directory" -xdev \
    -path "$working_directory/.git" -prune -o \
    -type f -links +1 -print -quit
)"
if [ -n "$unsafe_entry" ] || [ -n "$unsafe_hardlink" ]; then
  printf '%s\n' \
    'CODE_QUALITY_BOUNDARY_ERROR:safety:workspace-export-unsafe' >&2
  exit 77
fi

entry_count="$(
  find "$working_directory" -xdev -mindepth 1 \
    -path "$working_directory/.git" -prune -o -printf x |
    wc -c
)"
if [ "$entry_count" -gt "$workspace_max_entries" ]; then
  printf '%s\n' \
    'CODE_QUALITY_BOUNDARY_ERROR:safety:workspace-entry-limit-exceeded' >&2
  exit 77
fi
if ! find "$working_directory" -xdev \
  -path "$working_directory/.git" -prune -o \
  -type f -printf '%s\n' |
  awk -v limit="$workspace_max_bytes" \
    '{ total += $1; if (total > limit) exit 1 }'; then
  printf '%s\n' \
    'CODE_QUALITY_BOUNDARY_ERROR:safety:workspace-byte-limit-exceeded' >&2
  exit 77
fi

[ "$(cat "$workspace_output_source/.git/.ai-plugins-code-quality-workspace")" = \
  'ai-plugins downstream code-quality workspace' ] || exit 77
find "$workspace_output_source" -mindepth 1 -maxdepth 1 \
  ! -name .git -exec rm -rf -- {} +
if ! (cd "$working_directory" &&
  tar --create --format=posix --exclude=./.git --file=- .) |
  tar --extract --file=- --directory="$workspace_output_source" \
    --no-same-owner; then
  printf '%s\n' \
    'CODE_QUALITY_BOUNDARY_ERROR:safety:workspace-copy-out-failed' >&2
  exit 77
fi
printf '%s\n' complete \
  >"$workspace_output_source/.git/.workspace-export-complete"
exit "$codex_status"
SH
  sed -i "s|@FIXTURE_ROOT@|$FIXTURE_ROOT|g" "$TOOL_ROOT/bwrap"
  chmod 755 "$TOOL_ROOT/bwrap"
}

write_fake_timeout() {
  cat >"$TOOL_ROOT/timeout" <<'SH'
#!/bin/sh
set -eu

fixture_root='@FIXTURE_ROOT@'
printf '%s\n' "$@" >"$fixture_root/timeout-args"
while [ "$#" -gt 0 ]; do
  case "$1" in
    --signal=* | --kill-after=*) shift ;;
    *) break ;;
  esac
done
[ "$#" -ge 2 ] || exit 70
shift
exec "$@"
SH
  sed -i "s|@FIXTURE_ROOT@|$FIXTURE_ROOT|g" "$TOOL_ROOT/timeout"
  chmod 755 "$TOOL_ROOT/timeout"
}

write_fake_prlimit() {
  cat >"$TOOL_ROOT/prlimit" <<'SH'
#!/bin/sh
set -eu

fixture_root='@FIXTURE_ROOT@'
printf '%s\n' "$@" >"$fixture_root/prlimit-args"
while [ "$#" -gt 0 ] && [ "$1" != -- ]; do
  shift
done
[ "$#" -gt 1 ] || exit 70
shift
exec "$@"
SH
  sed -i "s|@FIXTURE_ROOT@|$FIXTURE_ROOT|g" "$TOOL_ROOT/prlimit"
  chmod 755 "$TOOL_ROOT/prlimit"
}

write_fake_systemd_run() {
  cat >"$TOOL_ROOT/systemd-run" <<'SH'
#!/bin/sh
set -eu

fixture_root='@FIXTURE_ROOT@'
printf '%s\n' "$@" >"$fixture_root/systemd-run-args"
if [ -e "$fixture_root/systemd-auto-cancel" ]; then
  printf '%s\n' "$$" >"$fixture_root/systemd-run-pid"
  trap 'printf "%s\n" terminated >"@FIXTURE_ROOT@/systemd-run-terminated"; exit 143' TERM
  kill -TERM "$PPID"
  sleep 10
fi
while [ "$#" -gt 0 ] && [ "$1" != -- ]; do
  shift
done
[ "$#" -gt 1 ] || exit 70
shift
if [ "${1##*/}" = resource-scope-entry ]; then
  scope_entry="$1"
  shift
  [ "$#" -gt 1 ] || exit 70
  scope_entry_marker="$1"
  shift
  while IFS= read -r line || [ -n "$line" ]; do
    printf '%s\n' "$line"
  done <"$scope_entry" >"$fixture_root/resource-scope-entry"
  printf '%s\n' entered >"$scope_entry_marker"
  unset XDG_RUNTIME_DIR
fi
exec "$@"
SH
  sed -i "s|@FIXTURE_ROOT@|$FIXTURE_ROOT|g" "$TOOL_ROOT/systemd-run"
  chmod 755 "$TOOL_ROOT/systemd-run"
}

build_boundary_env() {
  BOUNDARY_ENV=(
    "PATH=$SAFE_TOOL_PATH"
    'HOST_ONLY_SECRET=must-not-cross-boundary'
    "CODEX_HOME=${BOUNDARY_CODEX_HOME:-$CODEX_HOME_FIXTURE}"
    "HOME=$REAL_HOME"
    "TMPDIR=${BOUNDARY_PRIVATE_TMP:-$PRIVATE_TMP}"
    "CODE_QUALITY_CODEX_REAL_BIN=${BOUNDARY_CODEX_REAL_BIN:-$RUNTIME_ROOT/bin/codex}"
    "CODE_QUALITY_CODEX_EXPECTED_VERSION=${BOUNDARY_EXPECTED_VERSION:-codex-cli 0.144.5}"
    "CODE_QUALITY_CODEX_EXPECTED_SHA256=${BOUNDARY_EXPECTED_SHA256:-$CODEX_SHA256}"
    "CODE_QUALITY_CODEX_RESOURCE_BWRAP_EXPECTED_SHA256=${BOUNDARY_CODEX_RESOURCE_BWRAP_SHA256:-$CODEX_RESOURCE_BWRAP_SHA256}"
    "CODE_QUALITY_CODEX_RG_EXPECTED_SHA256=${BOUNDARY_CODEX_RG_SHA256:-$CODEX_RG_SHA256}"
    "CODE_QUALITY_NODE_BIN=$(realpath "$(command -v node)")"
    "CODE_QUALITY_BWRAP_BIN=${BOUNDARY_BWRAP_BIN:-$TOOL_ROOT/bwrap}"
    "CODE_QUALITY_BWRAP_EXPECTED_SHA256=${BOUNDARY_BWRAP_SHA256:-$BWRAP_SHA256}"
    "CODE_QUALITY_PRLIMIT_BIN=${BOUNDARY_PRLIMIT_BIN:-$TOOL_ROOT/prlimit}"
    "CODE_QUALITY_PRLIMIT_EXPECTED_SHA256=${BOUNDARY_PRLIMIT_SHA256:-$PRLIMIT_SHA256}"
    "CODE_QUALITY_SYSTEMD_RUN_BIN=${BOUNDARY_SYSTEMD_RUN_BIN:-$TOOL_ROOT/systemd-run}"
    "CODE_QUALITY_SYSTEMD_RUN_EXPECTED_SHA256=${BOUNDARY_SYSTEMD_RUN_SHA256:-$SYSTEMD_RUN_SHA256}"
    "CODE_QUALITY_TIMEOUT_BIN=${BOUNDARY_TIMEOUT_BIN:-$TOOL_ROOT/timeout}"
    "CODE_QUALITY_TIMEOUT_EXPECTED_SHA256=${BOUNDARY_TIMEOUT_SHA256:-$TIMEOUT_SHA256}"
    "CODE_QUALITY_WALL_TIMEOUT_SECONDS=${BOUNDARY_WALL_TIMEOUT_SECONDS:-3600}"
    "CODE_QUALITY_OUTPUT_MAX_BYTES=${BOUNDARY_OUTPUT_MAX_BYTES:-16777216}"
    "CODE_QUALITY_WORKSPACE_MAX_BYTES=${BOUNDARY_WORKSPACE_MAX_BYTES:-2147483648}"
    "CODE_QUALITY_WORKSPACE_MAX_ENTRIES=${BOUNDARY_WORKSPACE_MAX_ENTRIES:-100000}"
    "CODE_QUALITY_TOOL_PATH=${BOUNDARY_TOOL_PATH:-$SAFE_TOOL_PATH}"
    "CODE_QUALITY_NIX_STORE_CLOSURE=${BOUNDARY_NIX_STORE_CLOSURE:-$NIX_STORE_CLOSURE}"
    "CODE_QUALITY_NIX_STORE_CLOSURE_EXPECTED_SHA256=${BOUNDARY_NIX_STORE_CLOSURE_SHA256:-$NIX_STORE_CLOSURE_SHA256}"
  )
  if [ -n "${BOUNDARY_DISCOVERY_CANARY:-}" ]; then
    BOUNDARY_ENV+=(
      "CODE_QUALITY_CODEX_DISCOVERY_CANARY=$BOUNDARY_DISCOVERY_CANARY"
      "CODE_QUALITY_CODEX_DISCOVERY_PROMPT=$BOUNDARY_DISCOVERY_PROMPT"
    )
  fi
}

run_boundary() {
  local -a arguments=("$@")
  local model=fixture-model
  local has_model=0
  local has_reasoning=0
  local has_plugins=0
  local expected_plugins=false
  local index argument value

  if [ -z "${BOUNDARY_DISCOVERY_CANARY:-}" ] && \
    [ "${BOUNDARY_RAW_INVOCATION:-0}" != 1 ]; then
    for ((index = 0; index < ${#arguments[@]}; index += 1)); do
      argument="${arguments[$index]}"
      case "$argument" in
        --model|-m)
          has_model=1
          if [ $((index + 1)) -lt ${#arguments[@]} ] && \
            [[ "${arguments[$((index + 1))]}" != -* ]]; then
            model="${arguments[$((index + 1))]}"
          fi
          ;;
        --model=*)
          has_model=1
          model="${argument#--model=}"
          ;;
        --config|-c)
          if [ $((index + 1)) -lt ${#arguments[@]} ]; then
            value="${arguments[$((index + 1))]}"
            [[ "$value" == model_reasoning_effort=* ]] && has_reasoning=1
            [[ "$value" == features.plugins=* ]] && has_plugins=1
          fi
          ;;
        --config=model_reasoning_effort=*|-c=model_reasoning_effort=*)
          has_reasoning=1
          ;;
        --config=features.plugins=*|-c=features.plugins=*)
          has_plugins=1
          ;;
      esac
    done
    if [ "$has_model" -eq 0 ]; then
      arguments+=(--model "$model")
    fi
    if [ "$has_reasoning" -eq 0 ]; then
      arguments+=(--config 'model_reasoning_effort="medium"')
    fi
    if [ "$has_plugins" -eq 0 ]; then
      grep -Fq '[plugins."' "$CODEX_HOME_FIXTURE/config.toml" && \
        expected_plugins=true
      arguments+=(--config "features.plugins=$expected_plugins")
    fi
    if [ "${BOUNDARY_PRESERVE_EXECUTION_SURFACE:-0}" != 1 ]; then
      write_execution_surface "$model" medium
    fi
  fi
  build_boundary_env
  run env -i "${BOUNDARY_ENV[@]}" "$LAUNCHER" "${arguments[@]}"
}

write_execution_surface() {
  local model="$1"
  local reasoning="$2"
  jq -nS \
    --arg boundary "$(printf 'a%.0s' {1..64})" \
    --arg codex "$CODEX_SHA256" \
    --arg model "$model" \
    --arg reasoning "$reasoning" \
    --arg toolchain "$(printf 'b%.0s' {1..64})" \
    '{
      boundarySha256: $boundary,
      codexBinarySha256: $codex,
      codexVersion: "codex-cli 0.144.5",
      model: $model,
      reasoningEffort: $reasoning,
      schemaVersion: 1,
      toolchainCompositionSha256: $toolchain
    }' >"$CODEX_HOME_FIXTURE/.ai-plugins-execution-surface.json"
  chmod 600 "$CODEX_HOME_FIXTURE/.ai-plugins-execution-surface.json"
}

prepare_real_nix_closure() {
  command -v nix-store >/dev/null || skip 'nix-store is unavailable'
  real_closure="$FIXTURE_ROOT/real-nix-store-closure"
  mapfile -t store_roots < <(
    printf '%s\n' ${SAFE_TOOL_PATH//:/ } |
      sed 's#/bin$##' |
      sort -u
  )
  if ! nix-store --query --requisites "${store_roots[@]}" 2>/dev/null |
    sort -u >"$real_closure"; then
    skip 'the Nix store database is unavailable'
  fi
  chmod 400 "$real_closure"
  BOUNDARY_NIX_STORE_CLOSURE="$real_closure"
  BOUNDARY_NIX_STORE_CLOSURE_SHA256="$(sha256sum "$real_closure" | cut -d' ' -f1)"
}

prepare_real_inner_bwrap() {
  command -v codex >/dev/null || skip 'Codex is unavailable'
  real_codex="$(realpath "$(command -v codex)")"
  real_inner_bwrap="$(dirname "$(dirname "$real_codex")")/codex-resources/bwrap"
  [ -f "$real_inner_bwrap" ] || skip 'installed Codex has no packaged bubblewrap'
  cp "$real_inner_bwrap" "$RUNTIME_ROOT/codex-resources/bwrap"
  BOUNDARY_CODEX_RESOURCE_BWRAP_SHA256="$(
    sha256sum "$RUNTIME_ROOT/codex-resources/bwrap" | cut -d' ' -f1
  )"
}

prepare_real_discovery_runtime() {
  local resolver="$ROOT/scripts/evals/resolve-code-quality-codex.mjs"
  local workspace_preparer="$ROOT/scripts/evals/prepare-code-quality-workspaces.mjs"
  local runtime_preparer="$ROOT/scripts/evals/prepare-code-quality-runtime.mjs"
  local auth_preparer="$ROOT/scripts/evals/prepare-code-quality-auth.mjs"
  local resolution
  local discovery_codex_bin
  local discovery_codex_sha256
  local discovery_codex_version
  local discovery_model
  local discovery_reasoning
  local discovery_boundary_sha256
  local discovery_toolchain_sha256
  local version_home="$FIXTURE_ROOT/discovery-version-home"
  local version_tmp="$FIXTURE_ROOT/discovery-version-tmp"

  resolution="$(node "$resolver")"
  discovery_codex_bin="$(jq -er '.codexBin' <<<"$resolution")"
  discovery_codex_sha256="$(
    sha256sum "$discovery_codex_bin" | cut -d' ' -f1
  )"
  mkdir -m 700 "$version_home" "$version_tmp"
  discovery_codex_version="$(
    env -i \
      CODEX_HOME="$version_home" \
      HOME="$version_home" \
      TMPDIR="$version_tmp" \
      LANG=C.UTF-8 \
      LC_ALL=C.UTF-8 \
      "$discovery_codex_bin" --version
  )"
  discovery_model="$(
    jq -er '.provider.model' \
      "$ROOT/evals/benchmarks/downstream-code-quality/benchmark.json"
  )"
  discovery_reasoning="$(
    jq -er '.provider.reasoningEffort' \
      "$ROOT/evals/benchmarks/downstream-code-quality/benchmark.json"
  )"
  discovery_boundary_sha256="$(
    node - "$LAUNCHER" "$ROOT/scripts/evals/code-quality-codex-boundary.mjs" <<'NODE'
const crypto = require("node:crypto");
const fs = require("node:fs");
const hash = crypto.createHash("sha256");
for (const file of process.argv.slice(2)) {
  const bytes = fs.readFileSync(file);
  const length = Buffer.alloc(8);
  length.writeBigUInt64BE(BigInt(bytes.length));
  hash.update(length).update(bytes);
}
process.stdout.write(hash.digest("hex"));
NODE
  )"
  discovery_toolchain_sha256="$(
    printf '%s\n' \
      "$discovery_codex_sha256" \
      "$BOUNDARY_NIX_STORE_CLOSURE_SHA256" | sha256sum | cut -d' ' -f1
  )"

  DISCOVERY_WORK_ROOT="$FIXTURE_ROOT/discovery-workspaces"
  DISCOVERY_RUNTIME_ROOT="$FIXTURE_ROOT/discovery-runtime"
  node "$workspace_preparer" "$DISCOVERY_WORK_ROOT" \
    --case rust-cli-feature --samples 1 >/dev/null
  env -i \
    HOME="$version_home" \
    TMPDIR="$FIXTURE_ROOT" \
    PATH="$PATH" \
    LANG=C.UTF-8 \
    LC_ALL=C.UTF-8 \
    CODE_QUALITY_CODEX_REAL_BIN="$discovery_codex_bin" \
    CODE_QUALITY_CODEX_EXPECTED_SHA256="$discovery_codex_sha256" \
    CODE_QUALITY_CODEX_EXPECTED_VERSION="$discovery_codex_version" \
    CODE_QUALITY_CODEX_MODEL="$discovery_model" \
    CODE_QUALITY_CODEX_REASONING_EFFORT="$discovery_reasoning" \
    CODE_QUALITY_BOUNDARY_SHA256="$discovery_boundary_sha256" \
    CODE_QUALITY_TOOLCHAIN_COMPOSITION_SHA256="$discovery_toolchain_sha256" \
    "$(realpath "$(command -v node)")" "$runtime_preparer" \
      "$DISCOVERY_WORK_ROOT/manifest.json" \
      "$DISCOVERY_RUNTIME_ROOT" >/dev/null
  node "$auth_preparer" \
    "$CODEX_HOME_FIXTURE/auth.json" \
    "$DISCOVERY_RUNTIME_ROOT/manifest.json" >/dev/null

  DISCOVERY_RUNTIME_MANIFEST="$DISCOVERY_RUNTIME_ROOT/manifest.json"
  select_discovery_mode targeted-quality-skills
  BOUNDARY_CODEX_REAL_BIN="$discovery_codex_bin"
  BOUNDARY_EXPECTED_SHA256="$discovery_codex_sha256"
  BOUNDARY_EXPECTED_VERSION="$discovery_codex_version"
  BOUNDARY_CODEX_RESOURCE_BWRAP_SHA256="$(
    sha256sum "$(jq -er '.resourceBwrap' <<<"$resolution")" | cut -d' ' -f1
  )"
  BOUNDARY_CODEX_RG_SHA256="$(
    sha256sum "$(jq -er '.resourceRg' <<<"$resolution")" | cut -d' ' -f1
  )"
  BOUNDARY_DISCOVERY_CANARY=1
  BOUNDARY_DISCOVERY_PROMPT="$(
    node -e '
      const inputs = require(process.argv[1]);
      process.stdout.write(inputs.renderPrompt(inputs.promptFor({ caseId: "rust-cli-feature" })));
    ' "$ROOT/evals/benchmarks/downstream-code-quality/benchmark-inputs.cjs"
  )"
}

select_discovery_mode() {
  local mode="$1"
  WORKSPACE="$(
    jq -er --arg mode "$mode" '.rows[] | select(.mode == $mode) | .workspace' \
      "$DISCOVERY_RUNTIME_MANIFEST"
  )"
  CODEX_HOME_FIXTURE="$(
    jq -er --arg mode "$mode" '.rows[] | select(.mode == $mode) | .codexHome' \
      "$DISCOVERY_RUNTIME_MANIFEST"
  )"
  PRIVATE_TMP="$(
    jq -er --arg mode "$mode" '.rows[] | select(.mode == $mode) | .codexTmp' \
      "$DISCOVERY_RUNTIME_MANIFEST"
  )"
}

@test "Codex boundary injects ephemeral and mandatory model-tool safety overrides" {
  run_boundary \
    exec \
    --experimental-json \
    --model fixture-model \
    --sandbox workspace-write \
    --cd "$WORKSPACE"

  [ "$status" -eq 0 ]
  [ "$(paste -sd'|' "$CODEX_HOME_FIXTURE/received-args")" = \
    "exec|--ephemeral|--experimental-json|--model|fixture-model|--sandbox|workspace-write|--cd|/workspace|--config|model_reasoning_effort=\"medium\"|--config|features.plugins=false|--config|sandbox_workspace_write.network_access=false|--config|web_search=\"disabled\"|--config|approval_policy=\"never\"|--config|shell_environment_policy.inherit=\"none\"|--config|shell_environment_policy.experimental_use_profile=false|--config|shell_environment_policy.ignore_default_excludes=false" ]
  [ ! -e "$CODEX_HOME_FIXTURE/sessions" ]
}

@test "Codex boundary rejects model and reasoning drift from the execution surface" {
  write_execution_surface fixture-pinned-model medium
  BOUNDARY_PRESERVE_EXECUTION_SURFACE=1 \
    run_boundary \
      exec \
      --experimental-json \
      --model fixture-drifted-model \
      --sandbox workspace-write \
      --cd "$WORKSPACE"

  [ "$status" -eq 64 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:invocation:model-does-not-match-execution-surface"* ]]
  [ ! -e "$CODEX_HOME_FIXTURE/received-args" ]

  rm -f "$CODEX_HOME_FIXTURE/received-args"
  write_execution_surface fixture-pinned-model medium
  BOUNDARY_PRESERVE_EXECUTION_SURFACE=1 \
    run_boundary \
      exec \
      --experimental-json \
      --model fixture-pinned-model \
      --sandbox workspace-write \
      --cd "$WORKSPACE" \
      --config 'model_reasoning_effort="high"'

  [ "$status" -eq 64 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:invocation:reasoning-effort-does-not-match-execution-surface"* ]]
  [ ! -e "$CODEX_HOME_FIXTURE/received-args" ]
}

@test "Codex boundary rejects a plugin flag that disagrees with the runtime surface" {
  run_boundary \
    exec \
    --experimental-json \
    --model fixture-model \
    --sandbox workspace-write \
    --cd "$WORKSPACE" \
    --config features.plugins=true

  [ "$status" -eq 64 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:invocation:plugin-flag-does-not-match-runtime"* ]]
  [ ! -e "$CODEX_HOME_FIXTURE/received-args" ]
}

@test "Codex boundary requires explicit model reasoning and plugin controls" {
  BOUNDARY_RAW_INVOCATION=1 \
    run_boundary \
      exec \
      --experimental-json \
      --sandbox workspace-write \
      --cd "$WORKSPACE" \
      --config 'model_reasoning_effort="medium"' \
      --config features.plugins=false

  [ "$status" -eq 64 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:invocation:exactly-one-model-required"* ]]

  BOUNDARY_RAW_INVOCATION=1 \
    run_boundary \
      exec \
      --experimental-json \
      --model fixture-model \
      --sandbox workspace-write \
      --cd "$WORKSPACE" \
      --config features.plugins=false

  [ "$status" -eq 64 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:invocation:exactly-one-reasoning-effort-required"* ]]

  BOUNDARY_RAW_INVOCATION=1 \
    run_boundary \
      exec \
      --experimental-json \
      --model fixture-model \
      --sandbox workspace-write \
      --cd "$WORKSPACE" \
      --config 'model_reasoning_effort="medium"'

  [ "$status" -eq 64 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:invocation:exactly-one-plugin-flag-required"* ]]
}

@test "Codex SDK full provider config crosses the boundary contract" {
  build_boundary_env
  sdk_probe="$FIXTURE_ROOT/sdk-contract.mjs"
  sdk_launcher="$FIXTURE_ROOT/sdk-launcher"
  cat >"$sdk_launcher" <<'SH'
#!/bin/sh
printf '%s\n' "$@" >'@FIXTURE_ROOT@/sdk-received-args'
exec '@LAUNCHER@' "$@" 2>'@FIXTURE_ROOT@/sdk-boundary-stderr'
SH
  sed -i \
    -e "s|@FIXTURE_ROOT@|$FIXTURE_ROOT|g" \
    -e "s|@LAUNCHER@|$LAUNCHER|g" \
    "$sdk_launcher"
  chmod 755 "$sdk_launcher"
  write_execution_surface fixture-sdk-contract medium
  cat >"$sdk_probe" <<'NODE'
import fs from "node:fs";
import { pathToFileURL } from "node:url";

const [sdkPath, yamlPath, configPath, launcher, workspace] = process.argv.slice(2);
const { Codex } = await import(pathToFileURL(sdkPath));
const { default: YAML } = await import(pathToFileURL(yamlPath));
const parsed = YAML.parse(fs.readFileSync(configPath, "utf8"));
const provider = parsed.providers[0].config;
function resolveTemplates(value) {
  if (Array.isArray(value)) return value.map(resolveTemplates);
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value).map(([key, child]) => [key, resolveTemplates(child)]),
    );
  }
  if (typeof value !== "string") return value;
  return value
    .replaceAll("{{ workspace }}", workspace)
    .replaceAll(
      "{{ env.CODE_QUALITY_TOOL_PATH }}",
      process.env.CODE_QUALITY_TOOL_PATH,
    );
}

const codex = new Codex({
  codexPathOverride: launcher,
  config: resolveTemplates(provider.cli_config),
  env: process.env,
});
const thread = codex.startThread({
  approvalPolicy: provider.approval_policy,
  model: "fixture-sdk-contract",
  modelReasoningEffort: "medium",
  networkAccessEnabled: provider.network_access_enabled,
  sandboxMode: provider.sandbox_mode,
  skipGitRepoCheck: provider.skip_git_repo_check,
  webSearchMode: provider.web_search_mode,
  workingDirectory: workspace,
});
await thread.run("exercise the provider argv contract");
NODE

  node_bin="$(realpath "$(command -v node)")"
  run env -i \
    "${BOUNDARY_ENV[@]}" \
    "$node_bin" \
    "$sdk_probe" \
    "$ROOT/node_modules/@openai/codex-sdk/dist/index.js" \
    "$(realpath "$(node -p 'require.resolve("yaml")')")" \
    "$ROOT/evals/benchmarks/downstream-code-quality/promptfooconfig.yaml" \
    "$sdk_launcher" \
    "$WORKSPACE"

  if [ "$status" -ne 0 ]; then
    paste -sd'|' "$FIXTURE_ROOT/sdk-received-args" >&3
    cat "$FIXTURE_ROOT/sdk-boundary-stderr" >&3
  fi
  [ "$status" -eq 0 ]
  grep -Fxq 'history.persistence="none"' "$CODEX_HOME_FIXTURE/received-args"
  grep -Fxq 'features.shell_tool=true' "$CODEX_HOME_FIXTURE/received-args"
  grep -Fxq 'features.plugins=false' "$CODEX_HOME_FIXTURE/received-args"
  grep -Fxq \
    'shell_environment_policy.set.HOME="/workspace/.home"' \
    "$CODEX_HOME_FIXTURE/received-args"
  grep -Fxq \
    'shell_environment_policy.set.CARGO_TARGET_DIR="/workspace/target"' \
    "$CODEX_HOME_FIXTURE/received-args"
  ! grep -Fq -- "$WORKSPACE" "$CODEX_HOME_FIXTURE/received-args"
}

@test "Codex boundary reduces duplicate ephemeral options to exactly one" {
  run_boundary \
    exec \
    --ephemeral \
    --experimental-json \
    --ephemeral \
    --sandbox workspace-write \
    --cd "$WORKSPACE"

  [ "$status" -eq 0 ]
  [ "$(grep -Fxc -- '--ephemeral' "$CODEX_HOME_FIXTURE/received-args")" -eq 1 ]
}

@test "Codex boundary rejects positional arguments after the option terminator" {
  run_boundary \
    exec \
    --experimental-json \
    --sandbox workspace-write \
    --cd "$WORKSPACE" \
    -- \
    --ephemeral

  [ "$status" -eq 64 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:invocation:positional-arguments-forbidden"* ]]
  [ ! -e "$RUNTIME_ROOT/invocations" ]
}

@test "Codex boundary rejects a hash mismatch before executing the supplied binary" {
  BOUNDARY_EXPECTED_SHA256="$(printf '0%.0s' {1..64})" \
    run_boundary exec --experimental-json --sandbox workspace-write --cd "$WORKSPACE"

  [ "$status" -eq 65 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:integrity:sha256-mismatch"* ]]
  [ ! -e "$RUNTIME_ROOT/invocations" ]
}

@test "Codex boundary rejects a supplied binary whose exact version is not pinned" {
  printf '%s\n' 'codex-cli 0.145.0' >"$RUNTIME_ROOT/version"

  run_boundary exec --experimental-json --sandbox workspace-write --cd "$WORKSPACE"

  [ "$status" -eq 65 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:integrity:version-mismatch"* ]]
  [ "$(cat "$RUNTIME_ROOT/invocations")" = '--version' ]
  [ ! -e "$CODEX_HOME_FIXTURE/received-args" ]
}

@test "Codex boundary constructs a minimal mount and environment namespace" {
  run_boundary \
    exec \
    --experimental-json \
    --sandbox workspace-write \
    --cd "$WORKSPACE"

  [ "$status" -eq 0 ]
  boundary_argv="$(paste -sd'|' "$FIXTURE_ROOT/bwrap-args")"
  [[ "$boundary_argv" == *"--unshare-all|--share-net|--die-with-parent|--new-session"* ]]
  [[ "$boundary_argv" != *"--clearenv"* ]]
  [[ "$boundary_argv" != *"fixture-api-key"* ]]
  [[ "$boundary_argv" == *"--proc|/proc"* ]]
  [[ "$boundary_argv" != *"--ro-bind|/nix/store|/nix/store"* ]]
  while IFS= read -r store_path; do
    [[ "$boundary_argv" == *"--ro-bind|$store_path|$store_path"* ]]
  done <"$NIX_STORE_CLOSURE"
  [[ "$boundary_argv" == *"/codex-package/bin/codex|/runtime/codex-package/bin/codex"* ]]
  [[ "$boundary_argv" == *"/codex-package/codex-resources/bwrap|/runtime/codex-package/codex-resources/bwrap"* ]]
  [[ "$boundary_argv" == *"/codex-package/codex-path/rg|/runtime/codex-package/codex-path/rg"* ]]
  [[ "$boundary_argv" != *"--ro-bind|$RUNTIME_ROOT|/runtime/codex-package"* ]]
  [[ "$boundary_argv" == *"--ro-bind|/tmp/workspace-runtime-"*"/workspace-input|/runtime/workspace-input"* ]]
  [[ "$boundary_argv" == *"--bind|/tmp/workspace-runtime-"*"/workspace-output|/runtime/workspace-output"* ]]
  [[ "$boundary_argv" != *"--bind|$WORKSPACE|$WORKSPACE"* ]]
  [[ "$boundary_argv" == *"--size|2147483648|--tmpfs|/workspace"* ]]
  [[ "$boundary_argv" == *"--size|134217728|--tmpfs|/runtime/codex-home"* ]]
  [[ "$boundary_argv" != *".ai-plugins-eval-home"* ]]
  [[ "$boundary_argv" != *".ai-plugins-execution-surface.json"* ]]
  [[ "$boundary_argv" == *"--ro-bind|/tmp/workspace-runtime-"*"/codex-home-inputs/config.toml|/runtime/codex-home/config.toml"* ]]
  [[ "$boundary_argv" == *"--ro-bind|/tmp/workspace-runtime-"*"/codex-home-inputs/auth.json|/runtime/auth-input/auth.json"* ]]
  [[ "$boundary_argv" == *"--bind|/tmp/workspace-runtime-"*"/refreshed-auth.json|/runtime/auth-output/auth.json"* ]]
  [[ "$boundary_argv" != *"|$CODEX_HOME_FIXTURE/auth.json|"* ]]
  [[ "$boundary_argv" == *"--ro-bind|/tmp/workspace-runtime-"*"/codex-home-inputs/plugins|/runtime/codex-home/plugins"* ]]
  [[ "$boundary_argv" == *"--dir|/runtime/codex-home/skills|--ro-bind|/tmp/workspace-runtime-"*"/codex-home-inputs/skills/.system|/runtime/codex-home/skills/.system"* ]]
  [[ "$boundary_argv" == *"--dir|/runtime/marketplace|--ro-bind|/tmp/workspace-runtime-"*"/marketplace|/runtime/marketplace"* ]]
  [[ "$boundary_argv" != *"--ro-bind|$CODEX_HOME_FIXTURE|$CODEX_HOME_FIXTURE"* ]]
  [[ "$boundary_argv" == *"--size|134217728|--tmpfs|/runtime/tmp"* ]]
  [[ "$boundary_argv" == *"--setenv|HOME|/runtime/codex-home"* ]]
  [[ "$boundary_argv" == *"--setenv|PATH|$SAFE_TOOL_PATH"* ]]
  [[ "$boundary_argv" == *"--setenv|SHELL|/bin/bash"* ]]
  [[ "$boundary_argv" == *"--dir|/bin"* ]]
  [[ "$boundary_argv" == *"--ro-bind|/tmp/workspace-runtime-"*"/safe-shell|/bin/bash"* ]]
  [[ "$boundary_argv" == *"--symlink|bash|/bin/sh"* ]]
  [[ "$boundary_argv" == *"/passwd|/etc/passwd"* ]]
  [[ "$boundary_argv" == *"/group|/etc/group"* ]]
  [[ "$boundary_argv" == *"/hosts|/etc/hosts"* ]]
  [[ "$boundary_argv" == *"/nsswitch.conf|/etc/nsswitch.conf"* ]]
  [[ "$boundary_argv" == *"--hostname|workspace"* ]]
  [[ "$boundary_argv" == *"|/etc/resolv.conf"* ]]
  [[ "$boundary_argv" == *"|/etc/ssl/certs/ca-certificates.crt"* ]]
  [[ "$boundary_argv" == *"--setenv|SSL_CERT_FILE|/etc/ssl/certs/ca-certificates.crt"* ]]
  [[ "$boundary_argv" != *"--ro-bind|/etc|/etc"* ]]
  [[ "$boundary_argv" == *"--chdir|/workspace"* ]]
  [[ "$boundary_argv" != *"$REAL_HOME"* ]]
  [[ "$boundary_argv" != *"$SIBLING_WORKSPACE"* ]]
  [[ "$boundary_argv" != *"$ROOT"* ]]
  [[ "$boundary_argv" != *"$FIXTURE_ROOT"* ]]
  [[ "$boundary_argv" != *"ai-plugins-code-quality"* ]]
  [[ "$boundary_argv" != *"benchmark"* ]]
  [[ "$boundary_argv" != *"no-marketplace-skills"* ]]
  [[ "$boundary_argv" != *"targeted-quality-skills"* ]]
  [[ "$boundary_argv" != *"all-marketplace-skills"* ]]
  [[ ! "$boundary_argv" =~ sample-[0-9]+ ]]
}

@test "Codex boundary applies deterministic wall and operating-system resource limits" {
  run_boundary \
    exec \
    --experimental-json \
    --sandbox workspace-write \
    --cd "$WORKSPACE"

  [ "$status" -eq 0 ]
  timeout_argv="$(paste -sd'|' "$FIXTURE_ROOT/timeout-args")"
  prlimit_argv="$(paste -sd'|' "$FIXTURE_ROOT/prlimit-args")"
  systemd_argv="$(paste -sd'|' "$FIXTURE_ROOT/systemd-run-args")"
  [[ "$systemd_argv" == \
    "--user|--scope|--quiet|--collect|--expand-environment=false|--unit=ai-plugins-code-quality-"*"|--property=MemoryMax=8589934592|--property=MemorySwapMax=0|--property=TasksMax=512|--property=CPUQuota=400%|--property=KillMode=control-group|--|/tmp/workspace-runtime-"*"/resource-scope-entry|"* ]]
  [[ "$systemd_argv" != *"OOMPolicy"* ]]
  scope_entry="$FIXTURE_ROOT/resource-scope-entry"
  grep -Fq -- 'oom_group="/sys/fs/cgroup${cgroup_path}/memory.oom.group"' "$scope_entry"
  grep -Fq -- 'printf "%s\n" 1 >"$oom_group"' "$scope_entry"
  grep -Fq -- 'IFS= read -r oom_group_value <"$oom_group"' "$scope_entry"
  grep -Fq -- '[ "$oom_group_value" = 1 ]' "$scope_entry"
  oom_group_line="$(grep -Fn -- 'memory.oom.group' "$scope_entry" | head -n 1 | cut -d: -f1)"
  marker_line="$(grep -Fn -- 'entered >"$scope_entry_marker"' "$scope_entry" | cut -d: -f1)"
  [ "$oom_group_line" -lt "$marker_line" ]
  [[ "$timeout_argv" == \
    "--signal=TERM|--kill-after=5s|3600s|/tmp/workspace-runtime-"*"/runtime-tools/prlimit|"* ]]
  [[ "$prlimit_argv" == \
    "--as=8589934592|--cpu=1800|--fsize=1073741824|--nofile=1024|--core=0|--|/tmp/workspace-runtime-"*"/runtime-tools/bwrap|"* ]]
  [[ "$prlimit_argv" != *"--nproc="* ]]
}

@test "Codex boundary catches cancellation delivered during detached spawn" {
  touch "$FIXTURE_ROOT/systemd-auto-cancel"

  run_boundary \
    exec \
    --experimental-json \
    --sandbox workspace-write \
    --cd "$WORKSPACE"

  [ "$status" -eq 70 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:runtime:cancelled-SIGTERM"* ]]
  [ -e "$FIXTURE_ROOT/systemd-run-terminated" ]
  [ ! -e "$CODEX_HOME_FIXTURE/received-args" ]
}

@test "Codex boundary drains successful output under reader backpressure" {
  write_execution_surface fixture-success-output medium
  build_boundary_env
  slow_reader="$FIXTURE_ROOT/slow-reader.mjs"
  cat >"$slow_reader" <<'NODE'
import { spawn } from "node:child_process";

const [launcher, workspace] = process.argv.slice(2);
const child = spawn(
  launcher,
  [
    "exec",
    "--experimental-json",
    "--model",
    "fixture-success-output",
    "--sandbox",
    "workspace-write",
    "--cd",
    workspace,
    "--config",
    'model_reasoning_effort="medium"',
    "--config",
    "features.plugins=false",
  ],
  { env: process.env, stdio: ["ignore", "pipe", "pipe"] },
);
let bytes = 0;
let stderr = "";
child.stdout.on("data", (chunk) => {
  bytes += chunk.length;
  child.stdout.pause();
  setTimeout(() => child.stdout.resume(), 2);
});
child.stderr.setEncoding("utf8");
child.stderr.on("data", (chunk) => {
  stderr += chunk;
});
const childClosed = new Promise((resolve) => child.once("close", resolve));
const stdoutEnded = new Promise((resolve) => child.stdout.once("end", resolve));
const [code] = await Promise.all([childClosed, stdoutEnded]);
{
  process.stdout.write(JSON.stringify({ bytes, code, stderr }));
}
NODE

  run env -i \
    "${BOUNDARY_ENV[@]}" \
    CODE_QUALITY_OUTPUT_MAX_BYTES=2097152 \
    "$(realpath "$(command -v node)")" \
    "$slow_reader" \
    "$LAUNCHER" \
    "$WORKSPACE"

  [ "$status" -eq 0 ]
  [ "$(jq -er '.code' <<<"$output")" -eq 0 ]
  [ "$(jq -er '.bytes' <<<"$output")" -eq 1081362 ]
  [ "$(jq -er '.stderr' <<<"$output")" = "" ]
}

@test "Codex boundary classifies a real wall timeout deterministically" {
  BOUNDARY_TIMEOUT_BIN="$(command -v timeout)" \
    BOUNDARY_TIMEOUT_SHA256="$(sha256sum "$(realpath "$(command -v timeout)")" | cut -d' ' -f1)" \
    BOUNDARY_WALL_TIMEOUT_SECONDS=1 \
    run_boundary \
      exec \
      --experimental-json \
      --model fixture-sleep \
      --sandbox workspace-write \
      --cd "$WORKSPACE"

  [ "$status" -eq 124 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:timeout:wall-1s"* ]]
}

@test "Codex boundary forwards caller cancellation to the detached process group" {
  write_execution_surface fixture-cancel medium
  build_boundary_env
  boundary_output="$FIXTURE_ROOT/cancellation-output"
  env -i "${BOUNDARY_ENV[@]}" \
    "$LAUNCHER" \
    exec \
    --experimental-json \
    --model fixture-cancel \
    --sandbox workspace-write \
    --cd "$WORKSPACE" \
    --config 'model_reasoning_effort="medium"' \
    --config features.plugins=false \
    >"$boundary_output" 2>&1 &
  launcher_pid=$!

  for _ in {1..40}; do
    [ -e "$CODEX_HOME_FIXTURE/cancel-pid" ] && break
    sleep 0.05
  done
  [ -e "$CODEX_HOME_FIXTURE/cancel-pid" ]
  kill -TERM "$launcher_pid"
  cancellation_status=0
  wait "$launcher_pid" || cancellation_status=$?

  for _ in {1..20}; do
    [ -e "$CODEX_HOME_FIXTURE/cancelled" ] && break
    sleep 0.05
  done
  if [ ! -e "$CODEX_HOME_FIXTURE/cancelled" ]; then
    kill -KILL "$(cat "$CODEX_HOME_FIXTURE/cancel-pid")" 2>/dev/null || true
  fi

  [ "$cancellation_status" -eq 70 ]
  [ -e "$CODEX_HOME_FIXTURE/cancelled" ]
  [ ! -e "$CODEX_HOME_FIXTURE/survived-cancellation" ]
  grep -Fq \
    'CODE_QUALITY_BOUNDARY_ERROR:runtime:cancelled-SIGTERM' \
    "$boundary_output"
}

@test "Codex boundary rejects a Codex home without its ownership marker" {
  rm "$CODEX_HOME_FIXTURE/.ai-plugins-eval-home"

  run_boundary \
    exec \
    --experimental-json \
    --sandbox workspace-write \
    --cd "$WORKSPACE"

  [ "$status" -eq 64 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:configuration:codex-home-marker-invalid"* ]]
  [ ! -e "$FIXTURE_ROOT/bwrap-args" ]
}

@test "Codex boundary rejects a Codex home without its immutable config" {
  rm "$CODEX_HOME_FIXTURE/config.toml"

  run_boundary \
    exec \
    --experimental-json \
    --sandbox workspace-write \
    --cd "$WORKSPACE"

  [ "$status" -eq 64 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:configuration:codex-home-config-invalid"* ]]
  [ ! -e "$FIXTURE_ROOT/bwrap-args" ]
}

@test "Codex boundary requires immutable system skills and sanitized marketplace inputs" {
  rm -rf "$CODEX_HOME_FIXTURE/skills/.system"

  run_boundary \
    exec \
    --experimental-json \
    --sandbox workspace-write \
    --cd "$WORKSPACE"

  [ "$status" -eq 64 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:configuration:codex-home-system-skills-unsafe"* ]]
  [ ! -e "$FIXTURE_ROOT/bwrap-args" ]

  mkdir -p "$CODEX_HOME_FIXTURE/skills/.system"
  rm -rf "$CODEX_HOME_FIXTURE/marketplace"

  run_boundary \
    exec \
    --experimental-json \
    --sandbox workspace-write \
    --cd "$WORKSPACE"

  [ "$status" -eq 64 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:configuration:codex-home-marketplace-unsafe"* ]]
  [ ! -e "$FIXTURE_ROOT/bwrap-args" ]
}

@test "Codex boundary rejects symlinked immutable Codex home inputs" {
  rm "$CODEX_HOME_FIXTURE/config.toml"
  ln -s /etc/passwd "$CODEX_HOME_FIXTURE/config.toml"

  run_boundary \
    exec \
    --experimental-json \
    --sandbox workspace-write \
    --cd "$WORKSPACE"

  [ "$status" -eq 64 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:configuration:codex-home-config-invalid"* ]]
  [ ! -e "$FIXTURE_ROOT/bwrap-args" ]

  rm "$CODEX_HOME_FIXTURE/config.toml"
  printf '%s\n' '[marketplaces.ai-plugins]' >"$CODEX_HOME_FIXTURE/config.toml"
  ln -s /etc/passwd \
    "$CODEX_HOME_FIXTURE/plugins/cache/ai-plugins/fixture/0.1.0/escaped"

  run_boundary \
    exec \
    --experimental-json \
    --sandbox workspace-write \
    --cd "$WORKSPACE"

  [ "$status" -eq 64 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:configuration:codex-home-plugins-unsafe"* ]]
  [ ! -e "$FIXTURE_ROOT/bwrap-args" ]
}

@test "Codex receives disposable ChatGPT auth without API-key environment variables" {
  run_boundary \
    exec \
    --experimental-json \
    --model fixture-leak-probe \
    --sandbox workspace-write \
    --cd "$WORKSPACE"

  [ "$status" -eq 0 ]
  [ "$(cat "$CODEX_HOME_FIXTURE/leak-probe")" = model-shell-clean ]
}

@test "Codex boundary preserves refreshed ChatGPT auth for later turns" {
  run_boundary \
    exec \
    --experimental-json \
    --model fixture-auth-refresh-probe \
    --sandbox workspace-write \
    --cd "$WORKSPACE"

  [ "$status" -eq 0 ]
  jq -e '.tokens.refresh_token == "rotated-refresh"' \
    "$CODEX_HOME_FIXTURE/auth.json"
}

@test "Codex boundary rejects a config override that gives model tools network access" {
  run_boundary \
    exec \
    --experimental-json \
    --sandbox workspace-write \
    --cd "$WORKSPACE" \
    --config sandbox_workspace_write.network_access=true

  [ "$status" -eq 64 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:invocation:unsafe-network-override"* ]]
  [ ! -e "$FIXTURE_ROOT/bwrap-args" ]
}

@test "Codex boundary rejects a binary outside a package-native Codex layout" {
  rm "$RUNTIME_ROOT/codex-package.json"

  run_boundary \
    exec \
    --experimental-json \
    --sandbox workspace-write \
    --cd "$WORKSPACE"

  [ "$status" -eq 64 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:configuration:codex-package-invalid"* ]]
  [ ! -e "$FIXTURE_ROOT/bwrap-args" ]
}

@test "Codex boundary requires the verified binary to be the package manifest entrypoint" {
  cp "$RUNTIME_ROOT/bin/codex" "$RUNTIME_ROOT/bin/probe"
  probe_sha256="$(sha256sum "$RUNTIME_ROOT/bin/probe" | cut -d' ' -f1)"

  BOUNDARY_CODEX_REAL_BIN="$RUNTIME_ROOT/bin/probe" \
    BOUNDARY_EXPECTED_SHA256="$probe_sha256" \
    run_boundary \
      exec \
      --experimental-json \
      --sandbox workspace-write \
      --cd "$WORKSPACE"

  [ "$status" -eq 64 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:configuration:codex-entrypoint-mismatch"* ]]
  [ ! -e "$FIXTURE_ROOT/bwrap-args" ]
}

@test "Codex boundary rejects an unpinned packaged inner sandbox helper" {
  BOUNDARY_CODEX_RESOURCE_BWRAP_SHA256="$(printf '0%.0s' {1..64})" \
    run_boundary \
      exec \
      --experimental-json \
      --sandbox workspace-write \
      --cd "$WORKSPACE"

  [ "$status" -eq 65 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:integrity:codex-resource-bwrap-sha256-mismatch"* ]]
  [ ! -e "$FIXTURE_ROOT/bwrap-args" ]
}

@test "Codex boundary rejects a private temp directory with group or other access" {
  chmod 755 "$PRIVATE_TMP"

  run_boundary \
    exec \
    --experimental-json \
    --sandbox workspace-write \
    --cd "$WORKSPACE"

  [ "$status" -eq 64 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:configuration:private-tmp-not-private"* ]]
  [ ! -e "$FIXTURE_ROOT/bwrap-args" ]
}

@test "Codex boundary rejects overlap between workspace and private state" {
  printf 'ai-plugins Codex eval home\n' \
    >"$WORKSPACE/.ai-plugins-eval-home"

  BOUNDARY_CODEX_HOME="$WORKSPACE" \
    run_boundary \
      exec \
      --experimental-json \
      --sandbox workspace-write \
      --cd "$WORKSPACE"

  [ "$status" -eq 64 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:configuration:path-overlap"* ]]
  [ ! -e "$FIXTURE_ROOT/bwrap-args" ]
}

@test "Codex boundary refuses to mount the ai-plugins checkout as the writable workspace" {
  run_boundary \
    exec \
    --experimental-json \
    --sandbox workspace-write \
    --cd "$ROOT"

  [ "$status" -eq 64 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:configuration:workspace-overlaps-repository"* ]]
  [ ! -e "$FIXTURE_ROOT/bwrap-args" ]
}

@test "Codex boundary refuses a symlink in place of the pinned native binary" {
  ln -s "$RUNTIME_ROOT/bin/codex" "$RUNTIME_ROOT/bin/codex-link"

  BOUNDARY_CODEX_REAL_BIN="$RUNTIME_ROOT/bin/codex-link" \
    run_boundary \
      exec \
      --experimental-json \
      --sandbox workspace-write \
      --cd "$WORKSPACE"

  [ "$status" -eq 64 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:configuration:codex-binary-not-canonical"* ]]
  [ ! -e "$RUNTIME_ROOT/invocations" ]
}

@test "Codex boundary rejects a CLI sandbox bypass" {
  run_boundary \
    exec \
    --experimental-json \
    --sandbox danger-full-access \
    --cd "$WORKSPACE"

  [ "$status" -eq 64 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:invocation:unsafe-sandbox-mode"* ]]
  [ ! -e "$RUNTIME_ROOT/invocations" ]
}

@test "Codex boundary rejects the short sandbox bypass alias" {
  run_boundary \
    exec \
    --experimental-json \
    -s danger-full-access \
    --cd "$WORKSPACE"

  [ "$status" -eq 64 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:invocation:unsafe-sandbox-mode"* ]]
  [ ! -e "$RUNTIME_ROOT/invocations" ]
}

@test "Codex boundary rejects alternate providers profiles and feature flags" {
  run_boundary \
    exec \
    --experimental-json \
    --oss \
    --sandbox workspace-write \
    --cd "$WORKSPACE"

  [ "$status" -eq 64 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:invocation:unsupported-codex-option"* ]]
  [ ! -e "$RUNTIME_ROOT/invocations" ]
}

@test "Codex boundary rejects denied flags smuggled as option values" {
  run_boundary \
    exec \
    --experimental-json \
    --model \
    --oss \
    --sandbox workspace-write \
    --cd "$WORKSPACE"

  [ "$status" -eq 64 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:invocation:option-like-model-value"* ]]
  [ ! -e "$RUNTIME_ROOT/invocations" ]
}

@test "Codex boundary rejects MCP and extra writable-root configuration" {
  run_boundary \
    exec \
    --experimental-json \
    --sandbox workspace-write \
    --cd "$WORKSPACE" \
    --config 'mcp_servers.exfil.command="curl"'

  [ "$status" -eq 64 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:invocation:unsupported-config-override"* ]]
  [ ! -e "$RUNTIME_ROOT/invocations" ]

  run_boundary \
    exec \
    --experimental-json \
    --sandbox workspace-write \
    --cd "$WORKSPACE" \
    --config 'sandbox_workspace_write.writable_roots=["/codex-home"]'

  [ "$status" -eq 64 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:invocation:unsafe-writable-roots"* ]]
  [ ! -e "$RUNTIME_ROOT/invocations" ]
}

@test "real bubblewrap exposes only the declared runtime and disposable mounts" {
  command -v bwrap >/dev/null || skip 'bubblewrap is unavailable'
  prepare_real_nix_closure
  prepare_real_inner_bwrap

  BOUNDARY_BWRAP_BIN="$(realpath "$(command -v bwrap)")" \
    BOUNDARY_BWRAP_SHA256="$(sha256sum "$(realpath "$(command -v bwrap)")" | cut -d' ' -f1)" \
    run_boundary \
      exec \
      --experimental-json \
      --model fixture-containment-probe \
      --sandbox workspace-write \
      --cd "$WORKSPACE"

  [ "$status" -eq 0 ]
  [[ "$output" == *"contained"* ]]
  [ ! -e "$CODEX_HOME_FIXTURE/sessions" ]
}

@test "real bubblewrap keeps Codex state writable while protecting immutable home inputs" {
  command -v bwrap >/dev/null || skip 'bubblewrap is unavailable'
  prepare_real_nix_closure
  prepare_real_inner_bwrap

  BOUNDARY_BWRAP_BIN="$(realpath "$(command -v bwrap)")" \
    BOUNDARY_BWRAP_SHA256="$(sha256sum "$(realpath "$(command -v bwrap)")" | cut -d' ' -f1)" \
    run_boundary \
      exec \
      --experimental-json \
      --model fixture-home-overlay-probe \
      --sandbox workspace-write \
      --cd "$WORKSPACE"

  [ "$status" -eq 0 ]
  [[ "$output" == *"immutable-inputs-protected"* ]]
  [ ! -e "$CODEX_HOME_FIXTURE/ephemeral-state" ]
  [ "$(cat "$CODEX_HOME_FIXTURE/.ai-plugins-eval-home")" = 'ai-plugins Codex eval home' ]
  jq -e '
    .schemaVersion == 1 and
    .model == "fixture-home-overlay-probe" and
    .reasoningEffort == "medium"
  ' "$CODEX_HOME_FIXTURE/.ai-plugins-execution-surface.json"
  grep -Fq 'source_type = "local"' "$CODEX_HOME_FIXTURE/config.toml"
  [ "$(cat "$CODEX_HOME_FIXTURE/plugins/cache/ai-plugins/fixture/0.1.0/skills/fixture/SKILL.md")" = '# Fixture skill' ]
  [ "$(cat "$CODEX_HOME_FIXTURE/skills/.system/fixture-system/SKILL.md")" = '# Fixture system skill' ]
  [ "$(cat "$CODEX_HOME_FIXTURE/marketplace/.agents/plugins/marketplace.json")" = '{"plugins":[]}' ]
}

@test "exact boundary exposes only blinded condition-specific skill discovery" {
  command -v bwrap >/dev/null || skip 'bubblewrap is unavailable'
  command -v codex >/dev/null || skip 'Codex is unavailable'
  prepare_real_nix_closure
  prepare_real_discovery_runtime
  BOUNDARY_BWRAP_BIN="$(realpath "$(command -v bwrap)")"
  BOUNDARY_BWRAP_SHA256="$(
    sha256sum "$BOUNDARY_BWRAP_BIN" | cut -d' ' -f1
  )"

  for mode in \
    no-marketplace-skills \
    targeted-quality-skills \
    all-marketplace-skills; do
    select_discovery_mode "$mode"
    run_boundary \
      exec \
      --experimental-json \
      --sandbox workspace-write \
      --cd "$WORKSPACE"

    [ "$status" -eq 0 ]
    canary_json="$FIXTURE_ROOT/discovery-prompt-input-$mode.json"
    printf '%s\n' "$output" >"$canary_json"
    jq empty "$canary_json"

    run node - \
      "$canary_json" \
      "$DISCOVERY_RUNTIME_MANIFEST" \
      "$BOUNDARY_DISCOVERY_PROMPT" \
      "$mode" \
      "$ROOT" \
      "$FIXTURE_ROOT" \
      "$DISCOVERY_WORK_ROOT" \
      "$DISCOVERY_RUNTIME_ROOT" \
      "$WORKSPACE" \
      "$CODEX_HOME_FIXTURE" \
      "$PRIVATE_TMP" <<'NODE'
const assert = require("node:assert/strict");
const crypto = require("node:crypto");
const fs = require("node:fs");

const [
  promptFile,
  runtimeManifestFile,
  expectedPrompt,
  expectedMode,
  ...hostPaths
] = process.argv.slice(2);
const promptInput = JSON.parse(fs.readFileSync(promptFile, "utf8"));
const runtime = JSON.parse(fs.readFileSync(runtimeManifestFile, "utf8"));
const texts = promptInput.flatMap((item) =>
  Array.isArray(item?.content)
    ? item.content
        .filter((content) => content?.type === "input_text")
        .map((content) => content.text)
    : [],
);
const completePrompt = texts.join("\n");
const skillsBlock = texts.find((text) =>
  text.startsWith("<skills_instructions>"),
);
assert.ok(skillsBlock, "model-visible skills block is missing");
assert.ok(texts.includes(expectedPrompt), "rendered task prompt changed");

const availableSection = skillsBlock
  .split("### Available skills\n", 2)[1]
  ?.split("\n</skills_instructions>", 1)[0];
assert.ok(availableSection, "available-skills section is missing");
const discoveredNames = [
  ...availableSection.matchAll(
    /^- ([a-z0-9]+(?:-[a-z0-9]+)*(?::[a-z0-9]+(?:-[a-z0-9]+)*)?): /gm,
  ),
].map((match) => match[1]);
const canonicalDiscoveredNames = [...new Set(discoveredNames)].sort();
assert.equal(discoveredNames.length, canonicalDiscoveredNames.length);

const expectedRow = runtime.rows.find((row) => row.mode === expectedMode);
assert.ok(expectedRow, "expected condition row is missing");
const expectedNames = expectedRow.availableSkills
  .map((name) => name.replace(/^codex-system:/, ""))
  .sort();
assert.deepEqual(canonicalDiscoveredNames, expectedNames);
assert.ok(
  expectedRow.availableSkills.some((name) => name.startsWith("codex-system:")),
);
if (expectedMode === "no-marketplace-skills") {
  assert.ok(
    expectedRow.availableSkills.every((name) =>
      name.startsWith("codex-system:"),
    ),
  );
} else {
  assert.ok(
    expectedRow.availableSkills.some(
      (name) => !name.startsWith("codex-system:"),
    ),
  );
}

const skillPaths = [
  ...availableSection.matchAll(/\(file: ([^)]+\/SKILL\.md)\)/g),
].map((match) => match[1]);
assert.equal(skillPaths.length, discoveredNames.length);
for (const skillPath of skillPaths) {
  assert.match(
    skillPath,
    /^\/runtime\/codex-home\/(?:skills\/\.system\/[a-z0-9-]+|plugins\/cache\/ai-plugins\/[a-z0-9-]+\/[0-9A-Za-z.+-]+\/skills\/[a-z0-9-]+)\/SKILL\.md$/,
  );
}

for (const hostPath of hostPaths) {
  assert.ok(!completePrompt.includes(hostPath), `host path disclosed: ${hostPath}`);
}
for (const forbidden of [
  /\bsample-[0-9]+\b/,
  /\b(?:no-marketplace-skills|targeted-quality-skills|all-marketplace-skills)\b/,
  /\.plugin-eval(?:\/|\b)/,
  /(?:^|\/)benchmark\.json\b/,
  /(?:^|\/)evals\//,
  /\/runtime\/marketplace(?:\/|\b)/,
  /\.agents\/plugins\/marketplace\.json/,
  /(?:source_type|last_updated)\s*=/,
  /"plugins"\s*:/,
  /successChecklist/,
  /hidden[- ]test/i,
  /eval[- ]control/i,
]) {
  assert.doesNotMatch(completePrompt, forbidden);
}
assert.doesNotMatch(
  expectedPrompt,
  /\b(?:eval|evaluation|disposable|treatment|benchmark)\b/i,
);

const canonicalNames = `${canonicalDiscoveredNames.join("\n")}\n`;
const digest = crypto.createHash("sha256").update(canonicalNames).digest("hex");
process.stdout.write(JSON.stringify({ digest, names: canonicalDiscoveredNames }));
NODE

    if [ "$status" -ne 0 ]; then
      printf '%s\n' "$mode: $output" >&3
    fi
    [ "$status" -eq 0 ]
    [[ "$(jq -er '.digest' <<<"$output")" =~ ^[0-9a-f]{64}$ ]]
    [ "$(jq -er '.names | length' <<<"$output")" -gt 1 ]
  done
}

@test "real packaged inner sandbox starts with fresh proc and no API-key environment" {
  command -v bwrap >/dev/null || skip 'bubblewrap is unavailable'
  command -v codex >/dev/null || skip 'Codex is unavailable'
  prepare_real_nix_closure
  prepare_real_inner_bwrap

  BOUNDARY_BWRAP_BIN="$(realpath "$(command -v bwrap)")" \
    BOUNDARY_BWRAP_SHA256="$(sha256sum "$(realpath "$(command -v bwrap)")" | cut -d' ' -f1)" \
    run_boundary \
      exec \
      --experimental-json \
      --model fixture-inner-sandbox-probe \
      --sandbox workspace-write \
      --cd "$WORKSPACE"

  [ "$status" -eq 0 ]
  [[ "$output" == *"inner-sandbox-clean"* ]]
  [[ "$output" == *"codex-retained-chatgpt-auth"* ]]
}

@test "bounded workspace tmpfs contains open-unlinked disk growth" {
  command -v bwrap >/dev/null || skip 'bubblewrap is unavailable'
  prepare_real_nix_closure
  prepare_real_inner_bwrap

  BOUNDARY_WORKSPACE_MAX_BYTES=1048576 \
    BOUNDARY_BWRAP_BIN="$(realpath "$(command -v bwrap)")" \
    BOUNDARY_BWRAP_SHA256="$(sha256sum "$(realpath "$(command -v bwrap)")" | cut -d' ' -f1)" \
    run_boundary \
      exec \
      --experimental-json \
      --model fixture-unlinked-disk-probe \
      --sandbox workspace-write \
      --cd "$WORKSPACE"

  [ "$status" -eq 0 ]
  [[ "$output" == *"unlinked-disk-bounded"* ]]
  [ ! -e "$WORKSPACE/hidden-unlinked-output" ]
  [ "$(du -sb "$WORKSPACE" | cut -f1)" -lt 1048576 ]
}

@test "exact Nix closure hides undeclared repository source snapshots" {
  command -v bwrap >/dev/null || skip 'bubblewrap is unavailable'
  prepare_real_nix_closure
  prepare_real_inner_bwrap
  forbidden_store_path=""
  while IFS= read -r source_file; do
    candidate="/nix/store/$(cut -d/ -f4 <<<"$source_file")"
    if ! grep -Fxq -- "$candidate" "$BOUNDARY_NIX_STORE_CLOSURE"; then
      forbidden_store_path="$candidate"
      break
    fi
  done < <(
    rg -l --glob AGENTS.md \
      'multi-harness AI plugin marketplace' /nix/store 2>/dev/null || true
  )
  [ -n "$forbidden_store_path" ] || skip 'no undeclared ai-plugins store snapshot exists'
  printf '%s\n' "$forbidden_store_path" >"$WORKSPACE/.forbidden-store-path"

  BOUNDARY_BWRAP_BIN="$(realpath "$(command -v bwrap)")" \
    BOUNDARY_BWRAP_SHA256="$(sha256sum "$(realpath "$(command -v bwrap)")" | cut -d' ' -f1)" \
    run_boundary \
      exec \
      --experimental-json \
      --model fixture-store-closure-probe \
      --sandbox workspace-write \
      --cd "$WORKSPACE"

  [ "$status" -eq 0 ]
  [[ "$output" == *"undeclared-store-source-hidden"* ]]
}

@test "tmpfs copy-out preserves trusted Git metadata and candidate deletions" {
  command -v bwrap >/dev/null || skip 'bubblewrap is unavailable'
  prepare_real_nix_closure
  prepare_real_inner_bwrap
  printf '%s\n' original >"$WORKSPACE/original-working-tree-file"
  git_digest_before="$(tar -C "$WORKSPACE" -cf - .git | sha256sum | cut -d' ' -f1)"

  BOUNDARY_BWRAP_BIN="$(realpath "$(command -v bwrap)")" \
    BOUNDARY_BWRAP_SHA256="$(sha256sum "$(realpath "$(command -v bwrap)")" | cut -d' ' -f1)" \
    run_boundary \
      exec \
      --experimental-json \
      --model fixture-git-preservation-probe \
      --sandbox workspace-write \
      --cd "$WORKSPACE"

  [ "$status" -eq 0 ]
  [ ! -e "$WORKSPACE/original-working-tree-file" ]
  [ "$(cat "$WORKSPACE/candidate-working-tree-file")" = candidate ]
  [ "$(cat "$WORKSPACE/candidate-hardlink")" = candidate ]
  [ "$(stat -c %h "$WORKSPACE/candidate-working-tree-file")" -eq 1 ]
  [ "$(stat -c %h "$WORKSPACE/candidate-hardlink")" -eq 1 ]
  [ "$(cat "$WORKSPACE/.git/.ai-plugins-code-quality-workspace")" = \
    'ai-plugins downstream code-quality workspace' ]
  [ ! -e "$WORKSPACE/.git/config" ]
  [ "$(tar -C "$WORKSPACE" -cf - .git | sha256sum | cut -d' ' -f1)" = \
    "$git_digest_before" ]
}

@test "installed Codex sandbox hides authentication from model tools" {
  command -v codex >/dev/null || skip 'Codex is unavailable'
  real_codex="$(realpath "$(command -v codex)")"
  runtime_root="$(dirname "$(dirname "$real_codex")")"
  [ -f "$runtime_root/codex-package.json" ] || \
    skip 'installed Codex is not the package-native layout'
  bash_bin="$(realpath "$(command -v bash)")"
  probe_home="$FIXTURE_ROOT/installed-codex-sandbox-home"
  mkdir -p "$probe_home"
  chmod 700 "$probe_home"
  printf '%s\n' '{"auth_mode":"chatgpt","tokens":{"access_token":"sandbox-auth-canary"}}' \
    >"$probe_home/auth.json"
  chmod 600 "$probe_home/auth.json"
  probe='set -eu
[ ! -r "@AUTH_FILE@" ]
for environment in /proc/[0-9]*/environ; do
  [ -r "$environment" ] || continue
  while IFS= read -r -d "" entry; do
    case "$entry" in
      *fixture-api-key*)
        printf "LEAK:%s\\n" "$environment"
        exit 91
        ;;
    esac
  done <"$environment"
done
printf "model-shell-clean\\n"'
  probe="${probe//@AUTH_FILE@/$probe_home/auth.json}"

  # This regression isolates PID/proc and environment behavior. The production
  # config disables networking; leaving it enabled here avoids a nested
  # NETLINK_ROUTE operation that managed test sandboxes commonly forbid.
  run env \
    OPENAI_API_KEY=fixture-api-key \
    CODEX_API_KEY=fixture-api-key \
    CODEX_HOME="$probe_home" \
    HOME="$probe_home" \
    "$real_codex" sandbox \
      -P workspace \
      -C "$WORKSPACE" \
      -c 'default_permissions="workspace"' \
      -c "permissions.workspace.filesystem={\":minimal\"=\"read\", \"$runtime_root\"=\"read\", \"/nix/store\"=\"read\", \":project_roots\"=\"write\"}" \
      -c 'permissions.workspace.network={enabled=true}' \
      -c 'shell_environment_policy.inherit="none"' \
      -c "shell_environment_policy.set.PATH=\"$(dirname "$bash_bin")\"" \
      -- "$bash_bin" -c "$probe"

  [ "$status" -eq 0 ]
  [[ "$output" == *"model-shell-clean"* ]]
  [[ "$output" != *"LEAK:"* ]]
}

@test "the installed package-native Codex binary reaches exec help inside the boundary" {
  command -v bwrap >/dev/null || skip 'bubblewrap is unavailable'
  command -v codex >/dev/null || skip 'Codex is unavailable'
  prepare_real_nix_closure
  real_codex="$(realpath "$(command -v codex)")"
  [ -f "$(dirname "$(dirname "$real_codex")")/codex-package.json" ] || \
    skip 'installed Codex is not the package-native layout'
  real_version="$("$real_codex" --version)"
  real_sha256="$(sha256sum "$real_codex" | cut -d' ' -f1)"
  real_runtime_root="$(dirname "$(dirname "$real_codex")")"

  BOUNDARY_CODEX_REAL_BIN="$real_codex" \
    BOUNDARY_EXPECTED_VERSION="$real_version" \
    BOUNDARY_EXPECTED_SHA256="$real_sha256" \
    BOUNDARY_CODEX_RESOURCE_BWRAP_SHA256="$(sha256sum "$real_runtime_root/codex-resources/bwrap" | cut -d' ' -f1)" \
    BOUNDARY_CODEX_RG_SHA256="$(sha256sum "$real_runtime_root/codex-path/rg" | cut -d' ' -f1)" \
    BOUNDARY_BWRAP_BIN="$(realpath "$(command -v bwrap)")" \
    BOUNDARY_BWRAP_SHA256="$(sha256sum "$(realpath "$(command -v bwrap)")" | cut -d' ' -f1)" \
    run_boundary \
      exec \
      --help \
      --sandbox workspace-write \
      --cd "$WORKSPACE"

  [ "$status" -eq 0 ]
  [[ "$output" == *"Usage: codex exec"* ]]
  [ ! -e "$CODEX_HOME_FIXTURE/sessions" ]
}

@test "Codex boundary rejects a writable directory anywhere in the model tool PATH" {
  BOUNDARY_TOOL_PATH="$SAFE_TOOL_PATH:$WORKSPACE" \
    run_boundary \
      exec \
      --experimental-json \
      --sandbox workspace-write \
      --cd "$WORKSPACE"

  [ "$status" -eq 64 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:configuration:unsafe-tool-path"* ]]
  [ ! -e "$FIXTURE_ROOT/bwrap-args" ]
}

@test "Codex boundary rejects a resource wrapper whose binary hash is not pinned" {
  BOUNDARY_BWRAP_SHA256="$(printf '0%.0s' {1..64})" \
    run_boundary \
      exec \
      --experimental-json \
      --sandbox workspace-write \
      --cd "$WORKSPACE"

  [ "$status" -eq 65 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:integrity:bwrap-sha256-mismatch"* ]]
  [ ! -e "$FIXTURE_ROOT/bwrap-args" ]
}

@test "Codex boundary rejects a Nix closure changed after its digest was pinned" {
  chmod 600 "$NIX_STORE_CLOSURE"
  printf '%s\n' "$(head -n 1 "$NIX_STORE_CLOSURE")" >>"$NIX_STORE_CLOSURE"
  chmod 400 "$NIX_STORE_CLOSURE"

  run_boundary \
    exec \
    --experimental-json \
    --sandbox workspace-write \
    --cd "$WORKSPACE"

  [ "$status" -eq 65 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:integrity:nix-store-closure-sha256-mismatch"* ]]
  [ ! -e "$FIXTURE_ROOT/bwrap-args" ]
}

@test "Codex boundary rejects a hash-valid Nix closure missing a declared tool root" {
  missing_root="$(
    printf '%s\n' "$SAFE_TOOL_PATH" | tr : '\n' | head -n 1 | sed 's#/bin$##'
  )"
  incomplete_closure="$FIXTURE_ROOT/incomplete-nix-store-closure"
  grep -Fxv -- "$missing_root" "$NIX_STORE_CLOSURE" >"$incomplete_closure"
  chmod 400 "$incomplete_closure"
  incomplete_sha256="$(
    sha256sum "$incomplete_closure" | cut -d' ' -f1
  )"

  BOUNDARY_NIX_STORE_CLOSURE="$incomplete_closure" \
    BOUNDARY_NIX_STORE_CLOSURE_SHA256="$incomplete_sha256" \
    run_boundary \
      exec \
      --experimental-json \
      --sandbox workspace-write \
      --cd "$WORKSPACE"

  [ "$status" -eq 64 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:configuration:nix-store-closure-incomplete"* ]]
  [ ! -e "$FIXTURE_ROOT/bwrap-args" ]
}

@test "Codex boundary terminates a child whose combined stdout and stderr exceed the cap" {
  BOUNDARY_OUTPUT_MAX_BYTES=1024 \
    run_boundary \
      exec \
      --experimental-json \
      --model fixture-output-flood \
      --sandbox workspace-write \
      --cd "$WORKSPACE"

  [ "$status" -eq 77 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:safety:output-limit-exceeded"* ]]
  [ "${#output}" -lt 2048 ]
}

@test "Codex boundary rejects aggregate workspace bytes above the cap" {
  BOUNDARY_WORKSPACE_MAX_BYTES=2048 \
    run_boundary \
      exec \
      --experimental-json \
      --model fixture-workspace-flood \
      --sandbox workspace-write \
      --cd "$WORKSPACE"

  [ "$status" -eq 77 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:safety:workspace-byte-limit-exceeded"* ]]
}

@test "Codex boundary enforces aggregate writable-state limits while the child runs" {
  BOUNDARY_WALL_TIMEOUT_SECONDS=2 \
    BOUNDARY_WORKSPACE_MAX_BYTES=4096 \
    run_boundary \
      exec \
      --experimental-json \
      --model fixture-live-workspace-flood \
      --sandbox workspace-write \
      --cd "$WORKSPACE"

  [ "$status" -eq 77 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:safety:workspace-byte-limit-exceeded"* ]]
  [ "$(cat "$CODEX_HOME_FIXTURE/live-limit-stop")" = stopped-by-live-limit ]
}

@test "Codex boundary rejects symlinks created in the disposable workspace" {
  run_boundary \
    exec \
    --experimental-json \
    --model fixture-workspace-symlink \
    --sandbox workspace-write \
    --cd "$WORKSPACE"

  [ "$status" -eq 77 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:safety:workspace-symlink-detected"* ]]
}

@test "Codex boundary rejects unsafe initial workspace state before Codex starts" {
  ln -s /etc/passwd "$WORKSPACE/preexisting-escape"

  run_boundary \
    exec \
    --experimental-json \
    --sandbox workspace-write \
    --cd "$WORKSPACE"

  [ "$status" -eq 77 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:safety:workspace-symlink-detected"* ]]
  [ ! -e "$RUNTIME_ROOT/invocations" ]
  [ ! -e "$FIXTURE_ROOT/bwrap-args" ]
}

@test "Codex boundary rejects a dangling plugin-cache alias" {
  rm -rf "$CODEX_HOME_FIXTURE/plugins"
  ln -s "$FIXTURE_ROOT/missing-plugin-cache" "$CODEX_HOME_FIXTURE/plugins"

  run_boundary \
    exec \
    --experimental-json \
    --sandbox workspace-write \
    --cd "$WORKSPACE"

  [ "$status" -eq 64 ]
  [[ "$output" == *"CODE_QUALITY_BOUNDARY_ERROR:configuration:codex-home-plugins-unsafe"* ]]
  [ ! -e "$RUNTIME_ROOT/invocations" ]
  [ ! -e "$FIXTURE_ROOT/bwrap-args" ]
}
