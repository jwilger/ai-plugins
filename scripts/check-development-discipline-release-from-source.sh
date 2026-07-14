#!/usr/bin/env bash
set -euo pipefail

root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
plugin_root="$root/plugins/development-discipline"
manifest="$plugin_root/rust/Cargo.toml"
toolchain_file="$plugin_root/rust/rust-toolchain.toml"
toolchain="$(awk -F'"' '/^channel = "/ { print $2; exit }' "$toolchain_file")"
if [ -z "$toolchain" ]; then
  echo "release-toolchain-channel-missing path=plugins/development-discipline/rust/rust-toolchain.toml" >&2
  exit 1
fi
export RUSTUP_HOME="${RUSTUP_HOME:-$root/.dependencies/rustup}"
export CARGO_HOME="${CARGO_HOME:-$root/.dependencies/cargo}"
export PATH="$CARGO_HOME/bin:$PATH"
target_dir="$root/.dependencies/cargo-target/development-discipline-release"
source_binary="$target_dir/release/development-discipline-mcp"
flow_script="$root/scripts/tests/development-discipline-mcp-flow.mjs"
source_fingerprint="$(
  cd "$plugin_root/rust"
  sha256sum Cargo.toml Cargo.lock rust-toolchain.toml src/main.rs | sha256sum | awk '{ print $1 }'
)"

if ! rustup toolchain list | grep -Eq "^${toolchain}(-| )"; then
  rustup toolchain install "$toolchain" --profile minimal
fi

DEVELOPMENT_DISCIPLINE_SOURCE_FINGERPRINT="$source_fingerprint" \
  CARGO_TARGET_DIR="$target_dir" \
  rustup run "$toolchain" cargo build --manifest-path "$manifest" --release

source "$plugin_root/scripts/detect-target.sh"
release_target="$(detect_development_discipline_target)"
dist_binary="$plugin_root/dist/$release_target/development-discipline-mcp"

source_output="$(mktemp)"
dist_output="$(mktemp)"
project_root="$(mktemp -d)"
trap 'rm -rf "$source_output" "$dist_output" "$project_root"' EXIT

mkdir -p "$project_root/.development-discipline"
git -C "$project_root" init --quiet
git -C "$project_root" config user.name "Final Review Fixture"
git -C "$project_root" config user.email "final-review-fixture@example.test"
git -C "$project_root" commit --allow-empty --quiet -m "Initialize final-review fixture"
mkdir -p "$project_root/src"
printf '%s\n' 'fixture change' >"$project_root/src/new.rs"
printf '%s\n' \
  '[final_review.models]' \
  'pre_filter = "config-pre"' \
  'lens_review = "config-review"' \
  'post_filter = "config-post"' \
  'verifier = "config-verify"' \
  >"$project_root/.development-discipline/final-review.toml"

run_flow() {
  local binary="$1" output="$2"
  FINAL_REVIEW_TEST_PROJECT_ROOT="$project_root" \
    FINAL_REVIEW_ROUTING_PROJECT_ROOT="$root" \
    node "$flow_script" "$binary" >"$output"
}

run_flow "$source_binary" "$source_output"
run_flow "$dist_binary" "$dist_output"

if ! cmp "$source_output" "$dist_output" >/dev/null; then
  diff -u "$source_output" "$dist_output" || true
  echo "development-discipline-release-parity-mismatch=true" >&2
  exit 1
fi

jq -s -e '
  def response($id): map(select(.id == $id)) | first;
  (response(3).result.content[0].text | fromjson
    | .model_roles.pre_filter == "explicit-pre"
      and .model_roles.lens_review == "config-review"
      and .model_role_sources.lens_review == "project_toml_config")
  and (response(4).error.code == -32602
    and response(4).error.message == "review_state_out_of_sync=true")
  and (response(5).result.content[0].text | fromjson
    | (.actionable | map(.id)) == ["launcher-real"]
      and (.out_of_scope | map(.id)) == ["launcher-stale"])
  and (response(6).result.content[0].text | fromjson
    | .transition_status == "verifier_required"
      and .verifier_assignment.subagent_key == "bats-review:1:verifier"
      and .verifier_assignment.model_role == "config-verify")
  and (response(7).result.content[0].text | fromjson
    | .transition_status == "advanced"
      and (.filtered.verifier_rejected | map(.id)) == ["launcher-real"]
      and .state.clean_streak == 1)
  and (response(9).result.content[0].text | fromjson
    | .complete == true
      and .state.clean_streak == 3
      and (.next_assignments | length) == 0)
  and (response(11).error.code == -32602
    and response(11).error.message == "review_session_complete=true")
  and (response(12).result.content[0].text | fromjson
    | .model_roles.pre_filter == "gpt-5.6-luna"
      and .model_roles.lens_review == "gpt-5.6-terra"
      and .model_roles.post_filter == "gpt-5.6-luna"
      and .model_roles.verifier == "gpt-5.6-sol")
' "$dist_output" >/dev/null
