#!/usr/bin/env bats

setup() {
  umask 077
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  CHECKER="$ROOT/scripts/evals/check-code-quality-benchmark.mjs"
  SCANNER="$ROOT/scripts/evals/scan-code-quality-secrets.mjs"
  TEST_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/ai-plugins-code-quality-results.XXXXXX")"
  chmod 700 "$TEST_ROOT"
}

teardown() {
  rm -rf "$TEST_ROOT"
}

export_runtime_contract() {
  local resolution version_home version_tmp
  resolution="$(node "$ROOT/scripts/evals/resolve-code-quality-codex.mjs")"
  export CODE_QUALITY_CODEX_REAL_BIN="$(jq -er '.codexBin' <<<"$resolution")"
  export CODE_QUALITY_CODEX_EXPECTED_SHA256="$(
    sha256sum "$CODE_QUALITY_CODEX_REAL_BIN" | cut -d ' ' -f 1
  )"
  version_home="$TEST_ROOT/version-home"
  version_tmp="$TEST_ROOT/version-tmp"
  mkdir -m 700 "$version_home" "$version_tmp"
  export CODE_QUALITY_CODEX_EXPECTED_VERSION="$(
    env -i \
      HOME="$version_home" \
      CODEX_HOME="$version_home" \
      TMPDIR="$version_tmp" \
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
}

prepare_runtime() {
  WORK_ROOT="$1"
  WORKSPACE_MANIFEST="$WORK_ROOT/manifest.json"
  RUN_ROOT="$TEST_ROOT/run"
  HOST_TMP="$RUN_ROOT/host-tmp"
  RUNTIME_ROOT="$HOST_TMP/runtime"
  RUNTIME_MANIFEST="$RUNTIME_ROOT/manifest.json"
  mkdir -m 700 "$RUN_ROOT" "$HOST_TMP"
  printf 'ai-plugins downstream code-quality run root\n' \
    >"$RUN_ROOT/.ai-plugins-code-quality-run-root"
  chmod 600 "$RUN_ROOT/.ai-plugins-code-quality-run-root"
  export_runtime_contract
  node "$ROOT/scripts/evals/prepare-code-quality-workspaces.mjs" \
    "$WORK_ROOT" --case rust-cli-feature --samples 3 >/dev/null
  node "$ROOT/scripts/evals/prepare-code-quality-runtime.mjs" \
    "$WORKSPACE_MANIFEST" "$RUNTIME_ROOT" >/dev/null
}

prepare_trusted_runtime() {
  prepare_runtime "$TEST_ROOT/run/host-tmp/workspaces"
}

prepare_external_workspace_runtime() {
  prepare_runtime "$TEST_ROOT/external-workspaces"
}

