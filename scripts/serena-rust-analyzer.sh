#!/usr/bin/env bash
set -euo pipefail

# Serena starts outside the devshell. Reuse the pinned project environment,
# keeping shell-hook diagnostics off the language server's stdout protocol.
serena_repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$serena_repo_root"
serena_nix_env="$(nix print-dev-env)"
eval "$serena_nix_env" >&2
exec "${AI_PLUGINS_RUST_ANALYZER_BIN:?flake-selected rust-analyzer path unavailable}" "$@"
