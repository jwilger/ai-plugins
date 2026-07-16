#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  RUNTIME_PREPARER="$ROOT/scripts/evals/prepare-code-quality-runtime.mjs"
  WORKSPACE_PREPARER="$ROOT/scripts/evals/prepare-code-quality-workspaces.mjs"
  CODEX_RESOLVER="$ROOT/scripts/evals/resolve-code-quality-codex.mjs"
  TEST_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/ai-plugins-code-quality-runtime-test.XXXXXX")"
  WORK_ROOT="$TEST_ROOT/workspaces"
  RUNTIME_ROOT="$TEST_ROOT/runtime"

  CODEX_RESOLUTION="$(node "$CODEX_RESOLVER")"
  export CODE_QUALITY_CODEX_REAL_BIN="$(jq -er '.codexBin' <<<"$CODEX_RESOLUTION")"
  export CODE_QUALITY_CODEX_EXPECTED_SHA256="$(
    sha256sum "$CODE_QUALITY_CODEX_REAL_BIN" | cut -d ' ' -f 1
  )"
  mkdir -m 700 "$TEST_ROOT/version-home" "$TEST_ROOT/version-tmp"
  export CODE_QUALITY_CODEX_EXPECTED_VERSION="$(
    env -i \
      HOME="$TEST_ROOT/version-home" \
      CODEX_HOME="$TEST_ROOT/version-home" \
      TMPDIR="$TEST_ROOT/version-tmp" \
      LANG=C.UTF-8 \
      LC_ALL=C.UTF-8 \
      "$CODE_QUALITY_CODEX_REAL_BIN" --version
  )"
  export CODE_QUALITY_CODEX_MODEL="$(
    jq -er '.provider.model' \
      "$ROOT/evals/benchmarks/downstream-code-quality/benchmark.json"
  )"
  export CODE_QUALITY_CODEX_REASONING_EFFORT="$(
    jq -er '.provider.reasoningEffort' \
      "$ROOT/evals/benchmarks/downstream-code-quality/benchmark.json"
  )"
  export CODE_QUALITY_BOUNDARY_SHA256="$(
    printf 'runtime-boundary-test-v1' | sha256sum | cut -d ' ' -f 1
  )"
  export CODE_QUALITY_TOOLCHAIN_COMPOSITION_SHA256="$(
    printf 'runtime-toolchain-test-v1' | sha256sum | cut -d ' ' -f 1
  )"

  run node "$WORKSPACE_PREPARER" "$WORK_ROOT" \
    --case rust-cli-feature --samples 1
  [ "$status" -eq 0 ]
  WORKSPACE_MANIFEST="$WORK_ROOT/manifest.json"
  WORKSPACE_RUN_ID="$(jq -er '.runId | select(test("^[0-9a-f]{64}$"))' "$WORKSPACE_MANIFEST")"
}

teardown() {
  rm -rf "$TEST_ROOT"
}