write_valid_benchmark_inputs() {
  RUN_ROOT="$TEST_ROOT/run"
  RAW_RESULTS="$RUN_ROOT/raw/results.json"
  ARTIFACT_ROOT="$RUN_ROOT/artifacts"
  PROVENANCE="$RUN_ROOT/provenance.json"
  OUTPUT_DIRECTORY="$TEST_ROOT/sanitized"
  OUTPUT="$OUTPUT_DIRECTORY/results.json"
  mkdir -m 700 "$RUN_ROOT/raw" "$ARTIFACT_ROOT" "$OUTPUT_DIRECTORY"

  node - \
    "$RUNTIME_MANIFEST" \
    "$RAW_RESULTS" \
    "$ARTIFACT_ROOT" \
    "$PROVENANCE" \
    "$ROOT/evals/benchmarks/downstream-code-quality/cases.cjs" \
    "$ROOT/evals/benchmarks/downstream-code-quality/verifiers/verifier-composition.mjs" <<'NODE'
const { execFileSync } = require("node:child_process");
const crypto = require("node:crypto");
const fs = require("node:fs");
const path = require("node:path");

const [
  runtimeFile,
  rawFile,
  artifactRoot,
  provenanceFile,
  casesFile,
  verifierCompositionFile,
] =
  process.argv.slice(2);
const runtimeBytes = fs.readFileSync(runtimeFile);
const runtime = JSON.parse(runtimeBytes);
const digest = (value) => crypto.createHash("sha256").update(value).digest("hex");
const fixed = (character) => character.repeat(64);
const verifierCompositionSha256 = execFileSync(
  process.execPath,
  [verifierCompositionFile, "--sha256"],
  { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] },
).trim();
process.env.CODE_QUALITY_RUNTIME_MANIFEST = runtimeFile;
process.env.CODE_QUALITY_WORKSPACE_MANIFEST = runtime.workspaceManifest;
process.env.EVAL_CASE_FILTER = "rust-cli-feature";
process.env.EVAL_SAMPLES = "3";
const loadedCases = require(casesFile)();
const promptByRow = new Map(
  loadedCases.map((testCase) => [
    `${testCase.vars.case_id}\0${testCase.vars.sample_index}\0${testCase.vars.condition_id}`,
    testCase.vars.scenario_prompt,
  ]),
);
function sandboxSkillPath(row, qualifiedName) {
  const [namespace, skill] = qualifiedName.split(":");
  if (namespace === "codex-system") {
    return `/runtime/codex-home/skills/.system/${skill}/SKILL.md`;
  }
  const versionRoot = path.join(
    row.codexHome,
    "plugins/cache/ai-plugins",
    namespace,
  );
  const versions = fs.readdirSync(versionRoot);
  if (versions.length !== 1) throw new Error("unexpected plugin version surface");
  return `/runtime/codex-home/plugins/cache/ai-plugins/${namespace}/${versions[0]}/skills/${skill}/SKILL.md`;
}
const gates = {
  "source-rebuild": true,
  "black-box-behavior": true,
  "regression-tests": true,
  "baseline-regression-replay": true,
  format: true,
  clippy: true,
  "diff-scope": true,
  safety: true,
};
const rawResults = [];
for (const row of runtime.rows) {
  const providerLabel = `openai-codex-sdk-${row.mode}`;
  const scenarioPrompt = promptByRow.get(
    `${row.caseId}\0${row.sample}\0${row.mode}`,
  );
  if (!scenarioPrompt) throw new Error("missing trusted scenario prompt");
  const vars = {
    baseline_oid: row.baselineOid,
    benchmark_expected_samples: 3,
    case_id: row.caseId,
    condition_id: row.mode,
    expected_provider_label: providerLabel,
    fixture_digest: row.fixtureDigest,
    min_pass_rate: 0,
    sample_index: row.sample,
    scenario_prompt: scenarioPrompt,
    task_type: "feature",
    value_gate_mode: "measurement",
    workspace: row.workspace,
    available_skills: row.availableSkills,
    codex_home: row.codexHome,
    codex_tmp: row.codexTmp,
    composition_hash: row.compositionHash,
    contract_sha256: row.contractSha256,
    input_hash: row.inputHash,
    matrix_hash: row.matrixHash,
    run_id: row.runId,
    runtime_manifest_sha256: digest(runtimeBytes),
    workspace_manifest_sha256: row.workspaceManifestSha256,
  };
  const permittedSkill = row.availableSkills[0];
  const rawTurn = {
    items: permittedSkill
      ? [
          {
            id: "skill-read",
            type: "command_execution",
            command: `sed -n '1,220p' ${sandboxSkillPath(row, permittedSkill)}`,
            aggregated_output: "PRIVATE SKILL CONTENT SENTINEL",
            exit_code: 0,
            status: "completed",
          },
        ]
      : [],
    finalResponse: "PRIVATE FINAL RESPONSE SENTINEL",
    usage: {
      input_tokens: 100,
      cached_input_tokens: 10,
      output_tokens: 25,
      reasoning_output_tokens: 5,
    },
  };
  rawResults.push({
    provider: { id: "openai:codex-sdk", label: providerLabel },
    vars,
    testCase: { vars, description: "PRIVATE TEST CASE" },
    prompt: { raw: "PRIVATE PROMPT SENTINEL", label: "private" },
    response: {
      output: "PRIVATE OUTPUT SENTINEL candidate-secret.rs",
      raw: JSON.stringify(rawTurn),
      metadata: {
        skillCalls: [
          { name: "fabricated-metadata:must-be-ignored", path: "/private/path" },
          { name: "not-installed:private-skill", path: "/private/other" },
        ],
      },
    },
    success: true,
    score: 1,
    failureReason: 0,
    latencyMs: 1_000 + row.sample,
    cost: 0.125,
    tokenUsage: {
      prompt: 100,
      completion: 25,
      cached: 10,
      total: 125,
      numRequests: 1,
      completionDetails: { reasoning: 5 },
      assertions: { prompt: 2, completion: 1, total: 3 },
      privateText: "PRIVATE TOKEN SENTINEL",
    },
    gradingResult: {
      pass: true,
      score: 1,
      reason: "PRIVATE GRADING SENTINEL",
    },
    error: null,
  });

  const directory = path.join(artifactRoot, row.caseId, `sample-${row.sample}`);
  fs.mkdirSync(directory, { recursive: true, mode: 0o700 });
  for (const parent of [path.join(artifactRoot, row.caseId), directory]) {
    fs.chmodSync(parent, 0o700);
  }
  const artifact = {
    schemaVersion: 1,
    benchmarkId: "downstream-code-quality",
    caseId: row.caseId,
    taskType: "feature",
    conditionId: row.mode,
    sampleIndex: row.sample,
    providerLabel,
    baselineOid: row.baselineOid,
    runId: row.runId,
    contractSha256: row.contractSha256,
    workspaceManifestSha256: row.workspaceManifestSha256,
    runtimeManifestSha256: digest(runtimeBytes),
    matrixHash: row.matrixHash,
    fixtureDigest: row.fixtureDigest,
    inputHash: row.inputHash,
    compositionHash: row.compositionHash,
    promotionEligible: false,
    scoringMode: "trusted-source-rebuild",
    pass: true,
    outcomeClass: "pass",
    trustedFixtureSha256: row.fixtureDigest,
    verifierCompositionSha256,
    changeEvidence: {
      sourceFileCount: 2,
      sourceByteCount: 800,
      addedFileCount: 1,
      modifiedFileCount: 1,
      deletedFileCount: 0,
      changedFileCount: 2,
      candidateTreeSha256: fixed("b"),
      diffSha256: fixed("c"),
    },
    gates,
    verifier: "expense-report-trusted-source",
  };
  const artifactFile = path.join(directory, `${row.mode}.json`);
  fs.writeFileSync(artifactFile, `${JSON.stringify(artifact)}\n`, { mode: 0o600 });
}

const raw = {
  evalId: "PRIVATE EVAL ID",
  results: {
    version: 3,
    timestamp: "PRIVATE TIMESTAMP",
    results: rawResults,
    prompts: [{ raw: "PRIVATE PROMPT SENTINEL" }],
    stats: { private: "PRIVATE STATS SENTINEL" },
  },
  config: { private: "PRIVATE CONFIG SENTINEL" },
};
fs.writeFileSync(rawFile, `${JSON.stringify(raw)}\n`, { mode: 0o600 });

const provenance = {
  schemaVersion: 1,
  benchmarkId: "downstream-code-quality",
  runId: runtime.runId,
  contractSha256: runtime.contractSha256,
  workspaceManifestSha256: runtime.workspaceManifestSha256,
  runtimeManifestSha256: digest(runtimeBytes),
  matrixHash: runtime.matrixHash,
  model: "gpt-5.6-terra",
  reasoningEffort: "medium",
  codexVersion: "0.144.5",
  codexBinarySha256: fixed("d"),
  codexSdkVersion: "0.144.5",
  nodeVersion: "22.23.1",
  nodeBinarySha256: fixed("1"),
  promptfooVersion: "0.121.18",
  packageLockSha256: fixed("2"),
  boundarySha256: fixed("e"),
  toolchainCompositionSha256: fixed("f"),
};
fs.writeFileSync(provenanceFile, `${JSON.stringify(provenance)}\n`, { mode: 0o600 });
NODE
}

run_checker() {
  run node "$CHECKER" \
    --results "$RAW_RESULTS" \
    --artifacts "$ARTIFACT_ROOT" \
    --runtime-manifest "$RUNTIME_MANIFEST" \
    --provenance "$PROVENANCE" \
    --output "$OUTPUT"
}

@test "secret scanner detects an exact configured secret without disclosing it" {
  secret="sk-test-ThisExactCredentialMustNeverBePrinted-1234567890"
  scan_root="$TEST_ROOT/raw"
  mkdir -m 700 "$scan_root"
  printf '{"response":"%s"}\n' "$secret" >"$scan_root/results.json"
  chmod 600 "$scan_root/results.json"

  run env CODE_QUALITY_TEST_SECRET="$secret" \
    node "$SCANNER" --secret-env CODE_QUALITY_TEST_SECRET "$scan_root"

  [ "$status" -eq 1 ]
  [ "$output" = "secret-scan:secret-detected" ]
  [[ "$output" != *"$secret"* ]]
  [[ "$output" != *"$scan_root"* ]]
}

