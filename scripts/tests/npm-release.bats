#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  TMPROOT="$(mktemp -d)"
}

teardown() {
  rm -rf "$TMPROOT"
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
  jq -e '
    .publishConfig.registry == "https://registry.npmjs.org" and
    .publishConfig.access == "public"
  ' "$ROOT/plugins/development-system/package.json" >/dev/null
}

@test "automatic releases infer the highest conventional semantic change" {
  run node --input-type=module - "$ROOT/scripts/determine-development-system-bump.mjs" <<'NODE'
import assert from "node:assert/strict";
import { pathToFileURL } from "node:url";
const module = await import(pathToFileURL(process.argv[2]));
assert.deepEqual(module.determineBump(["docs: clarify setup"]), {
  bump: "patch",
  breaking: false,
  features: false,
});
assert.equal(
  module.determineBump(["fix: repair launcher", "feat(pi): add command"]).bump,
  "minor",
);
assert.equal(
  module.determineBump(["feat: add mode", "refactor(core)!: remove old mode"]).bump,
  "major",
);
assert.equal(
  module.determineBump(["fix: preserve API\n\nBREAKING CHANGE: config schema changed"]).bump,
  "major",
);
assert.equal(
  module.determineBump(["fix: preserve API\n\nbreaking-change: config schema changed"]).bump,
  "major",
);
assert.throws(() => module.determineBump([]));
NODE

  [ "$status" -eq 0 ]
}

@test "automatic semantic analysis recovers an untagged partial version commit" {
  mkdir -p "$TMPROOT/scripts" "$TMPROOT/plugins/development-system"
  cp "$ROOT/scripts/determine-development-system-bump.mjs" "$TMPROOT/scripts/"
  git -C "$TMPROOT" init --initial-branch=main >/dev/null
  git -C "$TMPROOT" config user.name Test
  git -C "$TMPROOT" config user.email test@example.invalid
  git -C "$TMPROOT" config commit.gpgSign false
  git -C "$TMPROOT" config tag.gpgSign false
  printf '{"version":"1.3.0"}\n' >"$TMPROOT/plugins/development-system/package.json"
  git -C "$TMPROOT" add .
  git -C "$TMPROOT" commit -m "feat: initial release" >/dev/null
  git -C "$TMPROOT" tag development-system-v1.3.0
  printf '{"version":"1.4.0"}\n' >"$TMPROOT/plugins/development-system/package.json"
  git -C "$TMPROOT" add .
  git -C "$TMPROOT" commit -m "chore(release): development-system v1.4.0" >/dev/null
  version_commit="$(git -C "$TMPROOT" rev-parse HEAD)"

  run node "$TMPROOT/scripts/determine-development-system-bump.mjs" \
    --current-version 1.4.0 --to HEAD

  [ "$status" -eq 0 ]
  printf '%s' "$output" | jq -e --arg from "$version_commit" '
    .bump == "current" and .commits == 0 and
    .from == $from and .pendingVersionCommit == true
  ' >/dev/null

  printf 'fix\n' >"$TMPROOT/fix.txt"
  git -C "$TMPROOT" add .
  git -C "$TMPROOT" commit -m "fix: recover publication" >/dev/null
  run node "$TMPROOT/scripts/determine-development-system-bump.mjs" \
    --current-version 1.4.0 --to HEAD

  [ "$status" -eq 0 ]
  printf '%s' "$output" | jq -e --arg from "$version_commit" '
    .bump == "patch" and .commits == 1 and
    .from == $from and .pendingVersionCommit == true
  ' >/dev/null
}

@test "automatic publish waits for successful main-push CI" {
  workflow="$ROOT/.github/workflows/publish-development-system.yml"

  run yq -r '.on.workflow_run.workflows[]' "$workflow"
  [ "$status" -eq 0 ]
  [ "$output" = CI ]
  run yq -r '.on.workflow_run.types[]' "$workflow"
  [ "$status" -eq 0 ]
  [ "$output" = completed ]
  run rg "github\.event\.workflow_run\.conclusion == 'success'" "$workflow"
  [ "$status" -eq 0 ]
  run rg "github\.event\.workflow_run\.event == 'push'" "$workflow"
  [ "$status" -eq 0 ]
  run rg "github\.event\.workflow_run\.head_branch == 'main'" "$workflow"
  [ "$status" -eq 0 ]
  run rg "github\.event\.workflow_run\.head_sha" "$workflow"
  [ "$status" -eq 0 ]
}

@test "publication uses npmjs trusted publishing without an npm token" {
  workflow="$ROOT/.github/workflows/publish-development-system.yml"

  [ "$(yq -r '.permissions."id-token"' "$workflow")" = write ]
  run rg "NPM_ID_TOKEN" "$workflow"
  [ "$status" -eq 0 ]
  run rg "ACTIONS_ID_TOKEN_REQUEST_URL" "$workflow"
  [ "$status" -eq 0 ]
  run rg "npm@11\.6\.2 publish" "$workflow"
  [ "$status" -eq 0 ]
  run rg -- "--provenance" "$workflow"
  [ "$status" -eq 0 ]
  run rg "https://registry\.npmjs\.org" "$workflow"
  [ "$status" -eq 0 ]
  run rg "NPM_TOKEN|NODE_AUTH_TOKEN|npm\.pkg\.github\.com|packages: write" "$workflow"
  [ "$status" -eq 1 ]
}

