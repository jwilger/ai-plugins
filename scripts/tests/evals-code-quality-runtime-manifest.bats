#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  WORKSPACE_PREPARER="$ROOT/scripts/evals/prepare-code-quality-workspaces.mjs"
  RUNTIME_PREPARER="$ROOT/scripts/evals/prepare-code-quality-runtime.mjs"
  CODEX_RESOLVER="$ROOT/scripts/evals/resolve-code-quality-codex.mjs"
  CASE_LOADER="$ROOT/evals/benchmarks/downstream-code-quality/cases.cjs"
  BENCHMARK_INPUTS="$ROOT/evals/benchmarks/downstream-code-quality/benchmark-inputs.cjs"
  PROMPTFOO_CONFIG="$ROOT/evals/benchmarks/downstream-code-quality/promptfooconfig.yaml"
  ASSERTION="$ROOT/evals/benchmarks/downstream-code-quality/assertions/expense-report.cjs"
  TEST_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/ai-plugins-runtime-manifest.XXXXXX")"
  WORK_ROOT="$TEST_ROOT/workspaces"
  RUNTIME_ROOT="$TEST_ROOT/host-tmp/runtime"
  ARTIFACT_ROOT="$TEST_ROOT/artifacts"

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

  printf 'ai-plugins downstream code-quality run root\n' \
    >"$TEST_ROOT/.ai-plugins-code-quality-run-root"
  chmod 600 "$TEST_ROOT/.ai-plugins-code-quality-run-root"
  mkdir -m 700 "$TEST_ROOT/host-tmp"
  mkdir -m 700 "$ARTIFACT_ROOT"

  node "$WORKSPACE_PREPARER" "$WORK_ROOT" \
    --case rust-cli-feature --samples 1 >/dev/null
  WORKSPACE_MANIFEST="$WORK_ROOT/manifest.json"
}

@test "shared prompt renderer is structurally identical to the Promptfoo wrapper" {
  run node - "$BENCHMARK_INPUTS" "$PROMPTFOO_CONFIG" <<'NODE'
const inputs = require(process.argv[2]);
const surface = inputs.loadPromptfooSurface(process.argv[3]);
if (surface.promptTemplate !== inputs.promptTemplate) {
  throw new Error("prompt-template-drift");
}
const scenario = inputs.promptFor({ caseId: "rust-cli-feature" });
const rendered = inputs.renderPrompt(scenario);
if (rendered !== surface.promptTemplate.replace("{{ scenario_prompt }}", scenario)) {
  throw new Error("rendered-prompt-drift");
}
process.stdout.write(rendered);
NODE

  [ "$status" -eq 0 ]
  [[ "$output" == "Complete this coding task"* ]]
  [[ "$output" == *"Run formatting, clippy with warnings denied"* ]]
  ! grep -Eiq '\b(eval(uation)?|disposable|treatment|condition)\b' \
    <<<"$output"
  [[ "$output" != *"marketplace"* ]]
}

@test "Promptfoo surface rejects noncanonical top-level execution controls" {
  run node - "$BENCHMARK_INPUTS" "$PROMPTFOO_CONFIG" "$TEST_ROOT" <<'NODE'
const fs = require("node:fs");
const path = require("node:path");
const YAML = require("yaml");
const inputs = require(process.argv[2]);
const source = YAML.parse(fs.readFileSync(process.argv[3], "utf8"));
const temporaryRoot = process.argv[4];
const mutations = [
  ["description", (value) => { value.description = "Different benchmark"; }],
  ["tests-loader", (value) => { value.tests = "file://different-cases.cjs"; }],
  ["tracing", (value) => { value.tracing.enabled = true; }],
  ["metadata", (value) => { value.metadata.benchmark = "different"; }],
  ["concurrency", (value) => { value.commandLineOptions.maxConcurrency = 2; }],
  ["sharing", (value) => { value.commandLineOptions.share = true; }],
  ["cache", (value) => { value.commandLineOptions.cache = true; }],
  ["writes", (value) => { value.commandLineOptions.write = true; }],
  ["extra-key", (value) => { value.unexpected = true; }],
];

const accepted = [];
for (const [name, mutate] of mutations) {
  const candidate = structuredClone(source);
  mutate(candidate);
  const file = path.join(temporaryRoot, `promptfoo-${name}.yaml`);
  fs.writeFileSync(file, YAML.stringify(candidate));
  try {
    inputs.loadPromptfooSurface(file);
    accepted.push(name);
  } catch {}
}
if (accepted.length > 0) {
  throw new Error(`accepted-noncanonical-top-level:${accepted.join(",")}`);
}
NODE

  [ "$status" -eq 0 ]
}