@test "secret scanner rejects common authentication material generically" {
  scan_file="$TEST_ROOT/raw.json"
  declare -a samples=(
    'ghp_0123456789abcdefghijklmnopqrstuvwxyzABCD'
    'github_pat_0123456789_abcdefghijklmnopqrstuvwxyz_ABCDEFGH'
    'sk-proj-0123456789abcdefghijklmnopqrstuvwxyz'
    'sk-ant-api03-0123456789abcdefghijklmnopqrstuvwxyz'
    'Authorization: Bearer abcdefghijklmnopqrstuvwxyz.0123456789'
    '-----BEGIN OPENSSH PRIVATE KEY-----'
    '{"access_token":"abcdefghijklmnopqrstuvwxyz0123456789"}'
  )

  for sample in "${samples[@]}"; do
    printf '%s\n' "$sample" >"$scan_file"
    chmod 600 "$scan_file"

    run node "$SCANNER" "$scan_file"

    [ "$status" -eq 1 ]
    [ "$output" = "secret-scan:secret-detected" ]
    [[ "$output" != *"$sample"* ]]
  done
}

@test "secret scanner detects authentication material in a filename without disclosing it" {
  scan_root="$TEST_ROOT/artifacts"
  credential_name="ghp_0123456789abcdefghijklmnopqrstuvwxyzABCD"
  mkdir -m 700 "$scan_root"
  : >"$scan_root/$credential_name"
  chmod 600 "$scan_root/$credential_name"

  run node "$SCANNER" "$scan_root"

  [ "$status" -eq 1 ]
  [ "$output" = "secret-scan:secret-detected" ]
  [[ "$output" != *"$credential_name"* ]]
  [[ "$output" != *"$scan_root"* ]]
}

@test "secret scanner detects authentication material in the input basename" {
  credential_name="github_pat_0123456789_abcdefghijklmnopqrstuvwxyz_ABCDEFGH"
  scan_file="$TEST_ROOT/$credential_name"
  : >"$scan_file"
  chmod 600 "$scan_file"

  run node "$SCANNER" "$scan_file"

  [ "$status" -eq 1 ]
  [ "$output" = "secret-scan:secret-detected" ]
  [[ "$output" != *"$credential_name"* ]]
  [[ "$output" != *"$TEST_ROOT"* ]]
}

@test "secret scanner rejects a hard-linked input that can escape the private tree" {
  outside="$TEST_ROOT/outside.json"
  scan_root="$TEST_ROOT/raw-hardlink"
  mkdir -m 700 "$scan_root"
  printf '{}\n' >"$outside"
  chmod 600 "$outside"
  ln "$outside" "$scan_root/results.json"

  run node "$SCANNER" "$scan_root"

  [ "$status" -eq 2 ]
  [ "$output" = "secret-scan:input-hard-linked" ]
  [[ "$output" != *"$outside"* ]]
  [[ "$output" != *"$scan_root"* ]]
}

@test "secret scanner fails closed on symlink special and oversized inputs" {
  scan_root="$TEST_ROOT/unsafe-inputs"
  mkdir -m 700 "$scan_root"
  printf '{}\n' >"$TEST_ROOT/target.json"
  chmod 600 "$TEST_ROOT/target.json"
  ln -s "$TEST_ROOT/target.json" "$scan_root/symlink.json"

  run node "$SCANNER" "$scan_root"
  [ "$status" -eq 2 ]
  [ "$output" = "secret-scan:input-symlink" ]
  [[ "$output" != *"$scan_root"* ]]
  rm "$scan_root/symlink.json"

  mkfifo -m 600 "$scan_root/special"
  run node "$SCANNER" "$scan_root"
  [ "$status" -eq 2 ]
  [ "$output" = "secret-scan:input-special-entry" ]
  [[ "$output" != *"$scan_root"* ]]
  rm "$scan_root/special"

  truncate -s $((64 * 1024 * 1024 + 1)) "$scan_root/oversized.json"
  chmod 600 "$scan_root/oversized.json"
  run node "$SCANNER" "$scan_root"
  [ "$status" -eq 2 ]
  [ "$output" = "secret-scan:input-file-too-large" ]
  [[ "$output" != *"$scan_root"* ]]
}

@test "exact-only scanner skips generic examples but keeps exact and filesystem gates" {
  scan_root="$TEST_ROOT/exact-only"
  scan_file="$scan_root/runtime.txt"
  exact_secret="runtime-exact-secret-0123456789"
  mkdir -m 700 "$scan_root"
  printf 'documentation example: ghp_0123456789abcdefghijklmnopqrstuvwxyzABCD\n' \
    >"$scan_file"
  chmod 600 "$scan_file"

  run env CODE_QUALITY_TEST_SECRET="$exact_secret" \
    node "$SCANNER" --exact-only \
      --secret-env CODE_QUALITY_TEST_SECRET "$scan_root"
  [ "$status" -eq 0 ]
  [ "$output" = "secret-scan:clean" ]

  printf '%s\n' "$exact_secret" >"$scan_file"
  run env CODE_QUALITY_TEST_SECRET="$exact_secret" \
    node "$SCANNER" --exact-only \
      --secret-env CODE_QUALITY_TEST_SECRET "$scan_root"
  [ "$status" -eq 1 ]
  [ "$output" = "secret-scan:secret-detected" ]
  [[ "$output" != *"$exact_secret"* ]]

  printf 'clean\n' >"$scan_file"
  ln -s "$scan_file" "$scan_root/unsafe-link"
  run env CODE_QUALITY_TEST_SECRET="$exact_secret" \
    node "$SCANNER" --exact-only \
      --secret-env CODE_QUALITY_TEST_SECRET "$scan_root"
  [ "$status" -eq 2 ]
  [ "$output" = "secret-scan:input-symlink" ]
  [[ "$output" != *"$scan_root"* ]]
}

@test "secret scanner bounds a single directory before traversing excessive fan-out" {
  scan_root="$TEST_ROOT/wide-tree"
  mkdir -m 700 "$scan_root"
  umask 077
  for index in $(seq 1 513); do
    : >"$scan_root/entry-$index"
  done

  run node "$SCANNER" "$scan_root"

  [ "$status" -eq 2 ]
  [ "$output" = "secret-scan:input-directory-too-large" ]
  [[ "$output" != *"$scan_root"* ]]
}

