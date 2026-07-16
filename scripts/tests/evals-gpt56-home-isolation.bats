#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  PROVIDER_TEST_ROOT="$(mktemp -d)"
  GPT56_PROVIDER_TEST_WORKSPACE="$PROVIDER_TEST_ROOT/workspace"
  mkdir "$GPT56_PROVIDER_TEST_WORKSPACE"
  printf 'ai-plugins GPT-5.6 benchmark workspace\n' \
    >"$GPT56_PROVIDER_TEST_WORKSPACE/.ai-plugins-gpt56-workspace"
  export GPT56_PROVIDER_TEST_WORKSPACE
}

teardown() {
  rm -rf "$PROVIDER_TEST_ROOT"
}

assert_provider_rejects_working_dir() {
  local mode="$1"
  local working_dir="${2-}"
  local codex_home="$3"

  node --input-type=module - "$ROOT" "$mode" "$working_dir" "$codex_home" <<'NODE'
import { pathToFileURL } from 'node:url';

const [root, mode, workingDir, codexHome] = process.argv.slice(2);
const { default: TraceEnforcedCodexProvider } = await import(
  pathToFileURL(
    `${root}/evals/benchmarks/gpt-5.6-model-family/trace-enforced-codex-provider.mjs`,
  )
);

const config = {
  cli_env: { CODEX_HOME: codexHome },
};
if (mode !== 'missing') {
  config.working_dir = workingDir;
}

let loadCount = 0;
const provider = new TraceEnforcedCodexProvider(
  { config },
  async () => {
    loadCount += 1;
    throw new Error('providerLoader was reached');
  },
);

let rejection;
try {
  await provider.callApi('Answer directly.');
} catch (error) {
  rejection = error;
}

if (!rejection || !rejection.message.includes('working_dir')) {
  throw new Error(
    `invalid working_dir was not rejected by the workspace guard: ${rejection?.message ?? 'no rejection'}`,
  );
}
if (loadCount !== 0) {
  throw new Error(`providerLoader ran ${loadCount} time(s) for invalid working_dir`);
}
NODE
}

@test "Codex home preparation initializes an empty unmarked directory in place" {
  temp_root="$(mktemp -d)"
  eval_home="$temp_root/codex-home"
  mkdir "$eval_home"
  chmod 0711 "$eval_home"
  setfacl -m d:u::rwx,d:g::r-x,d:o::--- "$eval_home"
  original_inode="$(stat -c %i "$eval_home")"
  original_acl="$(getfacl -cp "$eval_home")"

  run env OPENAI_API_KEY=fixture node \
    "$ROOT/scripts/evals/prepare-codex-home.mjs" \
    "$eval_home" \
    --plugin-mode no-plugins

  [ "$status" -eq 0 ]
  [ "$(stat -c %i "$eval_home")" = "$original_inode" ]
  [ "$(stat -c %a "$eval_home")" = "711" ]
  [ "$(getfacl -cp "$eval_home")" = "$original_acl" ]
  [ -f "$eval_home/.ai-plugins-eval-home" ]

  rm -rf "$temp_root"
}

@test "GPT-5.6 workspace preparation initializes an empty unmarked directory in place" {
  temp_root="$(mktemp -d)"
  workspace="$temp_root/workspace"
  mkdir "$workspace"
  chmod 0711 "$workspace"
  setfacl -m d:u::rwx,d:g::r-x,d:o::--- "$workspace"
  original_inode="$(stat -c %i "$workspace")"
  original_acl="$(getfacl -cp "$workspace")"

  run node \
    "$ROOT/scripts/evals/prepare-gpt56-workspace.mjs" \
    "$workspace"

  [ "$status" -eq 0 ]
  [ "$(stat -c %i "$workspace")" = "$original_inode" ]
  [ "$(stat -c %a "$workspace")" = "711" ]
  [ "$(getfacl -cp "$workspace")" = "$original_acl" ]
  [ -f "$workspace/.ai-plugins-gpt56-workspace" ]

  rm -rf "$temp_root"
}

@test "trace-enforced Codex provider rejects a missing working_dir before loading app-server" {
  temp_root="$(mktemp -d)"

  run assert_provider_rejects_working_dir \
    missing "" "$temp_root/codex-home"

  rm -rf "$temp_root"
  [ "$status" -eq 0 ]
}

@test "trace-enforced Codex provider rejects an unresolved working_dir before loading app-server" {
  temp_root="$(mktemp -d)"

  run assert_provider_rejects_working_dir \
    configured '{{ env.GPT56_BENCHMARK_WORKSPACE }}' "$temp_root/codex-home"

  rm -rf "$temp_root"
  [ "$status" -eq 0 ]
}

