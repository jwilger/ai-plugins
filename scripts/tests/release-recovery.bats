#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd -P)"
  mkdir -p "$BATS_TEST_TMPDIR/bin"
  printf '%s\n' '#!/usr/bin/env bash' 'cat "$RELEASE_JSON"' >"$BATS_TEST_TMPDIR/bin/gh"
  chmod +x "$BATS_TEST_TMPDIR/bin/gh"
}

check_preflight() {
  local plugin=$1
  node --input-type=module - "$ROOT/.github/workflows/release-$plugin.yml" <<'JS' >"$BATS_TEST_TMPDIR/preflight.sh"
import fs from 'node:fs';
import YAML from 'yaml';
const workflow = YAML.parse(fs.readFileSync(process.argv[2], 'utf8'));
console.log(workflow.jobs.build.steps.find(step => step.id === 'existing').run);
JS
  env PATH="$BATS_TEST_TMPDIR/bin:$PATH" RELEASE_JSON="$BATS_TEST_TMPDIR/release.json" \
    GITHUB_OUTPUT="$BATS_TEST_TMPDIR/output" GH_REPO=owner/repo TAG="$plugin-v1.0.0" \
    ARCHIVE="$plugin-v1.0.0-linux-x86_64.tar.gz" bash -e "$BATS_TEST_TMPDIR/preflight.sh"
}

@test "draft releases rebuild assets so publication can resume for either plugin" {
  printf '%s\n' '{"draft":true,"assets":[]}' >"$BATS_TEST_TMPDIR/release.json"
  for plugin in development-system tiber; do
    : >"$BATS_TEST_TMPDIR/output"
    run check_preflight "$plugin"
    [ "$status" -eq 0 ]
    grep -Fx existing=false "$BATS_TEST_TMPDIR/output"
  done
}

@test "published releases with missing assets remain immutable failures" {
  printf '%s\n' '{"draft":false,"assets":[]}' >"$BATS_TEST_TMPDIR/release.json"
  for plugin in development-system tiber; do
    run check_preflight "$plugin"
    [ "$status" -ne 0 ]
  done
}

@test "complete published releases skip rebuilding for either plugin" {
  for plugin in development-system tiber; do
    node - "$plugin" <<'JS' >"$BATS_TEST_TMPDIR/release.json"
const tag = `${process.argv[2]}-v1.0.0`;
const binary = `${tag}-linux-x86_64.tar.gz`;
const source = `${tag}-source.tar.gz`;
console.log(JSON.stringify({draft: false, assets: [binary, `${binary}.sha256`, source, `${source}.sha256`].map(name => ({name}))}));
JS
    : >"$BATS_TEST_TMPDIR/output"
    run check_preflight "$plugin"
    [ "$status" -eq 0 ]
    grep -Fx existing=true "$BATS_TEST_TMPDIR/output"
  done
}
