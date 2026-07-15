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
install_option=""
downstream_arg=""
with_agentic=0

usage() {
  cat <<'EOF'
Usage:
  scripts/codex-quality-core.sh install [--with-agentic]
  scripts/codex-quality-core.sh check [--with-agentic] [DOWNSTREAM]

install  Add or refresh the Codex quality-core plugins from this checkout.
check    Read only: verify plugin state and model visibility in a clean
         temporary downstream Git repository.

Add --with-agentic for AI-system projects that also provide the Promptfoo
tooling required by agentic-systems-engineering.
EOF
}

require_command() {
  local command_name="$1"

  if ! command -v "$command_name" >/dev/null 2>&1; then
    printf 'missing required command: %s\n' "$command_name" >&2
    exit 2
  fi
}

parse_configured_marketplace_root() {
  local marketplaces_json="$1"

  if ! configured_marketplace_root_value="$(
    jq -ser --arg name "$marketplace_name" '
      def nonempty_string: type == "string" and length > 0;

      select(length == 1)
      | .[0]
      | select(type == "object")
      | .marketplaces as $marketplaces
      | select(($marketplaces | type) == "array")
      | select(all(
          $marketplaces[];
          type == "object"
            and (.name? | nonempty_string)
            and (.root? | nonempty_string)
        ))
      | select(
          ([$marketplaces[].name] | unique | length)
            == ($marketplaces | length)
        )
      | [$marketplaces[] | select(.name == $name)] as $matches
      | if ($matches | length) == 0 then "" else $matches[0].root end
    ' <<<"$marketplaces_json"
  )"; then
    printf 'unsupported Codex marketplace schema; this helper was validated with Codex CLI 0.144.x (tested with 0.144.4). No marketplace or plugin changes were made.\n' >&2
    exit 2
  fi
}

assert_marketplace_is_current() {
  local marketplaces_json="$1"
  local configured_root

  parse_configured_marketplace_root "$marketplaces_json"
  configured_root="$configured_marketplace_root_value"
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

validate_plugin_state_json() {
  local plugins_json="$1"

  if ! validated_plugins_json="$(
    jq -sce '
      def nonempty_string: type == "string" and length > 0;
      def valid_plugin:
        type == "object"
          and (.name? | nonempty_string)
          and (.marketplaceName? | nonempty_string)
          and (.version? | nonempty_string)
          and ((.installed? | type) == "boolean")
          and ((.enabled? | type) == "boolean");

      select(length == 1)
      | .[0]
      | select(type == "object")
      | select((.installed? | type) == "array")
      | select((.available? | type) == "array")
      | select(all(.installed[]; valid_plugin and .installed))
      | select(all(.available[]; valid_plugin and (.installed | not)))
      | (.installed + .available) as $plugins
      | select(
          ($plugins | map([.marketplaceName, .name]) | unique | length)
            == ($plugins | length)
        )
    ' <<<"$plugins_json"
  )"; then
    printf 'unsupported Codex plugin state schema; this helper was validated with Codex CLI 0.144.x (tested with 0.144.4). Use compatible Codex and ai-plugins versions before rerunning.\n' >&2
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
      printf "missing Codex plugin: %s@%s; rerun '%s install%s'.\n" \
        "$plugin" "$marketplace_name" "$0" "$install_option" >&2
      exit 1
    fi
    if [ "$actual_version" != "$expected_version" ]; then
      printf 'stale Codex plugin: %s@%s has version %s; expected %s; rerun '\''%s install%s'\''.\n' \
        "$plugin" "$marketplace_name" "$actual_version" "$expected_version" "$0" "$install_option" >&2
      exit 1
    fi
    if [ "$enabled" != "true" ]; then
      printf "disabled Codex plugin: %s@%s; rerun '%s install%s' to re-enable it.\n" \
        "$plugin" "$marketplace_name" "$0" "$install_option" >&2
      exit 1
    fi
  done
}

