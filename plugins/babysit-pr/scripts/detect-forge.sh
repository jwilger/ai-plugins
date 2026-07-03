#!/usr/bin/env bash
# Classify the forge for a repository from its `origin` remote URL.
#
#   detect-forge.sh [repo-dir]
#
# Prints one of: github | gitlab | forgejo. Self-hosted remotes default to
# forgejo; override the detected forge in local automation config when needed.
set -euo pipefail

dir="${1:-.}"
url="$(git -C "$dir" remote get-url origin 2>/dev/null || true)"

case "$url" in
*github.com*) echo github ;;
*gitlab.com* | *gitlab.*) echo gitlab ;;
*) echo forgejo ;;
esac
