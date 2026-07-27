#!/usr/bin/env bats

setup() {
  REPO_ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  export REPO_ROOT
  TEST_ROOT="$(mktemp -d)"
  export TEST_ROOT
}

teardown() {
  rm -rf -- "$TEST_ROOT"
}

@test "Pi release metadata and support inventory are synchronized" {
  run node "$REPO_ROOT/scripts/sync-development-system-metadata.mjs" --check
  [ "$status" -eq 0 ]
}

@test "Pi support inventory declares exactly the eight public skills and extension" {
  run jq -e '
    .schemaVersion == 1 and
    .packages == [{
      name: "development-system",
      path: "./plugins/development-system",
      extension: "./extensions/development-system/index.ts",
      skills: [
        "agentic-systems", "delivery", "development-workflow",
        "engineering-standards", "eval-case-reporting", "setup", "tasks", "worktrees"
      ],
      componentEntrypoints: [
        "./bin/development-discipline-mcp", "./bin/tiber"
      ]
    }]
  ' "$REPO_ROOT/.agents/plugins/pi-support.json"
  [ "$status" -eq 0 ]
}

@test "Pi package manifest explicitly exposes only inventory resources" {
  run node "$REPO_ROOT/scripts/validate-pi-package.mjs"
  [ "$status" -eq 0 ]
}

@test "Pi package bootstrap verifies host component binaries without global npm installs" {
  run env DEVELOPMENT_SYSTEM_BOOTSTRAP_SKIP_NPM=1 \
    "$REPO_ROOT/scripts/bootstrap-pi-package.sh"
  [ "$status" -eq 0 ]
  [[ "$output" == *"development_system.pi_bootstrap_ready"* ]]
}

@test "provider-free Pi canary proves package provenance and no-package absence" {
  run node "$REPO_ROOT/scripts/pi-package-canary.mjs"
  [ "$status" -eq 0 ]
  printf '%s' "$output" | jq -e '
    .ok == true and .package == "development-system" and .piVersion == "0.82.1" and
    (.skills | length) == 8 and
    (.extension.extension | endswith("/plugins/development-system/extensions/development-system/index.ts"))
  ' >/dev/null
}
