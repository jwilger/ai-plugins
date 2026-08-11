#!/usr/bin/env bash
set -euo pipefail

root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"

cd "$root"
{
  sha256sum \
    plugins/development-system/components/development-discipline/rust/Cargo.toml \
    plugins/development-system/components/development-discipline/rust/Cargo.lock \
    plugins/development-system/components/development-discipline/rust/rust-toolchain.toml \
    plugins/development-system/components/tiber/rust/Cargo.toml \
    plugins/development-system/components/tiber/rust/Cargo.lock
  find plugins/development-system/components/development-discipline/rust/src \
    plugins/development-system/components/tiber/rust/crates/tiber-git \
    plugins/development-system/components/tiber/rust/crates/tiber-core \
    -type f \( -name Cargo.toml -o -name '*.rs' \) -print0 |
    LC_ALL=C sort -z |
    xargs -0 sha256sum
} | sha256sum | awk '{ print $1 }'