@test "Codex-runtime profile scans exact secrets without following allowed helper symlinks" {
  runtime="$TEST_ROOT/runtime"
  helper_root="$runtime/rust-cli-feature/sample-1/all-marketplace-skills/codex-home/tmp/arg0/codex-arg0abc"
  exact_secret="runtime-profile-exact-secret-0123456789"
  outside_helper="$TEST_ROOT/outside-helper"
  mkdir -m 700 -p "$helper_root"
  printf 'ai-plugins downstream code-quality runtime root\n' \
    >"$runtime/.ai-plugins-code-quality-runtime-root"
  printf 'documentation example ghp_0123456789abcdefghijklmnopqrstuvwxyzABCD\n' \
    >"$runtime/manifest.json"
  printf '%s\n' "$exact_secret" >"$outside_helper"
  : >"$helper_root/.lock"
  ln -s "$outside_helper" "$helper_root/apply_patch"

  run env CODE_QUALITY_TEST_SECRET="$exact_secret" \
    node "$SCANNER" --profile codex-runtime --exact-only \
      --secret-env CODE_QUALITY_TEST_SECRET "$runtime"

  [ "$status" -eq 0 ]
  [ "$output" = "secret-scan:clean" ]

  rm "$helper_root/apply_patch"
  ln -s "$exact_secret" "$helper_root/apply_patch"
  run env CODE_QUALITY_TEST_SECRET="$exact_secret" \
    node "$SCANNER" --profile codex-runtime --exact-only \
      --secret-env CODE_QUALITY_TEST_SECRET "$runtime"

  [ "$status" -eq 1 ]
  [ "$output" = "secret-scan:secret-detected" ]
  [[ "$output" != *"$exact_secret"* ]]
  [[ "$output" != *"$runtime"* ]]

  rm "$helper_root/apply_patch"
  ln -s "$outside_helper" "$helper_root/apply_patch"
  printf '%s\n' "$exact_secret" >"$runtime/manifest.json"
  run env CODE_QUALITY_TEST_SECRET="$exact_secret" \
    node "$SCANNER" --profile codex-runtime --exact-only \
      --secret-env CODE_QUALITY_TEST_SECRET "$runtime"

  [ "$status" -eq 1 ]
  [ "$output" = "secret-scan:secret-detected" ]
  [[ "$output" != *"$exact_secret"* ]]
}

@test "Codex-runtime profile rejects unsafe invocation ownership and helper topology" {
  runtime="$TEST_ROOT/runtime"
  helper_root="$runtime/rust-cli-feature/sample-1/all-marketplace-skills/codex-home/tmp/arg0/codex-arg0abc"
  mkdir -m 700 -p "$helper_root"
  printf 'ai-plugins downstream code-quality runtime root\n' \
    >"$runtime/.ai-plugins-code-quality-runtime-root"
  printf '{}\n' >"$runtime/manifest.json"
  : >"$helper_root/.lock"

  run node "$SCANNER" --profile codex-runtime "$runtime"
  [ "$status" -eq 2 ]
  [ "$output" = "secret-scan:invalid-arguments" ]

  run node "$SCANNER" --profile codex-runtime --exact-only \
    "$runtime" "$runtime"
  [ "$status" -eq 2 ]
  [ "$output" = "secret-scan:invalid-arguments" ]

  mv "$runtime/.ai-plugins-code-quality-runtime-root" \
    "$runtime/.runtime-marker-away"
  run node "$SCANNER" --profile codex-runtime --exact-only "$runtime"
  [ "$status" -eq 2 ]
  [ "$output" = "secret-scan:runtime-profile-invalid" ]
  mv "$runtime/.runtime-marker-away" \
    "$runtime/.ai-plugins-code-quality-runtime-root"

  printf 'not the owned runtime marker\n' \
    >"$runtime/.ai-plugins-code-quality-runtime-root"
  run node "$SCANNER" --profile codex-runtime --exact-only "$runtime"
  [ "$status" -eq 2 ]
  [ "$output" = "secret-scan:runtime-profile-invalid" ]
  printf 'ai-plugins downstream code-quality runtime root\n' \
    >"$runtime/.ai-plugins-code-quality-runtime-root"

  chmod 755 "$runtime"
  run node "$SCANNER" --profile codex-runtime --exact-only "$runtime"
  [ "$status" -eq 2 ]
  [ "$output" = "secret-scan:runtime-profile-invalid" ]
  chmod 700 "$runtime"

  ln -s '/private/escape' "$runtime/apply_patch"
  run node "$SCANNER" --profile codex-runtime --exact-only "$runtime"
  [ "$status" -eq 2 ]
  [ "$output" = "secret-scan:runtime-profile-invalid" ]
  [[ "$output" != *"/private/escape"* ]]
  rm "$runtime/apply_patch"

  ln -s '/private/wrong-helper' "$helper_root/not-a-helper"
  run node "$SCANNER" --profile codex-runtime --exact-only "$runtime"
  [ "$status" -eq 2 ]
  [ "$output" = "secret-scan:runtime-profile-invalid" ]
  rm "$helper_root/not-a-helper"

  printf 'clean\n' >"$TEST_ROOT/outside-hard-link"
  ln "$TEST_ROOT/outside-hard-link" "$runtime/hard-linked"
  run node "$SCANNER" --profile codex-runtime --exact-only "$runtime"
  [ "$status" -eq 2 ]
  [ "$output" = "secret-scan:input-hard-linked" ]
  rm "$runtime/hard-linked" "$TEST_ROOT/outside-hard-link"

  mkfifo -m 600 "$runtime/special"
  run node "$SCANNER" --profile codex-runtime --exact-only "$runtime"
  [ "$status" -eq 2 ]
  [ "$output" = "secret-scan:input-special-entry" ]
  rm "$runtime/special"

  mkdir -m 700 \
    "$runtime/rust-cli-feature/sample-1/all-marketplace-skills/codex-home/tmp/not-arg0"
  run node "$SCANNER" --profile codex-runtime --exact-only "$runtime"
  [ "$status" -eq 2 ]
  [ "$output" = "secret-scan:runtime-profile-invalid" ]
}

