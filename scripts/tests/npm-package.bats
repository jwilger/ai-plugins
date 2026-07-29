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
  jq -e '
    .private == true and
    (has("publishConfig") | not)
  ' "$ROOT/plugins/development-system/package.json" >/dev/null
}

@test "repository package lifecycle has no npm publication automation" {
  for removed in \
    .github/workflows/publish-development-system.yml \
    scripts/create-github-release-commit.mjs \
    scripts/determine-development-system-bump.mjs \
    scripts/version-development-system.mjs; do
    [ ! -e "$ROOT/$removed" ]
  done

  publication_pattern='npm(@[^[:space:]]+)?[[:space:]]+publish|oidc/token/exchange/package|development-system-v[0-9$]'
  run rg -n --glob '*.yml' --glob '*.yaml' --glob '*.js' --glob '*.mjs' --glob '*.sh' \
    "$publication_pattern" "$ROOT/.github/workflows" "$ROOT/scripts"
  [ "$status" -eq 1 ]
  run rg -n "$publication_pattern" \
    "$ROOT/justfile" \
    "$ROOT/package.json" \
    "$ROOT/plugins/development-system/package.json"
  [ "$status" -eq 1 ]
}

@test "eval dependency bootstrap migrates the former root install behind a stable symlink" {
  mkdir -p \
    "$BATS_TEST_TMPDIR/repo/tooling/evals" \
    "$BATS_TEST_TMPDIR/repo/node_modules/.bin" \
    "$BATS_TEST_TMPDIR/repo/node_modules/@openai/codex-sdk" \
    "$BATS_TEST_TMPDIR/repo/node_modules/@anthropic-ai/claude-agent-sdk"
  touch "$BATS_TEST_TMPDIR/repo/node_modules/.bin/promptfoo"

  run "$ROOT/scripts/evals/ensure-node-deps.sh" "$BATS_TEST_TMPDIR/repo"

  [ "$status" -eq 0 ]
  [ -L "$BATS_TEST_TMPDIR/repo/node_modules" ]
  [ "$(readlink "$BATS_TEST_TMPDIR/repo/node_modules")" = "tooling/evals/node_modules" ]
  [ -e "$BATS_TEST_TMPDIR/repo/tooling/evals/node_modules/.bin/promptfoo" ]
}

@test "main CI restores the pinned package toolchain before the full gate" {
  workflow="$ROOT/.github/workflows/ci.yml"

  run yq -r '.jobs.quality.steps[] | select(.name == "Restore pinned package dependencies") | .run' "$workflow"
  [ "$status" -eq 0 ]
  [[ "$output" == *"scripts/bootstrap-pi-package.sh"* ]]
}