@test "Promptfoo surface rejects noncanonical Codex provider controls" {
  run node - "$BENCHMARK_INPUTS" "$PROMPTFOO_CONFIG" "$TEST_ROOT" <<'NODE'
const fs = require("node:fs");
const path = require("node:path");
const YAML = require("yaml");
const inputs = require(process.argv[2]);
const source = YAML.parse(fs.readFileSync(process.argv[3], "utf8"));
const temporaryRoot = process.argv[4];
const first = (value) => value.providers[0].config;
const targeted = (value) => value.providers[1].config;
const mutations = [
  ["provider-order", (value) => { value.providers.reverse(); }],
  ["model", (value) => { first(value).model = "different-model"; }],
  ["reasoning", (value) => { first(value).model_reasoning_effort = "low"; }],
  ["working-dir", (value) => { first(value).working_dir = "/tmp"; }],
  ["sandbox", (value) => { first(value).sandbox_mode = "danger-full-access"; }],
  ["approval", (value) => { first(value).approval_policy = "on-request"; }],
  ["network", (value) => { first(value).network_access_enabled = true; }],
  ["web-search", (value) => { first(value).web_search_enabled = true; }],
  ["web-search-mode", (value) => { first(value).web_search_mode = "live"; }],
  ["git-check", (value) => { first(value).skip_git_repo_check = true; }],
  ["persistence", (value) => { first(value).persist_threads = true; }],
  ["codex-path", (value) => { first(value).codex_path_override = "/usr/bin/codex"; }],
  ["process-env", (value) => { first(value).inherit_process_env = true; }],
  ["streaming", (value) => { first(value).enable_streaming = true; }],
  ["tracing", (value) => { first(value).deep_tracing = true; }],
  ["missing-cli-env", (value) => { delete first(value).cli_env.CODEX_HOME; }],
  ["extra-cli-env", (value) => { first(value).cli_env.OPENAI_API_KEY = "{{ env.OPENAI_API_KEY }}"; }],
  ["changed-cli-env", (value) => { first(value).cli_env.CODE_QUALITY_TOOL_PATH = "/usr/bin"; }],
  ["cli-web-search", (value) => { first(value).cli_config.web_search = "live"; }],
  ["cli-network", (value) => { first(value).cli_config.sandbox_workspace_write.network_access = true; }],
  ["cli-writable-root", (value) => { first(value).cli_config.sandbox_workspace_write.writable_roots = ["/tmp"]; }],
  ["shell-inherit", (value) => { first(value).cli_config.shell_environment_policy.inherit = "all"; }],
  ["shell-path", (value) => { first(value).cli_config.shell_environment_policy.set.PATH = "/usr/bin"; }],
  ["guardian", (value) => { first(value).cli_config.features.guardian_approval = false; }],
  ["plugins-condition", (value) => { targeted(value).cli_config.features.plugins = false; }],
  ["extra-provider-key", (value) => { first(value).unexpected = true; }],
];

const accepted = [];
for (const [name, mutate] of mutations) {
  const candidate = structuredClone(source);
  mutate(candidate);
  const file = path.join(temporaryRoot, `provider-${name}.yaml`);
  fs.writeFileSync(file, YAML.stringify(candidate));
  try {
    inputs.loadPromptfooSurface(file);
    accepted.push(name);
  } catch {}
}
if (accepted.length > 0) {
  throw new Error(`accepted-noncanonical-provider:${accepted.join(",")}`);
}
NODE

  [ "$status" -eq 0 ]
}