@test "checker emits only the allowlisted canonical nine-run diagnostic" {
  prepare_trusted_runtime
  write_valid_benchmark_inputs

  run_checker

  [ "$status" -eq 0 ]
  [ "$output" = "code-quality-results:written" ]
  [ "$(stat -c '%a' "$OUTPUT")" = 600 ]
  jq -e '
    .schemaVersion == 1 and
    .benchmarkId == "downstream-code-quality" and
    .promotionEligible == false and
    .diagnosticEligible == true and
    (.runs | length) == 9 and
    (.diagnostics == {
      expectedRuns: 9,
      completeRuns: 9,
      unexpectedResults: 0,
      duplicateResults: 0,
      missingResults: 0,
      candidateFailuresAreMeasurementOutcomes: true,
      safetyFailures: 0,
      operationalFailures: 0,
      provenanceFailures: 0,
      providerFailures: 0,
      outcomes: {
        pass: 9,
        candidateFailure: 0,
        safetyFailure: 0,
        operationalFailure: 0,
        provenanceFailure: 0,
        providerFailure: 0
      }
    }) and
    (.aggregates | length) == 3 and
    all(.aggregates[];
      .sampleCount == 3 and
      .successCount == 3 and
      .successRate == 1 and
      .passAt3Capability == 1 and
      .passPower3Reliability == 1
    ) and
    all(.runs[];
      .complete == true and
      .pass == true and
      .outcomeClass == "pass" and
      (.metrics.latencyMs | type) == "number" and
      (.metrics.cost | type) == "number" and
      all(.metrics.tokenUsage[];
        (type == "number") or
        (type == "object")
      ) and
      .skillActivationEvidence == "codex-turn-successful-command-path-references" and
      (.skillActivations | length) == 1 and
      all(.skillActivations[]; test("^[a-z0-9-]+:[a-z0-9-]+$"))
    ) and
    .provenance.model == "gpt-5.6-terra" and
    .provenance.codexVersion == "0.144.5" and
    .provenance.nodeVersion == "22.23.1" and
    .provenance.promptfooVersion == "0.121.18" and
    (.provenance.codexBinarySha256 | test("^[0-9a-f]{64}$")) and
    (.provenance.nodeBinarySha256 | test("^[0-9a-f]{64}$")) and
    (.provenance.packageLockSha256 | test("^[0-9a-f]{64}$")) and
    (.provenance.boundarySha256 | test("^[0-9a-f]{64}$")) and
    (.provenance.toolchainCompositionSha256 | test("^[0-9a-f]{64}$"))
  ' "$OUTPUT"

  run grep -E \
    'PRIVATE|candidate-secret|not-installed|/tmp|/home' \
    "$OUTPUT"
  [ "$status" -eq 1 ]
}

@test "checker accepts SemVer prerelease and build metadata in provenance" {
  prepare_trusted_runtime
  write_valid_benchmark_inputs
  jq '
    .codexVersion = "0.144.5+linux.x64" |
    .codexSdkVersion = "0.144.5+sdk.1" |
    .nodeVersion = "22.23.1+runtime.1" |
    .promptfooVersion = "0.121.18-beta.1+build.2"
  ' "$PROVENANCE" >"$PROVENANCE.updated"
  chmod 600 "$PROVENANCE.updated"
  mv "$PROVENANCE.updated" "$PROVENANCE"

  run_checker

  [ "$status" -eq 0 ]
  jq -e '
    .provenance.codexVersion == "0.144.5+linux.x64" and
    .provenance.codexSdkVersion == "0.144.5+sdk.1" and
    .provenance.nodeVersion == "22.23.1+runtime.1" and
    .provenance.promptfooVersion == "0.121.18-beta.1+build.2"
  ' "$OUTPUT"
}

@test "checker treats a candidate gate failure as a diagnostic measurement" {
  prepare_trusted_runtime
  write_valid_benchmark_inputs
  first_mode="$(jq -r '.rows[0].mode' "$RUNTIME_MANIFEST")"
  artifact="$ARTIFACT_ROOT/rust-cli-feature/sample-1/$first_mode.json"
  jq '
    .pass = false |
    .outcomeClass = "candidate-failure" |
    .gates["black-box-behavior"] = false
  ' "$artifact" >"$artifact.updated"
  chmod 600 "$artifact.updated"
  mv "$artifact.updated" "$artifact"
  jq --arg mode "$first_mode" '
    .results.results |= map(
      if .vars.sample_index == 1 and .vars.condition_id == $mode then
        .success = false |
        .score = 0 |
        .failureReason = 1 |
        .gradingResult.pass = false |
        .gradingResult.score = 0
      else . end
    )
  ' "$RAW_RESULTS" >"$RAW_RESULTS.updated"
  chmod 600 "$RAW_RESULTS.updated"
  mv "$RAW_RESULTS.updated" "$RAW_RESULTS"

  run_checker

  [ "$status" -eq 0 ]
  jq -e --arg mode "$first_mode" '
    .diagnosticEligible == true and
    .diagnostics.candidateFailuresAreMeasurementOutcomes == true and
    .diagnostics.outcomes.candidateFailure == 1 and
    .diagnostics.outcomes.pass == 8 and
    (.runs[] |
      select(.conditionId == $mode and .sampleIndex == 1) |
      .complete == true and
      .pass == false and
      .outcomeClass == "candidate-failure"
    ) and
    (.aggregates[] |
      select(.conditionId == $mode) |
      .sampleCount == 3 and
      .successCount == 2 and
      .successRate == (2 / 3) and
      .passAt3Capability == 1 and
      .passPower3Reliability == 0
    )
  ' "$OUTPUT"
}

@test "checker classifies safety operational and provenance failures without unsafe detail" {
  prepare_trusted_runtime
  write_valid_benchmark_inputs
  safety_mode="$(jq -r '.rows[0].mode' "$RUNTIME_MANIFEST")"
  operational_mode="$(jq -r '.rows[1].mode' "$RUNTIME_MANIFEST")"
  provenance_mode="$(jq -r '.rows[2].mode' "$RUNTIME_MANIFEST")"

  safety_artifact="$ARTIFACT_ROOT/rust-cli-feature/sample-1/$safety_mode.json"
  jq '
    .pass = false |
    .outcomeClass = "safety-failure" |
    .gates.safety = false
  ' "$safety_artifact" >"$safety_artifact.updated"
  chmod 600 "$safety_artifact.updated"
  mv "$safety_artifact.updated" "$safety_artifact"
  rm "$ARTIFACT_ROOT/rust-cli-feature/sample-1/$operational_mode.json"

  jq \
    --arg safety "$safety_mode" \
    --arg operational "$operational_mode" \
    --arg provenance "$provenance_mode" '
      .results.results |= map(
        if .vars.sample_index == 1 and .vars.condition_id == $safety then
          .success = false |
          .score = 0 |
          .failureReason = 1 |
          .gradingResult.pass = false |
          .gradingResult.score = 0
        elif .vars.sample_index == 1 and .vars.condition_id == $operational then
          .success = false |
          .score = 0 |
          .failureReason = 2 |
          .gradingResult.pass = false |
          .gradingResult.score = 0 |
          .error = "CODE_QUALITY_BOUNDARY_ERROR:configuration:node-runtime-unavailable"
        elif .vars.sample_index == 1 and .vars.condition_id == $provenance then
          .vars.composition_hash = ("0" * 64)
        else . end
      )
    ' "$RAW_RESULTS" >"$RAW_RESULTS.updated"
  chmod 600 "$RAW_RESULTS.updated"
  mv "$RAW_RESULTS.updated" "$RAW_RESULTS"

  run_checker

  [ "$status" -eq 0 ]
  jq -e '
    .diagnosticEligible == false and
    .diagnostics.completeRuns == 8 and
    .diagnostics.safetyFailures == 1 and
    .diagnostics.operationalFailures == 1 and
    .diagnostics.provenanceFailures == 1 and
    .diagnostics.providerFailures == 0 and
    .diagnostics.outcomes == {
      pass: 6,
      candidateFailure: 0,
      safetyFailure: 1,
      operationalFailure: 1,
      provenanceFailure: 1,
      providerFailure: 0
    } and
    ([.runs[].outcomeClass] | sort) ==
      (["pass", "pass", "pass", "pass", "pass", "pass",
        "operational-failure", "provenance-failure", "safety-failure"] | sort)
  ' "$OUTPUT"
  run grep -E 'CODE_QUALITY_BOUNDARY_ERROR|node-runtime-unavailable|/tmp|/home' "$OUTPUT"
  [ "$status" -eq 1 ]
}

