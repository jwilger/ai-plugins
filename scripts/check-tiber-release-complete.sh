#!/usr/bin/env bash
set -euo pipefail

root="$(cd "${1:-.}" && pwd -P)"
manifest="$root/plugins/tiber/release-binaries.json"
checksums="$root/plugins/tiber/release-binaries.sha256"
launcher="$root/plugins/tiber/bin/tiber"

"$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)/check-tiber-release-manifest.sh" "$root"

# shellcheck source=/dev/null
source "$root/plugins/tiber/scripts/detect-target.sh"
host_target="$(detect_tiber_target)" || {
  echo "unsupported-host-release-binary os=$(uname -s) arch=$(uname -m)" >&2
  exit 1
}
host_path="$(
  jq -r --arg target "$host_target" \
    '.binaries[] | select(.target == $target) | .path' \
    "$manifest"
)"
expected_host_path="dist/$host_target/tiber"
if [ "$host_path" != "$expected_host_path" ]; then
  echo "host-release-manifest-path-mismatch target=$host_target manifest_path=plugins/tiber/$host_path launcher_path=plugins/tiber/$expected_host_path" >&2
  exit 1
fi

if [ ! -x "$launcher" ]; then
  echo "missing-release-launcher path=plugins/tiber/bin/tiber" >&2
  exit 1
fi

if [ ! -s "$launcher" ]; then
  echo "invalid-release-launcher path=plugins/tiber/bin/tiber" >&2
  exit 1
fi

if [ ! -s "$checksums" ]; then
  echo "missing-release-checksums path=plugins/tiber/release-binaries.sha256" >&2
  exit 1
fi

manifest_paths="$(mktemp)"
checksum_paths="$(mktemp)"
smoke_repo=""
smoke_origin=""
smoke_remote=""
smoke_local=""
cleanup() {
  rm -f "$manifest_paths" "$checksum_paths"
  if [ -n "$smoke_repo" ]; then
    rm -rf "$smoke_repo"
  fi
  if [ -n "$smoke_origin" ]; then
    rm -rf "$smoke_origin"
  fi
  if [ -n "$smoke_remote" ]; then
    rm -rf "$smoke_remote"
  fi
  if [ -n "$smoke_local" ]; then
    rm -rf "$smoke_local"
  fi
}
trap cleanup EXIT

write_task_path_to_tasks_ref() {
  local repo="$1" index_name="$2" commit_message="$3" path="$4" title="$5"
  local index="$smoke_repo/$index_name"
  local blob tree commit
  blob="$(
    printf '%s\n' '---' "title: $title" 'blocked_by: []' 'blocks: []' 'tags: []' '---' |
      git -C "$repo" hash-object -w --stdin
  )"
  GIT_INDEX_FILE="$index" git -C "$repo" read-tree tasks
  GIT_INDEX_FILE="$index" git -C "$repo" update-index --add --cacheinfo 100644 "$blob" "$path"
  tree="$(GIT_INDEX_FILE="$index" git -C "$repo" write-tree)"
  commit="$(git -C "$repo" commit-tree "$tree" -p tasks -m "$commit_message")"
  git -C "$repo" update-ref refs/heads/tasks "$commit"
}

run_mcp_stdio() {
  timeout --kill-after=5s 30s "$launcher" mcp stdio
}

jq -r '.binaries[].path' "$manifest" | sort >"$manifest_paths"
awk '{ print $2 }' "$checksums" | sort >"$checksum_paths"
if ! cmp -s "$manifest_paths" "$checksum_paths"; then
  echo "release-checksum-paths-mismatch path=plugins/tiber/release-binaries.sha256" >&2
  exit 1
fi

jq -r '.binaries[] | "\(.target)\t\(.path)"' "$manifest" |
  while IFS=$'\t' read -r target binary_path; do
    absolute_binary_path="$root/plugins/tiber/$binary_path"
    if [ ! -x "$absolute_binary_path" ]; then
      echo "missing-release-binary target=$target path=plugins/tiber/$binary_path" >&2
      exit 1
    fi
    if [ ! -s "$absolute_binary_path" ]; then
      echo "invalid-release-binary target=$target path=plugins/tiber/$binary_path" >&2
      exit 1
    fi
    expected_hash="$(
      awk -v path="$binary_path" '$2 == path { print $1 }' "$checksums"
    )"
    if [ -z "$expected_hash" ]; then
      echo "missing-release-checksum target=$target path=plugins/tiber/$binary_path" >&2
      exit 1
    fi
    actual_hash="$(sha256sum "$absolute_binary_path" | awk '{ print $1 }')"
    if [ "$actual_hash" != "$expected_hash" ]; then
      echo "stale-release-binary target=$target path=plugins/tiber/$binary_path" >&2
      exit 1
    fi
  done