teardown() {
  rm -rf "$TEST_ROOT"
}

prepare_runtime() {
  node "$RUNTIME_PREPARER" "$WORKSPACE_MANIFEST" "$RUNTIME_ROOT" >/dev/null
  RUNTIME_MANIFEST="$RUNTIME_ROOT/manifest.json"
}

@test "case loading requires a one-time runtime manifest" {
  run env \
    CODE_QUALITY_WORKSPACE_MANIFEST="$WORKSPACE_MANIFEST" \
    EVAL_CASE_FILTER=rust-cli-feature \
    node -e 'require(process.argv[1])()' "$CASE_LOADER"

  [ "$status" -ne 0 ]
  [[ "$output" == *"CODE_QUALITY_RUNTIME_MANIFEST"* ]]
}

@test "case loading binds every turn to its private runtime and evidence hashes" {
  prepare_runtime

  run env \
    CODE_QUALITY_WORKSPACE_MANIFEST="$WORKSPACE_MANIFEST" \
    CODE_QUALITY_RUNTIME_MANIFEST="$RUNTIME_MANIFEST" \
    EVAL_CASE_FILTER=rust-cli-feature \
    node -e 'process.stdout.write(JSON.stringify(require(process.argv[1])()))' \
    "$CASE_LOADER"

  [ "$status" -eq 0 ]
  [ "$(jq 'length' <<<"$output")" -eq 3 ]
  jq -e \
    --slurpfile runtime "$RUNTIME_MANIFEST" '
      ($runtime[0]) as $manifest |
      all(.[];
        (.vars) as $vars |
        ($manifest.rows[] |
          select(
            .caseId == $vars.case_id and
            .sample == $vars.sample_index and
            .mode == $vars.condition_id
          )
        ) as $row |
        $vars.codex_home == $row.codexHome and
        $vars.codex_tmp == $row.codexTmp and
        $vars.run_id == $manifest.runId and
        $vars.contract_sha256 == $manifest.contractSha256 and
        $vars.workspace_manifest_sha256 == $manifest.workspaceManifestSha256 and
        ($vars.runtime_manifest_sha256 | test("^[0-9a-f]{64}$")) and
        $vars.matrix_hash == $manifest.matrixHash and
        $vars.fixture_digest == $row.fixtureDigest and
        $vars.input_hash == $row.inputHash and
        $vars.composition_hash == $row.compositionHash and
        $vars.available_skills == $row.availableSkills
      )
    ' <<<"$output"
  [ "$(jq '[.[].vars.codex_home] | unique | length' <<<"$output")" -eq 3 ]
  [ "$(jq '[.[].vars.codex_tmp] | unique | length' <<<"$output")" -eq 3 ]
}

@test "runtime manifest validation rejects a row from a different run" {
  prepare_runtime
  tampered="$TEST_ROOT/tampered.json"
  jq '.rows[0].runId = ("0" * 64)' "$RUNTIME_MANIFEST" >"$tampered"
  chmod 600 "$tampered"
  mv "$tampered" "$RUNTIME_MANIFEST"

  run env \
    CODE_QUALITY_WORKSPACE_MANIFEST="$WORKSPACE_MANIFEST" \
    CODE_QUALITY_RUNTIME_MANIFEST="$RUNTIME_MANIFEST" \
    EVAL_CASE_FILTER=rust-cli-feature \
    node -e 'require(process.argv[1])()' "$CASE_LOADER"

  [ "$status" -ne 0 ]
  [[ "$output" == *"runtime row run identity does not match"* ]]
}