@test "trace-enforced Codex provider rejects a relative working_dir before loading app-server" {
  temp_root="$(mktemp -d)"

  run assert_provider_rejects_working_dir \
    configured relative/workspace "$temp_root/codex-home"

  rm -rf "$temp_root"
  [ "$status" -eq 0 ]
}

@test "trace-enforced Codex provider rejects an unmarked absolute working_dir before loading app-server" {
  temp_root="$(mktemp -d)"
  workspace="$temp_root/workspace"
  mkdir "$workspace"

  run assert_provider_rejects_working_dir \
    configured "$workspace" "$temp_root/codex-home"

  rm -rf "$temp_root"
  [ "$status" -eq 0 ]
}

@test "trace-enforced Codex provider rejects an inexact workspace marker before loading app-server" {
  temp_root="$(mktemp -d)"
  workspace="$temp_root/workspace"
  mkdir "$workspace"
  printf 'ai-plugins GPT-5.6 benchmark workspace' \
    >"$workspace/.ai-plugins-gpt56-workspace"

  run assert_provider_rejects_working_dir \
    configured "$workspace" "$temp_root/codex-home"

  rm -rf "$temp_root"
  [ "$status" -eq 0 ]
}

@test "trace-enforced Codex provider rejects unexpected workspace entries before loading app-server" {
  temp_root="$(mktemp -d)"
  workspace="$temp_root/workspace"
  mkdir "$workspace"
  printf 'ai-plugins GPT-5.6 benchmark workspace\n' \
    >"$workspace/.ai-plugins-gpt56-workspace"
  printf 'unexpected\n' >"$workspace/stale-entry"

  run assert_provider_rejects_working_dir \
    configured "$workspace" "$temp_root/codex-home"

  rm -rf "$temp_root"
  [ "$status" -eq 0 ]
}

@test "trace-enforced Codex provider rejects a marked working_dir beneath an ancestor AGENTS.md before loading app-server" {
  temp_root="$(mktemp -d)"
  workspace="$temp_root/project/nested/workspace"
  mkdir -p "$workspace"
  printf '%s\n' '# Host instructions' >"$temp_root/project/AGENTS.md"
  printf 'ai-plugins GPT-5.6 benchmark workspace\n' \
    >"$workspace/.ai-plugins-gpt56-workspace"

  run assert_provider_rejects_working_dir \
    configured "$workspace" "$temp_root/codex-home"

  rm -rf "$temp_root"
  [ "$status" -eq 0 ]
}

@test "trace-enforced Codex provider rejects a marked working_dir inside a Git checkout before loading app-server" {
  temp_root="$(mktemp -d)"
  checkout="$temp_root/checkout"
  workspace="$checkout/nested/workspace"
  git init -q "$checkout"
  mkdir -p "$workspace"
  printf 'ai-plugins GPT-5.6 benchmark workspace\n' \
    >"$workspace/.ai-plugins-gpt56-workspace"

  run assert_provider_rejects_working_dir \
    configured "$workspace" "$temp_root/codex-home"

  rm -rf "$temp_root"
  [ "$status" -eq 0 ]
}

@test "trace-enforced Codex provider still detects an enclosing checkout behind an invalid nested .git entry" {
  temp_root="$(mktemp -d)"
  checkout="$temp_root/checkout"
  workspace="$checkout/nested/workspace"
  git init -q "$checkout"
  mkdir -p "$workspace"
  printf 'gitdir: /does/not/exist\n' >"$checkout/nested/.git"
  printf 'ai-plugins GPT-5.6 benchmark workspace\n' \
    >"$workspace/.ai-plugins-gpt56-workspace"

  run assert_provider_rejects_working_dir \
    configured "$workspace" "$temp_root/codex-home"

  rm -rf "$temp_root"
  [ "$status" -eq 0 ]
}

