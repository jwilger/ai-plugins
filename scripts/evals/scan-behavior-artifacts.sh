#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
scanner="$root/scripts/evals/scan-code-quality-secrets.mjs"
metadata=""
private_roots=()
secret_files=()
inputs=()

while [ "$#" -gt 0 ]; do
  case "$1" in
    --metadata)
      [ "$#" -ge 2 ] || {
        echo "provider eval artifact scan configuration is invalid" >&2
        exit 2
      }
      metadata="$2"
      shift 2
      ;;
    --private-root)
      [ "$#" -ge 2 ] || {
        echo "provider eval artifact scan configuration is invalid" >&2
        exit 2
      }
      private_roots+=("$2")
      shift 2
      ;;
    --secret-file)
      [ "$#" -ge 2 ] || {
        echo "provider eval artifact scan configuration is invalid" >&2
        exit 2
      }
      secret_files+=("$2")
      shift 2
      ;;
    --)
      shift
      inputs+=("$@")
      break
      ;;
    -*)
      echo "provider eval artifact scan configuration is invalid" >&2
      exit 2
      ;;
    *)
      inputs+=("$1")
      shift
      ;;
  esac
done

if [ "${#private_roots[@]}" -eq 0 ] || [ "${#inputs[@]}" -eq 0 ]; then
  echo "provider eval artifact scan configuration is invalid" >&2
  exit 2
fi

canonical_roots=()
for private_root in "${private_roots[@]}"; do
  canonical_root="$(realpath -e -- "$private_root" 2>/dev/null)" || {
    echo "provider eval artifact path is not private" >&2
    exit 2
  }
  if [ ! -d "$canonical_root" ] || [ -L "$private_root" ] ||
    [ "$((8#$(stat -c %a -- "$canonical_root") & 8#077))" -ne 0 ]; then
    echo "provider eval artifact path is not private" >&2
    exit 2
  fi
  canonical_roots+=("$canonical_root")
done

canonical_inputs=()
for input in "${inputs[@]}"; do
  canonical_input="$(realpath -e -- "$input" 2>/dev/null)" || {
    echo "provider eval artifact path is invalid" >&2
    exit 2
  }
  covered=0
  for canonical_root in "${canonical_roots[@]}"; do
    case "$canonical_input" in
      "$canonical_root"/*) covered=1 ;;
    esac
  done
  if [ "$covered" -ne 1 ]; then
    echo "provider eval artifact path is invalid" >&2
    exit 2
  fi
  canonical_inputs+=("$canonical_input")
done

selected_claude=1
selected_codex=1
if [ -n "$metadata" ]; then
  canonical_metadata="$(realpath -e -- "$metadata" 2>/dev/null)" || {
    echo "provider eval artifact scan metadata is invalid" >&2
    exit 2
  }
  provider_selection="$({
    jq -er '
      if (.providerCompositions | type) != "array" or
         (.usesCodexGrader | type) != "boolean" then
        error("invalid provider metadata")
      else
        [
          any(.providerCompositions[]; .provider == "anthropic:claude-agent-sdk"),
          (any(.providerCompositions[]; .provider == "openai:codex-sdk") or .usesCodexGrader)
        ] | @tsv
      end
    ' "$canonical_metadata"
  } 2>/dev/null)" || {
    echo "provider eval artifact scan metadata is invalid" >&2
    exit 2
  }
  IFS=$'\t' read -r selected_claude selected_codex <<<"$provider_selection"
  [ "$selected_claude" = "true" ] || selected_claude=0
  [ "$selected_codex" = "true" ] || selected_codex=0
fi

scan_options=()
for secret_file in "${secret_files[@]}"; do
  scan_options+=(--secret-file "$(realpath -m -- "$secret_file")")
done
if [ "$selected_claude" != "0" ]; then
  scan_options+=(
    --secret-env ANTHROPIC_API_KEY
    --secret-env CLAUDE_CODE_OAUTH_TOKEN
  )
fi
if [ "$selected_codex" != "0" ]; then
  scan_options+=(
    --secret-env OPENAI_API_KEY
    --secret-env CODEX_API_KEY
  )
fi

if ! node "$scanner" "${scan_options[@]}" "${canonical_inputs[@]}" >/dev/null; then
  echo "provider eval artifacts failed secret scanning" >&2
  exit 86
fi