@test "runtime evidence rejects changed config, projected skills, and injected credentials" {
  prepare_runtime
  printf '\n# host-side config mutation\n' \
    >>"$RUNTIME_ROOT/rust-cli-feature/sample-1/no-marketplace-skills/codex-home/config.toml"

  run env \
    CODE_QUALITY_WORKSPACE_MANIFEST="$WORKSPACE_MANIFEST" \
    CODE_QUALITY_RUNTIME_MANIFEST="$RUNTIME_MANIFEST" \
    EVAL_CASE_FILTER=rust-cli-feature \
    node -e 'require(process.argv[1])()' "$CASE_LOADER"

  [ "$status" -ne 0 ]
  [[ "$output" == *"runtime config invalid"* ]]

  rm -rf "$RUNTIME_ROOT"
  prepare_runtime
  skill_file="$(find \
    "$RUNTIME_ROOT/rust-cli-feature/sample-1/targeted-quality-skills/codex-home/plugins/cache/ai-plugins/advisor" \
    -path '*/skills/*/SKILL.md' -type f -print -quit)"
  [ -n "$skill_file" ]
  printf '\nHost-side projected skill mutation.\n' >>"$skill_file"

  run env \
    CODE_QUALITY_WORKSPACE_MANIFEST="$WORKSPACE_MANIFEST" \
    CODE_QUALITY_RUNTIME_MANIFEST="$RUNTIME_MANIFEST" \
    EVAL_CASE_FILTER=rust-cli-feature \
    node -e 'require(process.argv[1])()' "$CASE_LOADER"

  [ "$status" -ne 0 ]
  [[ "$output" == *"runtime plugin projections differ"* ]]

  rm -rf "$RUNTIME_ROOT"
  prepare_runtime
  printf '{"token":"must-never-be-accepted"}\n' \
    >"$RUNTIME_ROOT/rust-cli-feature/sample-1/no-marketplace-skills/codex-home/auth.json"

  run env \
    CODE_QUALITY_WORKSPACE_MANIFEST="$WORKSPACE_MANIFEST" \
    CODE_QUALITY_RUNTIME_MANIFEST="$RUNTIME_MANIFEST" \
    EVAL_CASE_FILTER=rust-cli-feature \
    node -e 'require(process.argv[1])()' "$CASE_LOADER"

  [ "$status" -ne 0 ]
  [[ "$output" == *"runtime disposable auth invalid"* ]]
}

@test "runtime evidence only advertises skill directories backed by SKILL.md" {
  prepare_runtime
  skill_file="$(find \
    "$RUNTIME_ROOT/rust-cli-feature/sample-1/targeted-quality-skills/codex-home/plugins/cache/ai-plugins/advisor" \
    -path '*/skills/*/SKILL.md' -type f -print -quit)"
  [ -n "$skill_file" ]
  mv "$skill_file" "${skill_file%/SKILL.md}/NOT-A-SKILL.md"

  run env \
    CODE_QUALITY_WORKSPACE_MANIFEST="$WORKSPACE_MANIFEST" \
    CODE_QUALITY_RUNTIME_MANIFEST="$RUNTIME_MANIFEST" \
    EVAL_CASE_FILTER=rust-cli-feature \
    node -e 'require(process.argv[1])()' "$CASE_LOADER"

  [ "$status" -ne 0 ]
  [[ "$output" == *"projected skill is missing skill md"* ]]
}

@test "runtime evidence rejects a top-level skills file outside every SKILL.md-backed skill" {
  prepare_runtime
  codex_home="$RUNTIME_ROOT/rust-cli-feature/sample-1/targeted-quality-skills/codex-home"
  skills_root="$(find \
    "$codex_home/plugins/cache/ai-plugins/advisor" \
    -mindepth 2 -maxdepth 2 -type d -name skills -print -quit)"
  [ -n "$skills_root" ]
  printf 'not a skill runtime file\n' >"$skills_root/extra.txt"

  run node "$ROOT/scripts/evals/code-quality-runtime-evidence.mjs" \
    --codex-home "$codex_home" \
    --mode targeted-quality-skills \
    --phase pre-turn

  [ "$status" -eq 2 ]
  [ "$output" = \
    code-quality-runtime-evidence:provenance:runtime-projection-file-set-is-not-exact ]
}