@test "runtime preparation creates one private Codex home and tmp directory for every logical turn" {
  run node "$RUNTIME_PREPARER" "$WORKSPACE_MANIFEST" "$RUNTIME_ROOT"
  [ "$status" -eq 0 ]

  [ -f "$RUNTIME_ROOT/.ai-plugins-code-quality-runtime-root" ]
  [ -f "$RUNTIME_ROOT/manifest.json" ]
  cmp -s <(printf '%s\n' "$output") "$RUNTIME_ROOT/manifest.json"
  [ "$(stat -c '%a' "$RUNTIME_ROOT/.ai-plugins-code-quality-runtime-root")" = 600 ]
  [ "$(stat -c '%a' "$RUNTIME_ROOT/manifest.json")" = 600 ]
  [ -z "$(find "$RUNTIME_ROOT" -type d ! -perm 0700 -print -quit)" ]
  [ -z "$(find "$RUNTIME_ROOT" -type f ! -perm 0600 -print -quit)" ]

  for mode in \
    no-marketplace-skills \
    targeted-quality-skills \
    all-marketplace-skills; do
    row_root="$RUNTIME_ROOT/rust-cli-feature/sample-1/$mode"
    [ -d "$row_root/codex-home" ]
    [ -d "$row_root/tmp" ]
    [ "$(stat -c '%a' "$row_root")" = 700 ]
    [ "$(stat -c '%a' "$row_root/codex-home")" = 700 ]
    [ "$(stat -c '%a' "$row_root/tmp")" = 700 ]
  done

  jq -e \
    --arg manifest "$WORKSPACE_MANIFEST" \
    --arg runtime "$RUNTIME_ROOT" \
    --arg run_id "$WORKSPACE_RUN_ID" '
      . as $root |
      .schemaVersion == 1 and
      .benchmarkId == "downstream-code-quality" and
      .workspaceManifest == $manifest and
      .runtimeRoot == $runtime and
      .runId == $run_id and
      (.contractSha256 | test("^[0-9a-f]{64}$")) and
      (.workspaceManifestSha256 | test("^[0-9a-f]{64}$")) and
      (.matrixHash | test("^[0-9a-f]{64}$")) and
      (.rows | length) == 3 and
      ([.rows[].mode] == [
        "no-marketplace-skills",
        "targeted-quality-skills",
        "all-marketplace-skills"
      ]) and
      all(.rows[];
        .caseId == "rust-cli-feature" and
        .sample == 1 and
        .runId == $root.runId and
        .contractSha256 == $root.contractSha256 and
        .workspaceManifestSha256 == $root.workspaceManifestSha256 and
        .matrixHash == $root.matrixHash and
        (.fixtureDigest | test("^[0-9a-f]{64}$")) and
        (.workspace | startswith($manifest | sub("/manifest.json$"; "/"))) and
        (.baselineOid | test("^[0-9a-f]{40}$")) and
        (.codexHome == ($runtime + "/rust-cli-feature/sample-1/" + .mode + "/codex-home")) and
        (.codexTmp == ($runtime + "/rust-cli-feature/sample-1/" + .mode + "/tmp")) and
        (.inputHash | test("^[0-9a-f]{64}$")) and
        (.compositionHash | test("^[0-9a-f]{64}$"))
      )
    ' "$RUNTIME_ROOT/manifest.json"
}

@test "runtime preparation rejects a symlink in the requested runtime path" {
  mkdir "$TEST_ROOT/runtime-parent"
  ln -s "$TEST_ROOT/runtime-parent" "$TEST_ROOT/runtime-link"

  run node "$RUNTIME_PREPARER" \
    "$WORKSPACE_MANIFEST" "$TEST_ROOT/runtime-link/runtime"

  [ "$status" -eq 2 ]
  [[ "$output" == *"runtime path contains a symlink"* ]]
  [ ! -e "$TEST_ROOT/runtime-parent/runtime" ]
}