@test "trace-enforced Codex provider accepts and preserves a marker-owned isolated absolute working_dir" {
  temp_root="$(mktemp -d)"
  workspace="$temp_root/workspace"
  mkdir "$workspace"
  printf 'ai-plugins GPT-5.6 benchmark workspace\n' \
    >"$workspace/.ai-plugins-gpt56-workspace"

  run node --input-type=module - \
    "$ROOT" "$workspace" "$temp_root/codex-home" <<'NODE'
import { pathToFileURL } from 'node:url';

const [root, workspace, codexHome] = process.argv.slice(2);
const { default: TraceEnforcedCodexProvider } = await import(
  pathToFileURL(
    `${root}/evals/benchmarks/gpt-5.6-model-family/trace-enforced-codex-provider.mjs`,
  )
);

let loadedWorkingDir;
const provider = new TraceEnforcedCodexProvider(
  {
    config: {
      working_dir: workspace,
      cli_env: { CODEX_HOME: codexHome },
    },
  },
  async (_providerId, context) => {
    loadedWorkingDir = context?.options?.config?.working_dir;
    return {
      buildThreadStartParams: () => ({}),
      buildTurnStartParams: () => ({}),
      callApi: async () => ({
        output: 'Direct answer',
        raw: JSON.stringify({
          items: [{ type: 'agent_message', text: 'Direct answer' }],
          notifications: [
            {
              method: 'turn/started',
              params: { threadId: 'thread-1', turn: { id: 'turn-1' } },
            },
            {
              method: 'rawResponseItem/completed',
              params: {
                item: { type: 'message', role: 'assistant', content: [] },
              },
            },
            {
              method: 'turn/completed',
              params: {
                threadId: 'thread-1',
                turn: { id: 'turn-1', status: 'completed' },
              },
            },
          ],
          serverRequests: [],
        }),
      }),
    };
  },
);

const response = await provider.callApi('Answer directly.');
if (response.error) {
  throw new Error(response.error);
}
if (loadedWorkingDir !== workspace) {
  throw new Error(
    `provider did not preserve isolated working_dir: ${JSON.stringify(loadedWorkingDir)}`,
  );
}
NODE

  rm -rf "$temp_root"
  [ "$status" -eq 0 ]
}

@test "GPT-5.6 workspace check rejects an ancestor AGENTS.md without creating the workspace" {
  temp_root="$(mktemp -d)"
  workspace="$temp_root/project/nested/workspace"
  mkdir -p "$temp_root/project"
  printf '%s\n' '# Host instructions' >"$temp_root/project/AGENTS.md"

  run node \
    "$ROOT/scripts/evals/prepare-gpt56-workspace.mjs" \
    "$workspace" \
    --check

  actual_status="$status"
  actual_output="$output"
  workspace_was_created=0
  [ ! -e "$workspace" ] || workspace_was_created=1
  rm -rf "$temp_root"

  [ "$actual_status" -eq 2 ]
  [[ "$actual_output" == *"AGENTS.md"* ]]
  [ "$workspace_was_created" -eq 0 ]
}

@test "GPT-5.6 workspace preparation rejects an ancestor AGENTS.md without creating the workspace" {
  temp_root="$(mktemp -d)"
  workspace="$temp_root/project/nested/workspace"
  mkdir -p "$temp_root/project"
  printf '%s\n' '# Host instructions' >"$temp_root/project/AGENTS.md"

  run node \
    "$ROOT/scripts/evals/prepare-gpt56-workspace.mjs" \
    "$workspace"

  actual_status="$status"
  workspace_was_created=0
  [ ! -e "$workspace" ] || workspace_was_created=1
  rm -rf "$temp_root"

  [ "$actual_status" -eq 2 ]
  [ "$workspace_was_created" -eq 0 ]
}

@test "GPT-5.6 workspace check rejects a Git checkout without creating the workspace" {
  temp_root="$(mktemp -d)"
  checkout="$temp_root/checkout"
  workspace="$checkout/nested/workspace"
  git init -q "$checkout"

  run node \
    "$ROOT/scripts/evals/prepare-gpt56-workspace.mjs" \
    "$workspace" \
    --check

  actual_status="$status"
  actual_output="$output"
  workspace_was_created=0
  [ ! -e "$workspace" ] || workspace_was_created=1
  rm -rf "$temp_root"

  [ "$actual_status" -eq 2 ]
  [[ "$actual_output" == *"Git"* || "$actual_output" == *"git"* ]]
  [ "$workspace_was_created" -eq 0 ]
}

@test "GPT-5.6 workspace preparation tolerates an invalid enclosing .git entry" {
  temp_root="$(mktemp -d)"
  workspace="$temp_root/workspace"
  printf 'gitdir: /does/not/exist\n' >"$temp_root/.git"

  run node \
    "$ROOT/scripts/evals/prepare-gpt56-workspace.mjs" \
    "$workspace"

  actual_status="$status"
  marker_contents=""
  if [ -f "$workspace/.ai-plugins-gpt56-workspace" ]; then
    marker_contents="$(cat "$workspace/.ai-plugins-gpt56-workspace")"
  fi
  rm -rf "$temp_root"

  [ "$actual_status" -eq 0 ]
  [ "$marker_contents" = "ai-plugins GPT-5.6 benchmark workspace" ]
}