@test "post-turn evidence rejects all host Codex-home mutations including arg0 helpers" {
  prepare_runtime
  codex_home="$RUNTIME_ROOT/rust-cli-feature/sample-1/no-marketplace-skills/codex-home"
  arg0="$codex_home/tmp/arg0/codex-arg0fixture"
  mkdir -p "$arg0"
  : >"$arg0/.lock"
  for helper in \
    apply_patch \
    applypatch \
    codex-execve-wrapper \
    codex-linux-sandbox; do
    ln -s /dev/null "$arg0/$helper"
  done

  run env \
    CODE_QUALITY_WORKSPACE_MANIFEST="$WORKSPACE_MANIFEST" \
    CODE_QUALITY_RUNTIME_MANIFEST="$RUNTIME_MANIFEST" \
    node - "$ROOT/evals/benchmarks/downstream-code-quality/runtime-manifest.cjs" <<'NODE'
const { loadRuntimeManifest } = require(process.argv[2]);
process.stdout.write(String(loadRuntimeManifest({ phase: "post-turn" }).rows.length));
NODE

  [ "$status" -ne 0 ]
  [[ "$output" == *"runtime projection"* ]]

  run env \
    CODE_QUALITY_WORKSPACE_MANIFEST="$WORKSPACE_MANIFEST" \
    CODE_QUALITY_RUNTIME_MANIFEST="$RUNTIME_MANIFEST" \
    EVAL_CASE_FILTER=rust-cli-feature \
    node -e 'require(process.argv[1])()' "$CASE_LOADER"

  [ "$status" -ne 0 ]
  [[ "$output" == *"runtime projection"* ]]

  rm -rf "$codex_home/tmp"
  printf 'unexpected persistent history\n' >"$codex_home/history.jsonl"
  run env \
    CODE_QUALITY_WORKSPACE_MANIFEST="$WORKSPACE_MANIFEST" \
    CODE_QUALITY_RUNTIME_MANIFEST="$RUNTIME_MANIFEST" \
    node - "$ROOT/evals/benchmarks/downstream-code-quality/runtime-manifest.cjs" <<'NODE'
const { loadRuntimeManifest } = require(process.argv[2]);
loadRuntimeManifest({ phase: "post-turn" });
NODE

  [ "$status" -ne 0 ]
  [[ "$output" == *"runtime projection file set is not exact"* ]]
}

@test "runtime manifest classifies missing workspace infrastructure as operational" {
  prepare_runtime
  mv "$WORKSPACE_MANIFEST" "$TEST_ROOT/workspace-manifest.saved"

  run env \
    CODE_QUALITY_WORKSPACE_MANIFEST="$WORKSPACE_MANIFEST" \
    CODE_QUALITY_RUNTIME_MANIFEST="$RUNTIME_MANIFEST" \
    node - "$ROOT/evals/benchmarks/downstream-code-quality/runtime-manifest.cjs" <<'NODE'
const { loadRuntimeManifest, RuntimeManifestError } = require(process.argv[2]);
try {
  loadRuntimeManifest({ phase: "post-turn" });
} catch (error) {
  if (!(error instanceof RuntimeManifestError)) throw error;
  process.stdout.write(`${error.category}:${error.code}`);
}
NODE

  [ "$status" -eq 0 ]
  [ "$output" = operational:workspace-manifest-unavailable ]
}

