#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
}

@test "development-system npm payload contains every declared runtime resource" {
  run node "$ROOT/scripts/validate-development-system-npm-package.mjs"

  [ "$status" -eq 0 ]
  version="$(jq -r '.version' "$ROOT/plugins/development-system/package.json")"
  printf '%s' "$output" | jq -e --arg version "$version" '
    .ok == true and
    .name == "@jwilger/development-system-pi" and
    .version == $version and
    .files > 0 and
    .unpackedBytes < 104857600
  ' >/dev/null
}

@test "release versioning applies strict semantic increments" {
  run node --input-type=module - "$ROOT/scripts/version-development-system.mjs" <<'NODE'
import assert from "node:assert/strict";
import { pathToFileURL } from "node:url";
const module = await import(pathToFileURL(process.argv[2]));
assert.equal(module.nextVersion("1.2.3", "patch"), "1.2.4");
assert.equal(module.nextVersion("1.2.3", "minor"), "1.3.0");
assert.equal(module.nextVersion("1.2.3", "major"), "2.0.0");
assert.equal(module.nextVersion("1.2.3", "current"), "1.2.3");
assert.throws(() => module.nextVersion("1.2", "patch"));
assert.throws(() => module.nextVersion("1.2.3", "prerelease"));
NODE

  [ "$status" -eq 0 ]
}

@test "current-version recovery mode does not rewrite release metadata" {
  before="$(
    sha256sum \
      "$ROOT/plugins/development-system/package.json" \
      "$ROOT/.claude-plugin/marketplace.json" \
      "$ROOT/.agents/plugins/marketplace.json" \
      "$ROOT/README.md"
  )"

  version="$(jq -r '.version' "$ROOT/plugins/development-system/package.json")"
  run node "$ROOT/scripts/version-development-system.mjs" --bump current

  [ "$status" -eq 0 ]
  printf '%s' "$output" | jq -e --arg version "$version" '
    .version == $version and .previous == $version and .changed == false
  ' >/dev/null
  after="$(
    sha256sum \
      "$ROOT/plugins/development-system/package.json" \
      "$ROOT/.claude-plugin/marketplace.json" \
      "$ROOT/.agents/plugins/marketplace.json" \
      "$ROOT/README.md"
  )"
  [ "$before" = "$after" ]
}
