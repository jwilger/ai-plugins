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
        "agentic-systems", "beads", "delivery", "development-workflow",
        "engineering-standards", "eval-case-reporting", "setup", "worktrees"
      ],
      componentEntrypoints: ["./bin/development-discipline-mcp"]
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

@test "repository root is a lightweight Pi Git-package facade" {
  run jq -e '
    .private == true and
    (has("dependencies") | not) and
    (has("devDependencies") | not) and
    ([.peerDependenciesMeta[].optional] | all) and
    .pi.extensions == ["./plugins/development-system/extensions/development-system/index.ts"] and
    (.pi.skills | length) == 8 and
    (.pi.skills | all(startswith("./plugins/development-system/skills/")))
  ' "$REPO_ROOT/package.json"
  [ "$status" -eq 0 ]

  mkdir -p "$TEST_ROOT/git-package"
  cp "$REPO_ROOT/package.json" "$TEST_ROOT/git-package/"
  run npm --prefix "$TEST_ROOT/git-package" install \
    --ignore-scripts --no-audit --no-fund --package-lock=false --offline
  [ "$status" -eq 0 ]
  [ ! -e "$TEST_ROOT/git-package/node_modules" ]

  shallow="$TEST_ROOT/shallow-checkout"
  mkdir -p "$shallow"
  git -C "$REPO_ROOT" archive HEAD | tar -x -C "$shallow"
  cp "$REPO_ROOT/scripts/pi-package-canary.mjs" \
    "$shallow/scripts/pi-package-canary.mjs"
  git -C "$shallow" init -q --initial-branch=main
  git -C "$shallow" config user.name "Pi Canary"
  git -C "$shallow" config user.email "pi-canary@example.invalid"
  git -C "$shallow" add .
  git -C "$shallow" commit -qm "test: one-commit checkout"
  ln -s "$REPO_ROOT/tooling/evals/node_modules" "$shallow/node_modules"

  run node "$shallow/scripts/pi-package-canary.mjs" --git-source
  [ "$status" -eq 0 ]
  printf '%s' "$output" | jq -e '
    .ok == true and .package == "development-system" and
    .source.type == "git" and
    (.source.spec | startswith("git:github.com/jwilger/ai-plugins@")) and
    .source.observedSettings == true and
    .source.npmReconciled == true and
    .source.checkout == .source.requestedCommit and
    (.skills | length) == 8 and
    (.extension.extension | endswith("/plugins/development-system/extensions/development-system/index.ts"))
  ' >/dev/null
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