smoke_repo="$(mktemp -d)"

git -C "$smoke_repo" init >/dev/null
git -C "$smoke_repo" config user.email tiber-release-smoke@example.invalid
git -C "$smoke_repo" config user.name "Tiber Release Smoke"
git -C "$smoke_repo" config commit.gpgsign false

(
  cd "$smoke_repo"
  codex_sandbox_output="$("$launcher" codex-sandbox --dry-run)"
  if ! printf '%s\n' "$codex_sandbox_output" | grep -Fq 'Tiber Codex sandbox setup preview'; then
    echo "invalid-host-release-codex-sandbox-output target=$host_target path=plugins/tiber/bin/tiber" >&2
    exit 1
  fi
  if ! printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' |
    run_mcp_stdio |
    grep -Fq '"name":"tiber.codex_sandbox_setup"'; then
    echo "invalid-host-release-mcp-tools target=$host_target path=plugins/tiber/bin/tiber" >&2
    exit 1
  fi
  if ! printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' |
    run_mcp_stdio |
    grep -Fq '"name":"tiber.conflict_show"'; then
    echo "invalid-host-release-mcp-conflict-tool target=$host_target path=plugins/tiber/bin/tiber" >&2
    exit 1
  fi
  if ! printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' |
    run_mcp_stdio |
    grep -Fq '"name":"tiber.conflict_resolve"'; then
    echo "invalid-host-release-mcp-conflict-resolve-tool target=$host_target path=plugins/tiber/bin/tiber" >&2
    exit 1
  fi
  if ! printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' |
    run_mcp_stdio |
    grep -Fq '"name":"tiber.conflict_resolve_many"'; then
    echo "invalid-host-release-mcp-conflict-resolve-many-tool target=$host_target path=plugins/tiber/bin/tiber" >&2
    exit 1
  fi
  "$launcher" init >/dev/null
  conflict_output="$("$launcher" conflict show order.md)"
  if ! printf '%s\n' "$conflict_output" | jq -e '.path == "order.md" and (.local | type) == "string" and (.remote == null)' >/dev/null; then
    echo "invalid-host-release-conflict-output target=$host_target path=plugins/tiber/bin/tiber output=$conflict_output" >&2
    exit 1
  fi
  create_output="$("$launcher" create "Release smoke")"
  if ! printf '%s\n' "$create_output" | grep -Eq '^created [0-9]{8}-[abcdefghijkmnpqrstuvwxyz23456789]{4}-release-smoke$'; then
    echo "invalid-host-release-create-output target=$host_target path=plugins/tiber/bin/tiber output=$create_output" >&2
    exit 1
  fi
)