assert_skills_model_visible() {
  local downstream="$1"
  local prompt_json skill skills_registry

  prompt_json="$(
    codex -C "$downstream" -c 'developer_instructions=""' debug prompt-input \
      'Plan a small feature and identify the installed workflows that should guide implementation and verification.'
  )"

  if ! skills_registry="$(
    jq -ser '
      select(length == 1)
      | .[0]
      | select(type == "array")
      | [
        .[]?
        | select(.type == "message" and .role == "developer")
        | (.content? // []) as $content
        | select(($content | type) == "array")
        | select(any(
            $content[]?;
            .type == "input_text"
              and ((.text? // "") | startswith("<permissions instructions>"))
          ))
        | select(any(
            $content[]?;
            .type == "input_text"
              and ((.text? // "") | startswith("<plugins_instructions>"))
          ))
        | [
            $content[]?
            | select(.type == "input_text")
            | (.text? // "")
            | select(startswith("<skills_instructions>"))
          ] as $skills
        | select(($skills | length) == 1)
        | $skills[0]
      ] as $registries
      | select(($registries | length) == 1)
      | $registries[0]
    ' <<<"$prompt_json"
  )"; then
    printf 'unsupported Codex prompt schema; this check was validated with Codex CLI 0.144.x (tested with 0.144.4). Update Codex or this checkout, then rerun the command.\n' >&2
    exit 2
  fi

  for skill in "${representative_skills[@]}"; do
    if ! jq -ne --arg registry "$skills_registry" --arg skill "$skill" '
      any($registry | split("\n")[]; startswith("- " + $skill + ":"))
    ' >/dev/null; then
      printf "installed skill is not model-visible: %s; rerun '%s install%s', then start a new Codex thread.\n" \
        "$skill" "$0" "$install_option" >&2
      exit 1
    fi
  done
}

install_quality_core() {
  local marketplaces_json plugins_json plugin marketplace_missing

  marketplaces_json="$(codex plugin marketplace list --json)"
  marketplace_missing=0
  if ! assert_marketplace_is_current "$marketplaces_json"; then
    marketplace_missing=1
  fi

  plugins_json="$(codex plugin list --available --json)"
  validate_plugin_state_json "$plugins_json"

  if [ "$marketplace_missing" -eq 1 ]; then
    codex plugin marketplace add "$root" --json >/dev/null
  fi

  for plugin in "${core_plugins[@]}"; do
    codex plugin add "$plugin@$marketplace_name" --json >/dev/null
  done

  check_quality_core
  printf 'Start a new Codex thread in the downstream repository before relying on refreshed plugin behavior.\n'
}

check_quality_core() {
  local marketplaces_json plugins_json downstream owns_downstream

  marketplaces_json="$(codex plugin marketplace list --json)"
  if ! assert_marketplace_is_current "$marketplaces_json"; then
    printf "missing Codex marketplace: %s; rerun '%s install%s'.\n" \
      "$marketplace_name" "$0" "$install_option" >&2
    exit 1
  fi

  plugins_json="$(codex plugin list --available --json)"
  validate_plugin_state_json "$plugins_json"
  assert_plugins_installed "$validated_plugins_json"

  owns_downstream=0
  if [ -n "$downstream_arg" ]; then
    if [ ! -d "$downstream_arg" ] || ! git -C "$downstream_arg" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
      printf 'downstream path must be a Git repository: %s\n' "$downstream_arg" >&2
      exit 2
    fi
    downstream="$(cd "$downstream_arg" && pwd -P)"
  else
    downstream="$(mktemp -d "${TMPDIR:-/tmp}/ai-plugins-codex-smoke.XXXXXX")"
    owns_downstream=1
    trap 'rm -rf "$downstream"' EXIT
    git -C "$downstream" init -q
  fi
  assert_skills_model_visible "$downstream"
  if [ "$owns_downstream" -eq 1 ]; then
    rm -rf "$downstream"
    trap - EXIT
  fi

  printf 'Codex quality core is installed and model-visible from %s.\n' "$root"
}

if [ "$#" -eq 1 ] && { [ "$1" = "--help" ] || [ "$1" = "-h" ]; }; then
  usage
  exit 0
fi

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"

if [ "$#" -lt 1 ]; then
  usage >&2
  exit 2
fi

command_name="$1"
shift

if [ "$command_name" != "install" ] && [ "$command_name" != "check" ]; then
  printf 'unknown command: %s\n' "$command_name" >&2
  usage >&2
  exit 2
fi

while [ "$#" -gt 0 ]; do
  case "$1" in
    --with-agentic)
      if [ "$with_agentic" -eq 1 ]; then
        printf 'option specified more than once: --with-agentic\n' >&2
        usage >&2
        exit 2
      fi
      with_agentic=1
      ;;
    -*)
      printf 'unknown option: %s\n' "$1" >&2
      usage >&2
      exit 2
      ;;
    *)
      if [ "$command_name" = "install" ]; then
        printf 'unexpected argument for install: %s\n' "$1" >&2
        usage >&2
        exit 2
      fi
      if [ -n "$downstream_arg" ]; then
        printf 'too many downstream paths\n' >&2
        usage >&2
        exit 2
      fi
      downstream_arg="$1"
      ;;
  esac
  shift
done

if [ "$with_agentic" -eq 1 ]; then
  core_plugins+=(agentic-systems-engineering)
  representative_skills+=(agentic-systems-engineering:agentic-systems-engineering)
  install_option=" --with-agentic"
fi

require_command codex
require_command git
require_command jq

case "$command_name" in
  install) install_quality_core ;;
  check) check_quality_core ;;
  *)
    printf 'unknown command: %s\n' "$1" >&2
    usage >&2
    exit 2
    ;;
esac