@test "runtime Codex homes contain the exact condition-specific skills projections and no auth files" {
  run node "$RUNTIME_PREPARER" "$WORKSPACE_MANIFEST" "$RUNTIME_ROOT"
  [ "$status" -eq 0 ]

  mapfile -t codex_homes < <(jq -r '.rows[].codexHome' "$RUNTIME_ROOT/manifest.json")
  mapfile -t codex_tmps < <(jq -r '.rows[].codexTmp' "$RUNTIME_ROOT/manifest.json")
  [ "${#codex_homes[@]}" -eq 3 ]
  [ "$(printf '%s\n' "${codex_homes[@]}" | sort -u | wc -l)" -eq 3 ]
  [ "$(printf '%s\n' "${codex_tmps[@]}" | sort -u | wc -l)" -eq 3 ]

  no_skills_home="$RUNTIME_ROOT/rust-cli-feature/sample-1/no-marketplace-skills/codex-home"
  targeted_home="$RUNTIME_ROOT/rust-cli-feature/sample-1/targeted-quality-skills/codex-home"
  all_home="$RUNTIME_ROOT/rust-cli-feature/sample-1/all-marketplace-skills/codex-home"
  [ ! -e "$no_skills_home/plugins/cache/ai-plugins" ]

  targeted_plugins="$(
    find "$targeted_home/plugins/cache/ai-plugins" \
      -mindepth 1 -maxdepth 1 -type d -printf '%f\n' | sort
  )"
  [ "$targeted_plugins" = $'advisor\ndevelopment-discipline\nengineering-standards' ]

  advisor_reference="skills/advisor/references/playbook.md"
  cmp -s \
    "$ROOT/plugins/advisor/$advisor_reference" \
    "$targeted_home/plugins/cache/ai-plugins/advisor/0.2.0/$advisor_reference"
  cmp -s \
    "$ROOT/plugins/advisor/$advisor_reference" \
    "$targeted_home/marketplace/plugins/advisor/$advisor_reference"

  expected_all_plugins="$(
    jq -r '.plugins[].name' "$ROOT/.agents/plugins/marketplace.json" | sort
  )"
  actual_all_plugins="$(
    find "$all_home/plugins/cache/ai-plugins" \
      -mindepth 1 -maxdepth 1 -type d -printf '%f\n' | sort
  )"
  [ "$actual_all_plugins" = "$expected_all_plugins" ]

  while IFS= read -r version_root; do
    projected_entries="$(
      find "$version_root" -mindepth 1 -maxdepth 1 -printf '%f\n' | sort
    )"
    [ "$projected_entries" = $'.codex-plugin\nskills' ]
    [ "$(find "$version_root/.codex-plugin" -mindepth 1 -maxdepth 1 -printf '%f\n')" = plugin.json ]
  done < <(
    find "$targeted_home/plugins/cache/ai-plugins" "$all_home/plugins/cache/ai-plugins" \
      -mindepth 2 -maxdepth 2 -type d | sort
  )

  [ -z "$(find "$RUNTIME_ROOT" -type f \( -name auth.json -o -name .credentials.json \) -print -quit)" ]
  [ -z "$(find "$RUNTIME_ROOT" -path '*/.plugin-eval/*' -print -quit)" ]
  ! rg -Fq '"successChecklist"' "$RUNTIME_ROOT"
  ! rg -Fq '"userInput"' "$RUNTIME_ROOT"
}

@test "runtime Codex homes contain pinned bundled skills and a nonsecret execution surface" {
  run node "$RUNTIME_PREPARER" "$WORKSPACE_MANIFEST" "$RUNTIME_ROOT"
  [ "$status" -eq 0 ]

  mapfile -t codex_homes < <(jq -r '.rows[].codexHome' "$RUNTIME_ROOT/manifest.json")
  for codex_home in "${codex_homes[@]}"; do
    system_root="$codex_home/skills/.system"
    marker="$system_root/.codex-system-skills.marker"
    surface="$codex_home/.ai-plugins-execution-surface.json"
    [ -d "$system_root" ]
    [ -f "$marker" ]
    [ -f "$surface" ]
    [ -z "$(find "$system_root" ! -type d ! -type f -print -quit)" ]
    [ -z "$(find "$system_root" -type f \( -name auth.json -o -name .credentials.json \) -print -quit)" ]
    [[ "$(cat "$marker")" =~ ^[0-9a-f]{16}$ ]]
    jq -e \
      --arg model "$CODE_QUALITY_CODEX_MODEL" \
      --arg reasoning "$CODE_QUALITY_CODEX_REASONING_EFFORT" \
      --arg codex_sha "$CODE_QUALITY_CODEX_EXPECTED_SHA256" \
      --arg codex_version "$CODE_QUALITY_CODEX_EXPECTED_VERSION" \
      --arg boundary_sha "$CODE_QUALITY_BOUNDARY_SHA256" \
      --arg toolchain_sha "$CODE_QUALITY_TOOLCHAIN_COMPOSITION_SHA256" '
        (keys == [
          "boundarySha256",
          "codexBinarySha256",
          "codexVersion",
          "model",
          "reasoningEffort",
          "schemaVersion",
          "toolchainCompositionSha256"
        ]) and
        .schemaVersion == 1 and
        .model == $model and
        .reasoningEffort == $reasoning and
        .codexBinarySha256 == $codex_sha and
        .codexVersion == $codex_version and
        .boundarySha256 == $boundary_sha and
        .toolchainCompositionSha256 == $toolchain_sha
      ' "$surface"
  done

  jq -e '
    (.rows | map(.availableSkills | map(select(startswith("codex-system:")))) | unique | length) == 1 and
    (.rows[] | select(.mode == "no-marketplace-skills") |
      (.availableSkills | length) > 0 and
      all(.availableSkills[]; startswith("codex-system:")))
  ' "$RUNTIME_ROOT/manifest.json"
}