smoke_origin="$smoke_repo-origin.git"
smoke_remote="$smoke_repo-remote"
smoke_local="$smoke_repo-local"
mkdir -p "$smoke_origin"
git -C "$smoke_origin" init --bare >/dev/null
git clone "$smoke_origin" "$smoke_remote" >/dev/null 2>&1
git -C "$smoke_remote" config user.email tiber-release-smoke@example.invalid
git -C "$smoke_remote" config user.name "Tiber Release Smoke"
git -C "$smoke_remote" config commit.gpgsign false
printf '# release smoke\n' >"$smoke_remote/README.md"
git -C "$smoke_remote" add README.md
git -C "$smoke_remote" commit -m "Initial commit" >/dev/null
git -C "$smoke_remote" push origin HEAD:main >/dev/null
git -C "$smoke_origin" symbolic-ref HEAD refs/heads/main
git clone "$smoke_origin" "$smoke_local" >/dev/null 2>&1
git -C "$smoke_local" config user.email tiber-release-smoke@example.invalid
git -C "$smoke_local" config user.name "Tiber Release Smoke"
git -C "$smoke_local" config commit.gpgsign false
(
  cd "$smoke_remote"
  "$launcher" init >/dev/null
  create_output="$("$launcher" create "Release resolver smoke")"
  smoke_ref="${create_output#created }"
  "$launcher" sync >/dev/null
  git -C "$smoke_local" fetch origin tasks:tasks >/dev/null
  write_task_path_to_tasks_ref \
    "$smoke_remote" remote-index "Remote resolver smoke" "backlog/$smoke_ref.md" \
    "Remote release resolver smoke"
  git -C "$smoke_remote" push origin refs/heads/tasks:refs/heads/tasks >/dev/null
  cd "$smoke_local"
  write_task_path_to_tasks_ref \
    "$smoke_local" local-index "Local resolver smoke" "backlog/$smoke_ref.md" \
    "Local release resolver smoke"
  if "$launcher" sync >/dev/null 2>&1; then
    echo "invalid-host-release-resolver-setup target=$host_target path=plugins/tiber/bin/tiber" >&2
    exit 1
  fi
  conflict_output="$("$launcher" conflict show "backlog/$smoke_ref.md")"
  if ! printf '%s\n' "$conflict_output" |
    jq -e --arg path "backlog/$smoke_ref.md" '
      .path == $path
      and (.local | contains("title: Local release resolver smoke"))
      and (.remote | contains("title: Remote release resolver smoke"))
    ' >/dev/null; then
    echo "invalid-host-release-conflict-show target=$host_target path=plugins/tiber/bin/tiber output=$conflict_output" >&2
    exit 1
  fi
  "$launcher" conflict resolve "backlog/$smoke_ref.md" --local >/dev/null
  resolved_task="$(git -C "$smoke_origin" show "tasks:backlog/$smoke_ref.md")"
  if ! printf '%s\n' "$resolved_task" | grep -Fq 'title: Local release resolver smoke'; then
    echo "invalid-host-release-conflict-resolve target=$host_target path=plugins/tiber/bin/tiber" >&2
    exit 1
  fi
  cd "$smoke_remote"
  git -C "$smoke_remote" fetch origin tasks:refs/remotes/origin/tasks >/dev/null
  git -C "$smoke_remote" update-ref refs/heads/tasks refs/remotes/origin/tasks
  create_output="$("$launcher" create "Release MCP many first")"
  smoke_first="${create_output#created }"
  create_output="$("$launcher" create "Release MCP many second")"
  smoke_second="${create_output#created }"
  "$launcher" sync >/dev/null
  cd "$smoke_local"
  "$launcher" list >/dev/null
  cd "$smoke_remote"
  write_task_path_to_tasks_ref \
    "$smoke_remote" remote-many-index "Remote MCP many resolver smoke" "backlog/$smoke_first.md" \
    "Remote release MCP many first"
  write_task_path_to_tasks_ref \
    "$smoke_remote" remote-many-index "Remote MCP many resolver smoke" "backlog/$smoke_second.md" \
    "Remote release MCP many second"
  git -C "$smoke_remote" push origin refs/heads/tasks:refs/heads/tasks >/dev/null
  cd "$smoke_local"
  write_task_path_to_tasks_ref \
    "$smoke_local" local-many-index "Local MCP many resolver smoke" "backlog/$smoke_first.md" \
    "Local release MCP many first"
  write_task_path_to_tasks_ref \
    "$smoke_local" local-many-index "Local MCP many resolver smoke" "backlog/$smoke_second.md" \
    "Local release MCP many second"
  mcp_many_output="$(
    printf '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"tiber.conflict_resolve_many","arguments":{"resolutions":[{"path":"backlog/%s.md","side":"local"},{"path":"backlog/%s.md","side":"remote"}]}}}\n' "$smoke_first" "$smoke_second" |
      run_mcp_stdio
  )"
  if ! printf '%s\n' "$mcp_many_output" | grep -Fq "resolved \\\"backlog/$smoke_first.md\\\" side=local"; then
    echo "invalid-host-release-conflict-resolve-many-local target=$host_target path=plugins/tiber/bin/tiber output=$mcp_many_output" >&2
    exit 1
  fi
  if ! printf '%s\n' "$mcp_many_output" | grep -Fq "resolved \\\"backlog/$smoke_second.md\\\" side=remote"; then
    echo "invalid-host-release-conflict-resolve-many-remote target=$host_target path=plugins/tiber/bin/tiber output=$mcp_many_output" >&2
    exit 1
  fi
  resolved_first="$(git -C "$smoke_origin" show "tasks:backlog/$smoke_first.md")"
  if ! printf '%s\n' "$resolved_first" | grep -Fq 'title: Local release MCP many first'; then
    echo "invalid-host-release-conflict-resolve-many-first target=$host_target path=plugins/tiber/bin/tiber" >&2
    exit 1
  fi
  resolved_second="$(git -C "$smoke_origin" show "tasks:backlog/$smoke_second.md")"
  if ! printf '%s\n' "$resolved_second" | grep -Fq 'title: Remote release MCP many second'; then
    echo "invalid-host-release-conflict-resolve-many-second target=$host_target path=plugins/tiber/bin/tiber" >&2
    exit 1
  fi
)
