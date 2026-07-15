#!/usr/bin/env bash
set -euo pipefail

marketplace_name="ai-plugins"
core_plugins=(
  engineering-standards
  development-discipline
  advisor
)
representative_skills=(
  engineering-standards:engineering-standards
  development-discipline:test-driven-development
  development-discipline:verification-before-completion
  advisor:advisor
)

usage() {
  cat <<'EOF'
Usage: scripts/codex-quality-core.sh install [--with-agentic]

Install or refresh the Codex quality-core plugins from this checkout, then
verify that their representative skills are model-visible in a clean temporary
downstream Git repository. Add --with-agentic for AI-system projects that also
provide the Promptfoo tooling required by agentic-systems-engineering.
EOF
}

require_command() {
  local command_name="$1"

  if ! command -v "$command_name" >/dev/null 2>&1; then
    printf 'missing required command: %s\n' "$command_name" >&2
    exit 2
  fi
}

configured_marketplace_root() {
  local marketplaces_json="$1"

  jq -r --arg name "$marketplace_name" \
    '.marketplaces[]? | select(.name == $name) | .root' \
    <<<"$marketplaces_json"
}

assert_marketplace_is_current() {
  local marketplaces_json="$1"
  local configured_root

  configured_root="$(configured_marketplace_root "$marketplaces_json")"
  if [ -z "$configured_root" ]; then
    return 1
  fi

  if [ "$configured_root" != "$root" ]; then
    printf "Codex marketplace '%s' points to a different checkout.\n" "$marketplace_name" >&2
    printf '  configured: %s\n' "$configured_root" >&2
    printf '  requested:  %s\n' "$root" >&2
    printf "Resolve the source deliberately with 'codex plugin marketplace remove %s', then rerun this command.\n" "$marketplace_name" >&2
    exit 2
  fi
}

expected_plugin_version() {
  local plugin="$1"

  jq -er '.version' "$root/plugins/$plugin/.codex-plugin/plugin.json"
}

assert_plugins_installed() {
  local plugins_json="$1"
  local plugin expected_version actual_version enabled

  for plugin in "${core_plugins[@]}"; do
    expected_version="$(expected_plugin_version "$plugin")"
    actual_version="$(
      jq -r --arg plugin "$plugin" --arg marketplace "$marketplace_name" \
        '.installed[]? | select(.name == $plugin and .marketplaceName == $marketplace) | .version' \
        <<<"$plugins_json"
    )"
    enabled="$(
      jq -r --arg plugin "$plugin" --arg marketplace "$marketplace_name" \
        '.installed[]? | select(.name == $plugin and .marketplaceName == $marketplace) | .enabled' \
        <<<"$plugins_json"
    )"

    if [ -z "$actual_version" ]; then
      printf "missing Codex plugin: %s@%s; rerun '%s install'.\n" \
        "$plugin" "$marketplace_name" "$0" >&2
      exit 1
    fi
    if [ "$actual_version" != "$expected_version" ]; then
      printf 'stale Codex plugin: %s@%s has version %s; expected %s; rerun '\''%s install'\''.\n' \
        "$plugin" "$marketplace_name" "$actual_version" "$expected_version" "$0" >&2
      exit 1
    fi
    if [ "$enabled" != "true" ]; then
      printf "disabled Codex plugin: %s@%s; rerun '%s install' and enable the plugin.\n" \
        "$plugin" "$marketplace_name" "$0" >&2
      exit 1
    fi
  done
}

assert_skills_model_visible() {
  local downstream="$1"
  local prompt_json skill

  prompt_json="$(
    codex -C "$downstream" debug prompt-input \
      'Plan a small feature and identify the installed workflows that should guide implementation and verification.'
  )"

  for skill in "${representative_skills[@]}"; do
    if ! jq -e --arg skill "$skill" 'any(.. | strings; contains($skill))' \
      >/dev/null <<<"$prompt_json"; then
      printf "installed skill is not model-visible: %s; rerun '%s install', then start a new Codex thread.\n" \
        "$skill" "$0" >&2
      exit 1
    fi
  done
}

install_quality_core() {
  local marketplaces_json plugins_json plugin downstream

  marketplaces_json="$(codex plugin marketplace list --json)"
  if ! assert_marketplace_is_current "$marketplaces_json"; then
    codex plugin marketplace add "$root" --json >/dev/null
  fi

  for plugin in "${core_plugins[@]}"; do
    codex plugin add "$plugin@$marketplace_name" --json >/dev/null
  done

  plugins_json="$(codex plugin list --available --json)"
  assert_plugins_installed "$plugins_json"

  downstream="$(mktemp -d "${TMPDIR:-/tmp}/ai-plugins-codex-smoke.XXXXXX")"
  trap 'rm -rf "$downstream"' RETURN
  git -C "$downstream" init -q
  assert_skills_model_visible "$downstream"

  printf 'Codex quality core is installed and model-visible from %s.\n' "$root"
  printf 'Start a new Codex thread in the downstream repository before relying on refreshed plugin behavior.\n'
}

if [ "$#" -eq 1 ] && { [ "$1" = "--help" ] || [ "$1" = "-h" ]; }; then
  usage
  exit 0
fi

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"

if [ "$#" -lt 1 ] || [ "$#" -gt 2 ]; then
  usage >&2
  exit 2
fi

if [ "$1" != "install" ]; then
  printf 'unknown command: %s\n' "$1" >&2
  usage >&2
  exit 2
fi

if [ "$#" -eq 2 ]; then
  if [ "$2" != "--with-agentic" ]; then
    printf 'unknown option: %s\n' "$2" >&2
    usage >&2
    exit 2
  fi
  core_plugins+=(agentic-systems-engineering)
  representative_skills+=(agentic-systems-engineering:agentic-systems-engineering)
fi

require_command codex
require_command git
require_command jq

case "$1" in
  install) install_quality_core ;;
  *)
    printf 'unknown command: %s\n' "$1" >&2
    usage >&2
    exit 2
    ;;
esac
