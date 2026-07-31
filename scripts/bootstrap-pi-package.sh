#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if [[ "${DEVELOPMENT_SYSTEM_BOOTSTRAP_SKIP_NPM:-0}" != 1 ]]; then
  "$root/scripts/evals/ensure-node-deps.sh" "$root"
fi

node "$root/scripts/sync-development-system-metadata.mjs" --check
node "$root/scripts/validate-pi-package.mjs"

case "$(uname -s):$(uname -m)" in
  Linux:x86_64)
    discipline_target=x86_64-unknown-linux-musl
    ;;
  Linux:aarch64 | Linux:arm64)
    discipline_target=aarch64-unknown-linux-musl
    ;;
  Darwin:x86_64)
    discipline_target=x86_64-apple-darwin
    ;;
  Darwin:arm64 | Darwin:aarch64)
    discipline_target=aarch64-apple-darwin
    ;;
  *)
    printf 'development_system.unsupported_platform os=%s arch=%s\n' "$(uname -s)" "$(uname -m)" >&2
    exit 2
    ;;
esac

verify_component() {
  local component=$1 target=$2 binary=$3
  local directory="$root/plugins/development-system/components/$component"
  local relative="dist/$target/$binary"
  [[ -x "$directory/$relative" ]] || {
    printf 'development_system.component_unavailable component=%s target=%s\n' "$component" "$target" >&2
    exit 2
  }
  (cd "$directory" && grep -F "  $relative" release-binaries.sha256 | sha256sum --check --status) || {
    printf 'development_system.component_checksum_failed component=%s target=%s\n' "$component" "$target" >&2
    exit 2
  }
}

verify_component development-discipline "$discipline_target" development-discipline-mcp
tool_policy="$(node "$root/plugins/development-system/bin/install-development-tool.mjs" install --json)" || {
  printf 'development_system.tool_install_failed tools=bd\n' >&2
  exit 2
}
bd_bin="$(TOOL_POLICY="$tool_policy" node -e '
  const policy = JSON.parse(process.env.TOOL_POLICY);
  const bd = policy.tools.find((tool) => tool.name === "bd");
  if (!bd?.executable || bd.status !== "compatible") process.exit(2);
  process.stdout.write(bd.executable);
')" || {
  printf 'development_system.tool_install_failed tools=bd\n' >&2
  exit 2
}
printf 'development_system.pi_bootstrap_ready pi_compatibility=%s target=%s beads=%s\n' \
  "$(jq -r .piCompatibility "$root/plugins/development-system/package.json")" \
  "$discipline_target" "$("$bd_bin" version | awk '{print $3}')"
