#!/usr/bin/env bash
set -euo pipefail

# Git exports repository selectors to hooks. Fixture Git commands must select
# their own repositories, while the parent commit retains its signing context.
git_local_env_names="$(git rev-parse --local-env-vars)"
while IFS= read -r git_local_env_name; do
  if [[ ! "$git_local_env_name" =~ ^GIT_[A-Z0-9_]+$ ]]; then
    printf '%s\n' 'pre_commit.invalid_git_local_environment_name' >&2
    exit 2
  fi
  unset -- "$git_local_env_name"
done <<< "$git_local_env_names"

project_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$project_root"
just validate-marketplace github-actions
bash tiber/scripts/check-lint-policy.sh
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target}/tiber-harness" cargo fmt --manifest-path tiber/Cargo.toml --all --check
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target}/tiber-harness" cargo clippy --manifest-path tiber/Cargo.toml --workspace --all-targets --all-features -- -D warnings
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target}/tiber-harness" cargo test --manifest-path tiber/Cargo.toml --workspace --all-features
cargo fmt --manifest-path plugins/development-system/components/tiber/rust/Cargo.toml --all --check
cargo clippy --manifest-path plugins/development-system/components/tiber/rust/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path plugins/development-system/components/tiber/rust/Cargo.toml --workspace --lib
cargo fmt --manifest-path plugins/development-system/components/development-discipline/rust/Cargo.toml --all --check
cargo clippy --manifest-path plugins/development-system/components/development-discipline/rust/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path plugins/development-system/components/development-discipline/rust/Cargo.toml --bin development-discipline-mcp -- --test-threads=1
