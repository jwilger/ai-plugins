#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd -P)"
  RELEASE_BUILD="$ROOT/scripts/build-development-discipline-release-all.sh"
}

@test "development-discipline release builder does not require a macOS forge artifact" {
  run env -u DEVELOPMENT_DISCIPLINE_MACOS_UNIVERSAL_BINARY \
    -u DEVELOPMENT_DISCIPLINE_MACOS_ARTIFACT_ENVELOPE \
    bash "$RELEASE_BUILD"

  [ "$status" -eq 0 ]
  [[ "$output" != *"release-macos-artifact-required=true"* ]]
}