@test "trace-enforced Codex provider replaces caller HOME with CODEX_HOME before loading app-server" {
  temp_root="$(mktemp -d)"
  hostile_home="$temp_root/hostile-home"
  codex_home="$temp_root/codex-home"
  mkdir -p "$hostile_home/.agents/skills/hostile-skill" "$codex_home"

  run env HOME="$hostile_home" node --input-type=module - \
    "$ROOT" "$hostile_home" "$codex_home" <<'NODE'
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const [root, hostileHome, codexHome] = process.argv.slice(2);
const { default: TraceEnforcedCodexProvider } = await import(
  pathToFileURL(
    `${root}/evals/benchmarks/gpt-5.6-model-family/trace-enforced-codex-provider.mjs`,
  )
);

let loadedCliEnv;
const providerLoader = async (providerId, context) => {
  if (providerId !== 'openai:codex-app-server') {
    throw new Error(`unexpected inner provider: ${providerId}`);
  }

  loadedCliEnv = context?.options?.config?.cli_env;
  if (loadedCliEnv?.HOME !== codexHome) {
    throw new Error(
      `app-server loaded with non-isolated HOME: ${JSON.stringify(loadedCliEnv)}`,
    );
  }
  if (fs.existsSync(path.join(loadedCliEnv.HOME, '.agents', 'skills'))) {
    throw new Error('app-server can discover caller HOME skills');
  }

  return {
    buildThreadStartParams: () => ({}),
    buildTurnStartParams: () => ({}),
    callApi: async () => ({
      output: 'Direct answer',
      raw: JSON.stringify({
        items: [{ type: 'agent_message', text: 'Direct answer' }],
        notifications: [
          {
            method: 'turn/started',
            params: { threadId: 'thread-1', turn: { id: 'turn-1' } },
          },
          {
            method: 'rawResponseItem/completed',
            params: {
              item: { type: 'message', role: 'assistant', content: [] },
            },
          },
          {
            method: 'turn/completed',
            params: {
              threadId: 'thread-1',
              turn: { id: 'turn-1', status: 'completed' },
            },
          },
        ],
        serverRequests: [],
      }),
    }),
  };
};

const provider = new TraceEnforcedCodexProvider(
  {
    config: {
      model: 'gpt-5.6-terra',
      working_dir: process.env.GPT56_PROVIDER_TEST_WORKSPACE,
      cli_env: {
        CODEX_HOME: codexHome,
        HOME: hostileHome,
        PRESERVED_ENTRY: 'preserved-value',
      },
    },
  },
  providerLoader,
);

const response = await provider.callApi('Answer directly.');
if (response.error) {
  throw new Error(response.error);
}
if (loadedCliEnv.PRESERVED_ENTRY !== 'preserved-value') {
  throw new Error(`cli_env entry was not preserved: ${JSON.stringify(loadedCliEnv)}`);
}
NODE

  rm -rf "$temp_root"
  [ "$status" -eq 0 ]
}

@test "trace-enforced Codex provider rejects invalid CODEX_HOME before loading app-server" {
  run node --input-type=module - "$ROOT" <<'NODE'
import { pathToFileURL } from 'node:url';

const root = process.argv[2];
const { default: TraceEnforcedCodexProvider } = await import(
  pathToFileURL(
    `${root}/evals/benchmarks/gpt-5.6-model-family/trace-enforced-codex-provider.mjs`,
  )
);

const invalidHomes = [
  42,
  '',
  '   \t',
  '{{ env.CODEX_EVAL_HOME_NO_PLUGINS }}',
  'relative/codex-home',
];

for (const codexHome of invalidHomes) {
  let loadCount = 0;
  const provider = new TraceEnforcedCodexProvider(
    { config: { cli_env: { CODEX_HOME: codexHome } } },
    async () => {
      loadCount += 1;
      throw new Error('app-server loader must not be reached');
    },
  );

  let rejection;
  try {
    await provider.callApi('Answer directly.');
  } catch (error) {
    rejection = error;
  }

  if (!rejection || !rejection.message.includes('CODEX_HOME')) {
    throw new Error(
      `invalid CODEX_HOME was not rejected: ${JSON.stringify(codexHome)}`,
    );
  }
  if (loadCount !== 0) {
    throw new Error(
      `app-server loader ran for invalid CODEX_HOME: ${JSON.stringify(codexHome)}`,
    );
  }
}
NODE

  [ "$status" -eq 0 ]
}

