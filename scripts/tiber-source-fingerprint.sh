#!/usr/bin/env bash
set -euo pipefail

root="${1:-$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)}"
source_root="plugins/development-system/components/tiber/rust"

cd "$root"
{
  printf '%s\0' "$source_root/Cargo.toml" "$source_root/Cargo.lock"
  find "$source_root/crates" -type f \
    \( -name Cargo.toml -o -name '*.rs' \) -print0
} |
  LC_ALL=C sort -z |
  xargs -0 sha256sum |
  sha256sum |
  awk '{ print $1 }'