@test "checker derives activations only from successful raw-turn skill path references" {
  prepare_trusted_runtime
  write_valid_benchmark_inputs
  mode="all-marketplace-skills"
  node - "$RAW_RESULTS" "$RUNTIME_MANIFEST" "$mode" <<'NODE'
const fs = require("node:fs");
const path = require("node:path");
const [resultsFile, runtimeFile, mode] = process.argv.slice(2);
const raw = JSON.parse(fs.readFileSync(resultsFile));
const runtime = JSON.parse(fs.readFileSync(runtimeFile));
const row = runtime.rows.find(
  (candidate) => candidate.sample === 1 && candidate.mode === mode,
);
if (!row) throw new Error("missing target row");

function pathFor(qualifiedName) {
  const [plugin, skill] = qualifiedName.split(":");
  if (plugin === "codex-system") {
    return `/runtime/codex-home/skills/.system/${skill}/SKILL.md`;
  }
  const versionRoot = path.join(
    row.codexHome,
    "plugins/cache/ai-plugins",
    plugin,
  );
  const versions = fs.readdirSync(versionRoot);
  if (versions.length !== 1) throw new Error("unexpected plugin version surface");
  return `/runtime/codex-home/plugins/cache/ai-plugins/${plugin}/${versions[0]}/skills/${skill}/SKILL.md`;
}

const successful = row.availableSkills.find((name) => name === "advisor:advisor");
const failed = row.availableSkills.find((name) => name === "worktrees:setup");
if (!successful || !failed) throw new Error("expected marketplace skills unavailable");
const target = raw.results.results.find(
  (result) =>
    result.vars.sample_index === 1 && result.vars.condition_id === mode,
);
target.response.metadata.skillCalls = [
  { name: failed, path: "/PRIVATE/spoofed-metadata" },
];
target.response.raw = JSON.stringify({
  items: [
    {
      id: "allowed-success",
      type: "command_execution",
      command: `cat '${pathFor(successful)}'`,
      aggregated_output: "PRIVATE ALLOWED CONTENT",
      exit_code: 0,
      status: "completed",
    },
    {
      id: "allowed-duplicate",
      type: "command_execution",
      command: `sed -n '1,80p' \"${pathFor(successful)}\"`,
      aggregated_output: "PRIVATE DUPLICATE CONTENT",
      exit_code: 0,
      status: "completed",
    },
    {
      id: "allowed-failed",
      type: "command_execution",
      command: `cat ${pathFor(failed)}`,
      aggregated_output: "PRIVATE FAILED CONTENT",
      exit_code: 1,
      status: "failed",
    },
    {
      id: "fabricated-path",
      type: "command_execution",
      command:
        "cat /runtime/codex-home/plugins/cache/ai-plugins/private-plugin/9.9.9/skills/private-skill/SKILL.md",
      aggregated_output: "PRIVATE FABRICATED CONTENT",
      exit_code: 0,
      status: "completed",
    },
  ],
  finalResponse: "PRIVATE FINAL RESPONSE",
  usage: null,
});
fs.writeFileSync(`${resultsFile}.updated`, `${JSON.stringify(raw)}\n`, {
  mode: 0o600,
});
NODE
  chmod 600 "$RAW_RESULTS.updated"
  mv "$RAW_RESULTS.updated" "$RAW_RESULTS"

  run_checker

  [ "$status" -eq 0 ]
  jq -e --arg mode "$mode" '
    .diagnosticEligible == true and
    (.runs[] |
      select(.conditionId == $mode and .sampleIndex == 1) |
      .skillActivationEvidence == "codex-turn-successful-command-path-references" and
      .skillActivations == ["advisor:advisor"]
    )
  ' "$OUTPUT"
  run grep -E 'PRIVATE|private-plugin|spoofed-metadata|SKILL\.md' "$OUTPUT"
  [ "$status" -eq 1 ]
}

@test "checker records valid empty activation evidence as an observed zero" {
  prepare_trusted_runtime
  write_valid_benchmark_inputs
  jq '
    .results.results |= map(
      .response.raw = ({
        items: [{id: "pwd", type: "command_execution", command: "pwd", aggregated_output: "/workspace", exit_code: 0, status: "completed"}],
        finalResponse: "PRIVATE FINAL RESPONSE",
        usage: null
      } | tojson)
    )
  ' "$RAW_RESULTS" >"$RAW_RESULTS.updated"
  chmod 600 "$RAW_RESULTS.updated"
  mv "$RAW_RESULTS.updated" "$RAW_RESULTS"

  run_checker

  [ "$status" -eq 0 ]
  jq -e '
    .diagnosticEligible == true and
    all(.runs[];
      .skillActivationEvidence == "codex-turn-successful-command-path-references" and
      .skillActivations == []
    )
  ' "$OUTPUT"
}

@test "checker makes a completed run provenance-ineligible when raw telemetry is missing" {
  prepare_trusted_runtime
  write_valid_benchmark_inputs
  jq 'del(.results.results[0].response.raw)' \
    "$RAW_RESULTS" >"$RAW_RESULTS.updated"
  chmod 600 "$RAW_RESULTS.updated"
  mv "$RAW_RESULTS.updated" "$RAW_RESULTS"

  run_checker

  [ "$status" -eq 0 ]
  jq -e '
    .diagnosticEligible == false and
    .diagnostics.provenanceFailures == 1 and
    .diagnostics.outcomes.provenanceFailure == 1 and
    (.runs | map(select(.outcomeClass == "provenance-failure")) | length) == 1 and
    (.runs[] | select(.outcomeClass == "provenance-failure") |
      .skillActivations == [] and
      (has("skillActivationEvidence") | not)
    )
  ' "$OUTPUT"
}