@test "runtime config uses only the sanitized sandbox marketplace source" {
  run node "$RUNTIME_PREPARER" "$WORKSPACE_MANIFEST" "$RUNTIME_ROOT"
  [ "$status" -eq 0 ]

  while IFS= read -r codex_home; do
    config="$codex_home/config.toml"
    marketplace="$codex_home/marketplace/.agents/plugins/marketplace.json"
    grep -Fxq 'source = "/runtime/marketplace"' "$config"
    ! rg -q '/home/|last_updated' "$config"
    jq empty "$marketplace"
    [ -z "$(find "$codex_home/marketplace" -path '*/.plugin-eval/*' -print -quit)" ]
  done < <(jq -r '.rows[].codexHome' "$RUNTIME_ROOT/manifest.json")
}

@test "runtime rows expose only sorted namespaced skills from their copied projection" {
  run node "$RUNTIME_PREPARER" "$WORKSPACE_MANIFEST" "$RUNTIME_ROOT"
  [ "$status" -eq 0 ]

  jq -e '
    .rows[] |
    (.availableSkills == (.availableSkills | sort | unique)) and
    all(.availableSkills[]; test("^[a-z0-9]+(?:-[a-z0-9]+)*:[a-z0-9]+(?:-[a-z0-9]+)*$"))
  ' "$RUNTIME_ROOT/manifest.json"
  jq -e '
    .rows[] | select(.mode == "no-marketplace-skills") |
    (.availableSkills | length) > 0 and
    all(.availableSkills[]; startswith("codex-system:"))
  ' "$RUNTIME_ROOT/manifest.json"

  while IFS=$'\t' read -r mode plugin_csv; do
    expected="$TEST_ROOT/$mode.expected"
    : >"$expected"
    IFS=, read -ra plugins <<<"$plugin_csv"
    for plugin in "${plugins[@]}"; do
      [ -n "$plugin" ] || continue
      find "$ROOT/plugins/$plugin/skills" \
        -mindepth 1 -maxdepth 1 -type d -printf "$plugin:%f\n" \
        >>"$expected"
    done
    sort -u -o "$expected" "$expected"
    jq -r \
      --arg mode "$mode" \
      '.rows[] |
        select(.mode == $mode) |
        .availableSkills[] |
        select(startswith("codex-system:") | not)' \
      "$RUNTIME_ROOT/manifest.json" >"$TEST_ROOT/$mode.actual"
    cmp -s "$expected" "$TEST_ROOT/$mode.actual"
  done <<EOF
targeted-quality-skills	advisor,development-discipline,engineering-standards
all-marketplace-skills	$(jq -r '[.plugins[].name] | join(",")' "$ROOT/.agents/plugins/marketplace.json")
EOF
}