@test "trusted publisher binding is verified before release metadata mutates" {
  workflow="$ROOT/.github/workflows/publish-development-system.yml"

  run yq -r '.jobs.release.steps[].name' "$workflow"
  [ "$status" -eq 0 ]
  preflight_line="$(printf '%s\n' "$output" | grep -n '^Verify npm trusted publisher binding before version mutation$' | cut -d: -f1)"
  version_line="$(printf '%s\n' "$output" | grep -n '^Resolve semantic change and apply canonical version$' | cut -d: -f1)"
  [ -n "$preflight_line" ]
  [ -n "$version_line" ]
  [ "$preflight_line" -lt "$version_line" ]
  run rg 'oidc/token/exchange/package' "$workflow"
  [ "$status" -eq 0 ]
}

@test "release commit contains every synchronized surface and requires signed GraphQL output" {
  run node --input-type=module - "$ROOT/scripts/create-github-release-commit.mjs" "$ROOT" <<'NODE'
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";
const module = await import(pathToFileURL(process.argv[2]));
const root = process.argv[3];
const input = module.releaseCommitInput({
  repository: "jwilger/ai-plugins",
  branch: "main",
  expectedHeadOid: "a".repeat(40),
  version: "2.3.4",
});
assert.equal(input.message.headline, "chore(release): development-system v2.3.4");
assert.equal(input.fileChanges.additions.length, module.releaseFiles.length);
assert.deepEqual(
  input.fileChanges.additions.map(({ path }) => path),
  [...module.releaseFiles],
);
for (const addition of input.fileChanges.additions) {
  assert.deepEqual(
    Buffer.from(addition.contents, "base64"),
    fs.readFileSync(path.join(root, addition.path)),
  );
}
assert.throws(() => module.releaseCommitInput({
  repository: "invalid",
  branch: "main",
  expectedHeadOid: "a".repeat(40),
  version: "2.3.4",
}));
NODE

  [ "$status" -eq 0 ]
  workflow="$ROOT/.github/workflows/publish-development-system.yml"
  run rg "scripts/create-github-release-commit\.mjs" "$workflow"
  [ "$status" -eq 0 ]
  run rg "git commit" "$workflow"
  [ "$status" -eq 1 ]
}

@test "main CI restores the pinned package toolchain before the full gate" {
  workflow="$ROOT/.github/workflows/ci.yml"

  run yq -r '.jobs.quality.steps[] | select(.name == "Restore pinned package dependencies") | .run' "$workflow"
  [ "$status" -eq 0 ]
  [[ "$output" == *"scripts/bootstrap-pi-package.sh"* ]]
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

@test "version command updates every canonical release surface in an isolated checkout" {
  mkdir -p "$TMPROOT/scripts"
  cp "$ROOT/scripts/version-development-system.mjs" \
    "$ROOT/scripts/sync-development-system-metadata.mjs" \
    "$ROOT/scripts/generate-pi-support-docs.mjs" \
    "$TMPROOT/scripts/"
  while read -r relative; do
    mkdir -p "$TMPROOT/$(dirname "$relative")"
    cp "$ROOT/$relative" "$TMPROOT/$relative"
  done <<'FILES'
.agents/plugins/marketplace.json
.agents/plugins/pi-support.json
.claude-plugin/marketplace.json
README.md
plugins/development-system/README.md
plugins/development-system/package.json
plugins/development-system/.claude-plugin/plugin.json
plugins/development-system/.codex-plugin/plugin.json
plugins/development-system/.mcp.json
plugins/development-system/components/agentic-systems-engineering/.mcp.json
plugins/development-system/components/development-discipline/.mcp.json
plugins/development-system/components/tiber/.mcp.json
FILES

  current="$(jq -r '.version' "$TMPROOT/plugins/development-system/package.json")"
  expected="$(
    node --input-type=module - "$TMPROOT/scripts/version-development-system.mjs" "$current" <<'NODE'
import { pathToFileURL } from "node:url";
const module = await import(pathToFileURL(process.argv[2]));
process.stdout.write(module.nextVersion(process.argv[3], "patch"));
NODE
  )"
  run node "$TMPROOT/scripts/version-development-system.mjs" --bump patch

  [ "$status" -eq 0 ]
  printf '%s' "$output" | jq -e --arg current "$current" --arg expected "$expected" '
    .previous == $current and .version == $expected and .changed == true
  ' >/dev/null
  [ "$(jq -r '.version' "$TMPROOT/plugins/development-system/package.json")" = "$expected" ]
  [ "$(jq -r '.version' "$TMPROOT/plugins/development-system/.claude-plugin/plugin.json")" = "$expected" ]
  [ "$(jq -r '.version' "$TMPROOT/plugins/development-system/.codex-plugin/plugin.json")" = "$expected" ]
  [ "$(jq -r '.plugins[] | select(.name == "development-system") | .version' "$TMPROOT/.claude-plugin/marketplace.json")" = "$expected" ]
  [ "$(jq -r '.plugins[] | select(.name == "development-system") | .version' "$TMPROOT/.agents/plugins/marketplace.json")" = "$expected" ]
  run bash -c 'expected=$1; shift; rg -o "development-system/[0-9.]+/bin/" "$@" | grep -Fv "development-system/$expected/bin/"' \
    _ "$expected" \
    "$TMPROOT/plugins/development-system/.mcp.json" \
    "$TMPROOT/plugins/development-system/components/agentic-systems-engineering/.mcp.json" \
    "$TMPROOT/plugins/development-system/components/development-discipline/.mcp.json" \
    "$TMPROOT/plugins/development-system/components/tiber/.mcp.json"
  [ "$status" -eq 1 ]
  run rg "\\| $expected\\s+\\|" "$TMPROOT/README.md"
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