@test "checker makes a completed run provenance-ineligible when raw telemetry is malformed" {
  prepare_trusted_runtime
  write_valid_benchmark_inputs
  jq '.results.results[0].response.raw = "{not-json"' \
    "$RAW_RESULTS" >"$RAW_RESULTS.updated"
  chmod 600 "$RAW_RESULTS.updated"
  mv "$RAW_RESULTS.updated" "$RAW_RESULTS"

  run_checker

  [ "$status" -eq 0 ]
  jq -e '
    .diagnosticEligible == false and
    .diagnostics.provenanceFailures == 1 and
    .diagnostics.outcomes.provenanceFailure == 1
  ' "$OUTPUT"
}

@test "checker keeps the outcome taxonomy to nine canonical rows when raw results contain an extra row" {
  prepare_trusted_runtime
  write_valid_benchmark_inputs
  jq '
    .results.results += [
      (.results.results[0] |
        .vars.case_id = "unexpected-private-case" |
        .testCase.vars.case_id = "unexpected-private-case" |
        .response.output = "PRIVATE UNEXPECTED OUTPUT"
      )
    ]
  ' "$RAW_RESULTS" >"$RAW_RESULTS.updated"
  chmod 600 "$RAW_RESULTS.updated"
  mv "$RAW_RESULTS.updated" "$RAW_RESULTS"

  run_checker

  [ "$status" -eq 0 ]
  jq -e '
    .diagnosticEligible == false and
    (.runs | length) == 9 and
    .diagnostics.unexpectedResults == 1 and
    .diagnostics.provenanceFailures == 1 and
    .diagnostics.outcomes == {
      pass: 9,
      candidateFailure: 0,
      safetyFailure: 0,
      operationalFailure: 0,
      provenanceFailure: 0,
      providerFailure: 0
    } and
    ([.diagnostics.outcomes[]] | add) == 9
  ' "$OUTPUT"
  run grep -E 'PRIVATE|unexpected-private' "$OUTPUT"
  [ "$status" -eq 1 ]
}

@test "checker refuses to overwrite a sanitized result" {
  prepare_trusted_runtime
  write_valid_benchmark_inputs
  run_checker
  [ "$status" -eq 0 ]
  before="$(sha256sum "$OUTPUT" | cut -d' ' -f1)"

  run_checker

  [ "$status" -eq 2 ]
  [ "$output" = "code-quality-results:output-already-exists" ]
  [ "$(sha256sum "$OUTPUT" | cut -d' ' -f1)" = "$before" ]
}

@test "checker rejects a hard-linked trusted input without writing output" {
  prepare_trusted_runtime
  write_valid_benchmark_inputs
  outside="$TEST_ROOT/provenance-outside.json"
  mv "$PROVENANCE" "$outside"
  ln "$outside" "$PROVENANCE"

  run_checker

  [ "$status" -eq 2 ]
  [ "$output" = "code-quality-results:provenance-invalid" ]
  [ ! -e "$OUTPUT" ]
  [[ "$output" != *"$outside"* ]]
  [[ "$output" != *"$PROVENANCE"* ]]
}

@test "checker requires the runtime manifest inside the owned run root" {
  prepare_trusted_runtime
  write_valid_benchmark_inputs
  outside_runtime="$TEST_ROOT/outside-runtime"
  node "$ROOT/scripts/evals/prepare-code-quality-runtime.mjs" \
    "$WORKSPACE_MANIFEST" "$outside_runtime" >/dev/null

  run node "$CHECKER" \
    --results "$RAW_RESULTS" \
    --artifacts "$ARTIFACT_ROOT" \
    --runtime-manifest "$outside_runtime/manifest.json" \
    --provenance "$PROVENANCE" \
    --output "$OUTPUT"

  [ "$status" -eq 2 ]
  [ "$output" = "code-quality-results:run-layout-invalid" ]
  [ ! -e "$OUTPUT" ]
  [[ "$output" != *"$outside_runtime"* ]]
}

@test "checker requires the workspace manifest inside the owned run root" {
  prepare_external_workspace_runtime
  write_valid_benchmark_inputs

  run_checker

  [ "$status" -eq 2 ]
  [ "$output" = "code-quality-results:run-layout-invalid" ]
  [ ! -e "$OUTPUT" ]
  [[ "$output" != *"$WORKSPACE_MANIFEST"* ]]
}

@test "checker preserves a classified boundary safety failure without its detail" {
  prepare_trusted_runtime
  write_valid_benchmark_inputs
  mode="$(jq -r '.rows[0].mode' "$RUNTIME_MANIFEST")"
  rm "$ARTIFACT_ROOT/rust-cli-feature/sample-1/$mode.json"
  jq --arg mode "$mode" '
    .results.results |= map(
      if .vars.sample_index == 1 and .vars.condition_id == $mode then
        .success = false |
        .score = 0 |
        .failureReason = 2 |
        .gradingResult.pass = false |
        .gradingResult.score = 0 |
        .error = "CODE_QUALITY_BOUNDARY_ERROR:safety:output-limit-exceeded"
      else . end
    )
  ' "$RAW_RESULTS" >"$RAW_RESULTS.updated"
  chmod 600 "$RAW_RESULTS.updated"
  mv "$RAW_RESULTS.updated" "$RAW_RESULTS"

  run_checker

  [ "$status" -eq 0 ]
  jq -e --arg mode "$mode" '
    .diagnosticEligible == false and
    .diagnostics.safetyFailures == 1 and
    .diagnostics.operationalFailures == 0 and
    (.runs[] |
      select(.conditionId == $mode and .sampleIndex == 1) |
      .complete == true and
      .outcomeClass == "safety-failure"
    )
  ' "$OUTPUT"
  run grep -E 'CODE_QUALITY_BOUNDARY_ERROR|output-limit-exceeded' "$OUTPUT"
  [ "$status" -eq 1 ]
}

