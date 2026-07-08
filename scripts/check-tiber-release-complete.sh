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
cleanup() {
  rm -f "$manifest_paths" "$checksum_paths"
  if [ -n "$smoke_repo" ]; then
    rm -rf "$smoke_repo"
  fi
}
trap cleanup EXIT
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
    "$launcher" mcp stdio |
    grep -Fq '"name":"tiber.codex_sandbox_setup"'; then
    echo "invalid-host-release-mcp-tools target=$host_target path=plugins/tiber/bin/tiber" >&2
    exit 1
  fi
  "$launcher" init >/dev/null
  create_output="$("$launcher" create "Release smoke")"
  if ! printf '%s\n' "$create_output" | grep -Eq '^created [0-9]{8}-[abcdefghijkmnpqrstuvwxyz23456789]{4}-release-smoke$'; then
    echo "invalid-host-release-create-output target=$host_target path=plugins/tiber/bin/tiber output=$create_output" >&2
    exit 1
  fi
)