@test "runtime preparation preserves the workspace run identity and keeps evidence hashes stable" {
  second_runtime="$TEST_ROOT/runtime-second"

  run node "$RUNTIME_PREPARER" "$WORKSPACE_MANIFEST" "$RUNTIME_ROOT"
  [ "$status" -eq 0 ]
  run node "$RUNTIME_PREPARER" "$WORKSPACE_MANIFEST" "$second_runtime"
  [ "$status" -eq 0 ]

  first_manifest="$RUNTIME_ROOT/manifest.json"
  second_manifest="$second_runtime/manifest.json"
  [ "$(jq -r '.runId' "$first_manifest")" = "$WORKSPACE_RUN_ID" ]
  [ "$(jq -r '.runId' "$second_manifest")" = "$WORKSPACE_RUN_ID" ]
  [ "$(jq -r '.contractSha256' "$first_manifest")" = "$(sha256sum "$ROOT/evals/benchmarks/downstream-code-quality/benchmark.json" | cut -d ' ' -f 1)" ]
  [ "$(jq -r '.workspaceManifestSha256' "$first_manifest")" = "$(sha256sum "$WORKSPACE_MANIFEST" | cut -d ' ' -f 1)" ]
  [ "$(jq -c '[.rows[] | {compositionHash, inputHash}]' "$first_manifest")" = "$(jq -c '[.rows[] | {compositionHash, inputHash}]' "$second_manifest")" ]
  [ "$(jq -r '.matrixHash' "$first_manifest")" = "$(jq -r '.matrixHash' "$second_manifest")" ]
  [ "$(jq -r '[.rows[].inputHash] | unique | length' "$first_manifest")" -eq 1 ]
  [ "$(jq -r '[.rows[].compositionHash] | unique | length' "$first_manifest")" -eq 3 ]
}

@test "runtime composition changes when the resolved nonsecret provider surface changes" {
  second_runtime="$TEST_ROOT/runtime-second"

  run node "$RUNTIME_PREPARER" "$WORKSPACE_MANIFEST" "$RUNTIME_ROOT"
  [ "$status" -eq 0 ]
  run env CODE_QUALITY_CODEX_MODEL=fixture-alternate-model \
    node "$RUNTIME_PREPARER" "$WORKSPACE_MANIFEST" "$second_runtime"
  [ "$status" -eq 0 ]

  [ "$(jq -c '[.rows[].compositionHash]' "$RUNTIME_ROOT/manifest.json")" != \
    "$(jq -c '[.rows[].compositionHash]' "$second_runtime/manifest.json")" ]
}

@test "runtime preparation rejects a Codex binary whose pinned digest is wrong" {
  wrong_digest="$(printf '0%.0s' {1..64})"

  run env CODE_QUALITY_CODEX_EXPECTED_SHA256="$wrong_digest" \
    node "$RUNTIME_PREPARER" "$WORKSPACE_MANIFEST" "$RUNTIME_ROOT"

  [ "$status" -eq 2 ]
  [[ "$output" == *"Codex binary digest does not match"* ]]
  [ ! -e "$RUNTIME_ROOT" ]
}

@test "runtime preparation fails closed instead of overwriting a completed owned runtime" {
  run node "$RUNTIME_PREPARER" "$WORKSPACE_MANIFEST" "$RUNTIME_ROOT"
  [ "$status" -eq 0 ]
  original_manifest_digest="$(sha256sum "$RUNTIME_ROOT/manifest.json")"
  original_run_id="$(jq -r '.runId' "$RUNTIME_ROOT/manifest.json")"

  run node "$RUNTIME_PREPARER" "$WORKSPACE_MANIFEST" "$RUNTIME_ROOT"

  [ "$status" -eq 2 ]
  [[ "$output" == *"runtime artifacts already exist"* ]]
  [ "$(sha256sum "$RUNTIME_ROOT/manifest.json")" = "$original_manifest_digest" ]
  [ "$(jq -r '.runId' "$RUNTIME_ROOT/manifest.json")" = "$original_run_id" ]
}

