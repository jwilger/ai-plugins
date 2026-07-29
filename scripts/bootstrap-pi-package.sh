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
    tiber_target=x86_64-unknown-linux-gnu
    discipline_target=x86_64-unknown-linux-musl
    ;;
  Linux:aarch64 | Linux:arm64)
    tiber_target=aarch64-unknown-linux-gnu
    discipline_target=aarch64-unknown-linux-musl
    ;;
  Darwin:x86_64)
    tiber_target=x86_64-apple-darwin
    discipline_target=x86_64-apple-darwin
    ;;
  Darwin:arm64 | Darwin:aarch64)
    tiber_target=aarch64-apple-darwin
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

verify_component tiber "$tiber_target" tiber
verify_component development-discipline "$discipline_target" development-discipline-mcp
printf 'development_system.pi_bootstrap_ready pi_compatibility=%s target=%s/%s\n' \
  "$(jq -r .piCompatibility "$root/plugins/development-system/package.json")" \
  "$tiber_target" "$discipline_target"