@test "checker keeps an unprefixed bound provider error distinct from operational failures" {
  prepare_trusted_runtime
  write_valid_benchmark_inputs
  mode="$(jq -r '.rows[0].mode' "$RUNTIME_MANIFEST")"
  rm "$ARTIFACT_ROOT/rust-cli-feature/sample-1/$mode.json"
  jq --arg mode "$mode" '
    .results.results |= map(
      if .vars.sample_index == 1 and .vars.condition_id == $mode then
        .success = false |
        .score = 0 |
        .failureReason = 2 |
        .gradingResult.pass = false |
        .gradingResult.score = 0 |
        .error = "Error calling provider: PRIVATE upstream API outage" |
        del(.latencyMs, .cost, .tokenUsage)
      else . end
    )
  ' "$RAW_RESULTS" >"$RAW_RESULTS.updated"
  chmod 600 "$RAW_RESULTS.updated"
  mv "$RAW_RESULTS.updated" "$RAW_RESULTS"

  run_checker

  [ "$status" -eq 0 ]
  jq -e --arg mode "$mode" '
    .diagnosticEligible == false and
    .diagnostics.completeRuns == 8 and
    .diagnostics.providerFailures == 1 and
    .diagnostics.operationalFailures == 0 and
    .diagnostics.provenanceFailures == 0 and
    .diagnostics.safetyFailures == 0 and
    .diagnostics.outcomes.providerFailure == 1 and
    (.runs[] |
      select(.conditionId == $mode and .sampleIndex == 1) |
      .complete == false and
      .outcomeClass == "provider-failure"
    )
  ' "$OUTPUT"
  run grep -E 'PRIVATE|upstream|API outage|Error calling provider' "$OUTPUT"
  [ "$status" -eq 1 ]
}

@test "checker defaults a non-provider Promptfoo error to operational failure" {
  prepare_trusted_runtime
  write_valid_benchmark_inputs
  mode="$(jq -r '.rows[0].mode' "$RUNTIME_MANIFEST")"
  rm "$ARTIFACT_ROOT/rust-cli-feature/sample-1/$mode.json"
  jq --arg mode "$mode" '
    .results.results |= map(
      if .vars.sample_index == 1 and .vars.condition_id == $mode then
        .success = false |
        .score = 0 |
        .failureReason = 2 |
        .gradingResult.pass = false |
        .gradingResult.score = 0 |
        .error = "PRIVATE evaluator worker crashed" |
        del(.latencyMs, .cost, .tokenUsage)
      else . end
    )
  ' "$RAW_RESULTS" >"$RAW_RESULTS.updated"
  chmod 600 "$RAW_RESULTS.updated"
  mv "$RAW_RESULTS.updated" "$RAW_RESULTS"

  run_checker

  [ "$status" -eq 0 ]
  jq -e --arg mode "$mode" '
    .diagnosticEligible == false and
    .diagnostics.completeRuns == 8 and
    .diagnostics.providerFailures == 0 and
    .diagnostics.operationalFailures == 1 and
    (.runs[] |
      select(.conditionId == $mode and .sampleIndex == 1) |
      .complete == false and
      .outcomeClass == "operational-failure"
    )
  ' "$OUTPUT"
  run grep -E 'PRIVATE|evaluator worker crashed' "$OUTPUT"
  [ "$status" -eq 1 ]
}

@test "checker rejects disagreement between Promptfoo result and test-case bindings" {
  prepare_trusted_runtime
  write_valid_benchmark_inputs
  mode="$(jq -r '.rows[0].mode' "$RUNTIME_MANIFEST")"
  jq --arg mode "$mode" '
    .results.results |= map(
      if .vars.sample_index == 1 and .vars.condition_id == $mode then
        .testCase.vars.run_id = ("0" * 64) |
        .testCase.vars.scenario_prompt = "PRIVATE SUBSTITUTED PROMPT"
      else . end
    )
  ' "$RAW_RESULTS" >"$RAW_RESULTS.updated"
  chmod 600 "$RAW_RESULTS.updated"
  mv "$RAW_RESULTS.updated" "$RAW_RESULTS"

  run_checker

  [ "$status" -eq 0 ]
  jq -e --arg mode "$mode" '
    .diagnosticEligible == false and
    .diagnostics.provenanceFailures == 1 and
    (.runs[] |
      select(.conditionId == $mode and .sampleIndex == 1) |
      .complete == false and
      .outcomeClass == "provenance-failure"
    )
  ' "$OUTPUT"
  run grep -E 'PRIVATE|SUBSTITUTED PROMPT' "$OUTPUT"
  [ "$status" -eq 1 ]
}

@test "checker rejects an artifact whose pass outcome contradicts its gates" {
  prepare_trusted_runtime
  write_valid_benchmark_inputs
  mode="$(jq -r '.rows[0].mode' "$RUNTIME_MANIFEST")"
  artifact="$ARTIFACT_ROOT/rust-cli-feature/sample-1/$mode.json"
  jq '.gates.safety = false' "$artifact" >"$artifact.updated"
  chmod 600 "$artifact.updated"
  mv "$artifact.updated" "$artifact"

  run_checker

  [ "$status" -eq 0 ]
  jq -e --arg mode "$mode" '
    .diagnosticEligible == false and
    .diagnostics.provenanceFailures == 1 and
    (.runs[] |
      select(.conditionId == $mode and .sampleIndex == 1) |
      .outcomeClass == "provenance-failure" and
      .pass == false
    )
  ' "$OUTPUT"
}

@test "checker binds every artifact to the exact trusted verifier composition" {
  prepare_trusted_runtime
  write_valid_benchmark_inputs
  while IFS= read -r artifact; do
    jq '.verifierCompositionSha256 = ("0" * 64)' "$artifact" \
      >"$artifact.updated"
    chmod 600 "$artifact.updated"
    mv "$artifact.updated" "$artifact"
  done < <(find "$ARTIFACT_ROOT" -type f -name '*.json' -print)

  run_checker

  [ "$status" -eq 0 ]
  jq -e '
    .diagnosticEligible == false and
    .diagnostics.provenanceFailures == 9 and
    .diagnostics.outcomes.provenanceFailure == 9 and
    all(.runs[];
      .pass == false and
      .outcomeClass == "provenance-failure"
    )
  ' "$OUTPUT"
}

@test "checker excludes incomplete verifier passes from reliability aggregates" {
  prepare_trusted_runtime
  write_valid_benchmark_inputs
  jq '
    .results.results |= map(del(.latencyMs, .cost, .tokenUsage))
  ' "$RAW_RESULTS" >"$RAW_RESULTS.updated"
  chmod 600 "$RAW_RESULTS.updated"
  mv "$RAW_RESULTS.updated" "$RAW_RESULTS"

  run_checker

  [ "$status" -eq 0 ]
  jq -e '
    .diagnosticEligible == false and
    .diagnostics.completeRuns == 0 and
    .diagnostics.outcomes.pass == 9 and
    all(.runs[]; .complete == false and .pass == true) and
    all(.aggregates[];
      .sampleCount == 3 and
      .successCount == 0 and
      .successRate == 0 and
      .passAt3Capability == 0 and
      .passPower3Reliability == 0
    )
  ' "$OUTPUT"
}