@test "runtime preparation preserves and rejects an unowned destination" {
  mkdir "$RUNTIME_ROOT"
  printf 'user-owned\n' >"$RUNTIME_ROOT/sentinel"

  run node "$RUNTIME_PREPARER" "$WORKSPACE_MANIFEST" "$RUNTIME_ROOT"

  [ "$status" -eq 2 ]
  [[ "$output" == *"refusing to replace unowned runtime root"* ]]
  [ "$(cat "$RUNTIME_ROOT/sentinel")" = user-owned ]
  [ ! -e "$RUNTIME_ROOT/manifest.json" ]
}

@test "runtime preparation rejects stale credentials even in a marked runtime root" {
  mkdir -p "$RUNTIME_ROOT/rust-cli-feature/sample-1/no-marketplace-skills/codex-home"
  printf 'ai-plugins downstream code-quality runtime root\n' \
    >"$RUNTIME_ROOT/.ai-plugins-code-quality-runtime-root"
  printf 'stale-secret\n' \
    >"$RUNTIME_ROOT/rust-cli-feature/sample-1/no-marketplace-skills/codex-home/auth.json"

  run node "$RUNTIME_PREPARER" "$WORKSPACE_MANIFEST" "$RUNTIME_ROOT"

  [ "$status" -eq 2 ]
  [[ "$output" == *"runtime tree contains forbidden auth credentials"* ]]
  [ "$(cat "$RUNTIME_ROOT/rust-cli-feature/sample-1/no-marketplace-skills/codex-home/auth.json")" = stale-secret ]
}

@test "runtime preparation rejects roots that overlap workspaces or the configured auth source" {
  run node "$RUNTIME_PREPARER" "$WORKSPACE_MANIFEST" "$WORK_ROOT/runtime"
  [ "$status" -eq 2 ]
  [[ "$output" == *"runtime root overlaps protected workspace root"* ]]

  auth_home="$TEST_ROOT/auth-source"
  mkdir "$auth_home"
  run env CODEX_EVAL_AUTH_HOME="$auth_home" \
    node "$RUNTIME_PREPARER" "$WORKSPACE_MANIFEST" "$auth_home/runtime"
  [ "$status" -eq 2 ]
  [[ "$output" == *"runtime root overlaps protected auth source"* ]]
}

@test "runtime preparation rejects the OS temp root, repository, and real home as protected destinations" {
  for protected_root in "${TMPDIR:-/tmp}" "$ROOT" "$HOME"; do
    run node "$RUNTIME_PREPARER" "$WORKSPACE_MANIFEST" "$protected_root"
    [ "$status" -eq 2 ]
    [[ "$output" == *"runtime root must be below"* ]]
  done
}

@test "runtime preparation does not copy a dedicated API key or auth-source files" {
  auth_home="$TEST_ROOT/auth-source"
  mkdir "$auth_home"
  printf '{"token":"runtime-auth-canary"}\n' >"$auth_home/auth.json"

  run env \
    CODEX_EVAL_AUTH_HOME="$auth_home" \
    OPENAI_API_KEY=runtime-api-key-canary \
    node "$RUNTIME_PREPARER" "$WORKSPACE_MANIFEST" "$RUNTIME_ROOT"

  [ "$status" -eq 0 ]
  ! rg -q 'runtime-auth-canary|runtime-api-key-canary' "$RUNTIME_ROOT"
  [ -z "$(find "$RUNTIME_ROOT" -type f \( -name auth.json -o -name .credentials.json \) -print -quit)" ]
}

@test "runtime manifest preserves every validated workspace fixture digest" {
  run node "$RUNTIME_PREPARER" "$WORKSPACE_MANIFEST" "$RUNTIME_ROOT"

  [ "$status" -eq 0 ]
  jq -e -s '
    ([.[0].workspaces[] | {
      caseId, sample, mode, fixtureDigest
    }] | sort_by(.caseId, .sample, .mode)) ==
    ([.[1].rows[] | {
      caseId, sample, mode, fixtureDigest
    }] | sort_by(.caseId, .sample, .mode))
  ' "$WORKSPACE_MANIFEST" "$RUNTIME_ROOT/manifest.json"
}