@test "GPT-5.6 runner canonicalizes relative Codex home overrides from the caller directory" {
  temp_root="$(mktemp -d)"
  caller_dir="$temp_root/outside-repo-caller"
  mock_promptfoo="$temp_root/promptfoo"
  captured_env="$temp_root/captured-env"
  mkdir -p "$caller_dir"

  node - "$mock_promptfoo" <<'NODE'
const fs = require('node:fs');
fs.writeFileSync(
  process.argv[2],
  `#!/usr/bin/env bash
printf '%s\\n%s\\n' "$CODEX_EVAL_HOME_SKILLS_ONLY_MARKETPLACE" "$CODEX_EVAL_HOME_NO_PLUGINS" > "$CAPTURED_ENV"
exit 42
`,
  { mode: 0o755 },
);
NODE

  run bash -c '
    cd "$1"
    env \
      OPENAI_API_KEY=fixture \
      CODEX_EVAL_HOME_SKILLS_ONLY_MARKETPLACE=relative/skills-home \
      CODEX_EVAL_HOME_NO_PLUGINS=relative/no-plugins-home \
      GPT56_BENCHMARK_WORKSPACE="$2/workspace" \
      GPT56_BENCHMARK_OUT_ROOT="$2/out" \
      PROMPTFOO_BIN="$3" \
      CAPTURED_ENV="$4" \
      "$5" --phase execution
  ' _ "$caller_dir" "$temp_root" "$mock_promptfoo" "$captured_env" \
    "$ROOT/scripts/evals/run-gpt56-benchmark.sh"

  [ "$status" -eq 42 ]
  mapfile -t homes < "$captured_env"
  [ "${homes[0]}" = "$caller_dir/relative/skills-home" ]
  [ "${homes[1]}" = "$caller_dir/relative/no-plugins-home" ]
  [ -f "$caller_dir/relative/skills-home/.ai-plugins-eval-home" ]
  [ -f "$caller_dir/relative/no-plugins-home/.ai-plugins-eval-home" ]

  rm -rf "$temp_root"
}

@test "trace-enforced provider pins app-server to the repo-local Codex 0.144.5 CLI" {
  run node --input-type=module - "$ROOT" <<'NODE'
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const root = process.argv[2];
const expectedCodex = path.join(root, 'node_modules', '.bin', 'codex');
const lock = JSON.parse(fs.readFileSync(path.join(root, 'package-lock.json')));
const lockedCli = lock.packages?.['node_modules/@openai/codex'];
const lockedSdk = lock.packages?.['node_modules/@openai/codex-sdk'];
if (
  lockedCli?.version !== '0.144.5' ||
  lockedCli?.bin?.codex !== 'bin/codex.js' ||
  lockedSdk?.dependencies?.['@openai/codex'] !== '0.144.5'
) {
  throw new Error('package-lock.json does not pin the Codex 0.144.5 CLI');
}

const { default: TraceEnforcedCodexProvider } = await import(
  pathToFileURL(
    `${root}/evals/benchmarks/gpt-5.6-model-family/trace-enforced-codex-provider.mjs`,
  )
);

let loadedConfig;
const provider = new TraceEnforcedCodexProvider(
  {
    config: {
      codex_path_override: '/tmp/caller-selected-codex',
      working_dir: process.env.GPT56_PROVIDER_TEST_WORKSPACE,
      cli_env: {
        CODEX_HOME: '/tmp/codex-home-fixture',
        PATH: '/tmp/caller-selected-bin',
      },
    },
  },
  async (_providerId, context) => {
    loadedConfig = context?.options?.config;
    if (loadedConfig?.codex_path_override !== expectedCodex) {
      throw new Error(
        `app-server did not receive pinned Codex CLI: ${JSON.stringify(loadedConfig)}`,
      );
    }
    return {
      buildThreadStartParams: () => ({}),
      buildTurnStartParams: () => ({}),
      callApi: async () => ({
        output: 'Direct answer',
        raw: JSON.stringify({
          items: [{ type: 'agent_message', text: 'Direct answer' }],
          notifications: [
            {
              method: 'turn/started',
              params: { threadId: 'thread-1', turn: { id: 'turn-1' } },
            },
            {
              method: 'rawResponseItem/completed',
              params: {
                item: { type: 'message', role: 'assistant', content: [] },
              },
            },
            {
              method: 'turn/completed',
              params: {
                threadId: 'thread-1',
                turn: { id: 'turn-1', status: 'completed' },
              },
            },
          ],
          serverRequests: [],
        }),
      }),
    };
  },
);

const response = await provider.callApi('Answer directly.');
if (response.error) {
  throw new Error(response.error);
}
if (loadedConfig.cli_env.PATH !== '/tmp/caller-selected-bin') {
  throw new Error(`unrelated cli_env entry was not preserved: ${JSON.stringify(loadedConfig)}`);
}
NODE

  [ "$status" -eq 0 ]
}