@test "runtime manifest classifies content tampering as provenance" {
  prepare_runtime
  printf '\n# tampered\n' \
    >>"$RUNTIME_ROOT/rust-cli-feature/sample-1/no-marketplace-skills/codex-home/config.toml"

  run env \
    CODE_QUALITY_WORKSPACE_MANIFEST="$WORKSPACE_MANIFEST" \
    CODE_QUALITY_RUNTIME_MANIFEST="$RUNTIME_MANIFEST" \
    node - "$ROOT/evals/benchmarks/downstream-code-quality/runtime-manifest.cjs" <<'NODE'
const { loadRuntimeManifest, RuntimeManifestError } = require(process.argv[2]);
try {
  loadRuntimeManifest({ phase: "post-turn" });
} catch (error) {
  if (!(error instanceof RuntimeManifestError)) throw error;
  process.stdout.write(`${error.category}:${error.code}`);
}
NODE

  [ "$status" -eq 0 ]
  [ "$output" = provenance:runtime-config-invalid ]
}

@test "post-turn assertion classifies missing runtime infrastructure separately from tampering" {
  prepare_runtime

  run env \
    CODE_QUALITY_WORKSPACE_MANIFEST="$WORKSPACE_MANIFEST" \
    CODE_QUALITY_RUNTIME_MANIFEST="$RUNTIME_MANIFEST" \
    CODE_QUALITY_VERIFIER_OUT_ROOT="$ARTIFACT_ROOT" \
    EVAL_CASE_FILTER=rust-cli-feature \
    node - "$CASE_LOADER" "$ASSERTION" <<'NODE'
const fs = require("node:fs");
const loadCases = require(process.argv[2]);
const assertion = require(process.argv[3]);
const testCase = loadCases()[0];
fs.unlinkSync(process.env.CODE_QUALITY_RUNTIME_MANIFEST);
try {
  assertion("", {
    provider: { label: testCase.providers[0] },
    vars: testCase.vars,
  });
} catch (error) {
  process.stdout.write(error.message);
}
NODE

  [ "$status" -eq 0 ]
  [ "$output" = operational-failure:runtime-manifest-unavailable ]
}

@test "post-turn assertion rejects an artifact directory from another run" {
  prepare_runtime
  other_artifacts="$TEST_ROOT/other-artifacts"
  mkdir -m 700 "$other_artifacts"

  run env \
    CODE_QUALITY_WORKSPACE_MANIFEST="$WORKSPACE_MANIFEST" \
    CODE_QUALITY_RUNTIME_MANIFEST="$RUNTIME_MANIFEST" \
    CODE_QUALITY_VERIFIER_OUT_ROOT="$other_artifacts" \
    EVAL_CASE_FILTER=rust-cli-feature \
    node - "$CASE_LOADER" "$ASSERTION" <<'NODE'
const loadCases = require(process.argv[2]);
const assertion = require(process.argv[3]);
const testCase = loadCases()[0];
try {
  assertion("", {
    provider: { label: testCase.providers[0] },
    vars: testCase.vars,
  });
} catch (error) {
  process.stdout.write(error.message);
}
NODE

  [ "$status" -eq 0 ]
  [ "$output" = provenance-failure:output-root-unbound ]
  [ -z "$(find "$other_artifacts" -mindepth 1 -print -quit)" ]
}

@test "post-turn assertion rejects stale execution vars before invoking the scorer" {
  prepare_runtime

  run env \
    CODE_QUALITY_WORKSPACE_MANIFEST="$WORKSPACE_MANIFEST" \
    CODE_QUALITY_RUNTIME_MANIFEST="$RUNTIME_MANIFEST" \
    EVAL_CASE_FILTER=rust-cli-feature \
    node - "$CASE_LOADER" "$ASSERTION" <<'NODE'
const loadCases = require(process.argv[2]);
const assertion = require(process.argv[3]);
const testCase = loadCases()[0];
try {
  assertion("", {
    provider: { label: testCase.providers[0] },
    vars: { ...testCase.vars, workspace: "/tmp/substituted-workspace" },
  });
} catch (error) {
  process.stdout.write(error.message);
}
NODE

  [ "$status" -eq 0 ]
  [ "$output" = provenance-failure:workspace-binding-invalid ]
}
