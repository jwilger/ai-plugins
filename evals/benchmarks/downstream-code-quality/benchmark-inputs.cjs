const crypto = require("node:crypto");
const fs = require("node:fs");
const path = require("node:path");
const YAML = require("yaml");
const { providerLabelFor } = require("./manifest.cjs");

const promptfooConfig = path.join(__dirname, "promptfooconfig.yaml");
const benchmarkModes = Object.freeze([
  "no-marketplace-skills",
  "targeted-quality-skills",
  "all-marketplace-skills",
]);
const promptTemplate =
  "Complete this coding task in the provided repository. The task is self-contained. Use any installed skill guidance when relevant.\n\n{{ scenario_prompt }}\n";
const rustFeaturePrompt = [
  "Implement the requested feature directly in this repository; do not merely describe a solution.",
  "Preserve the existing `validate` command.",
  "Add `totals [--minimum-cents POSITIVE_U64]` for stdin records formatted as `category,cents`.",
  "Aggregate duplicate categories with checked u64 addition, sort output lexically by category, and make the optional minimum inclusive.",
  "Reject invalid arguments, malformed records, invalid minimums, amount overflow, and aggregate overflow with a nonzero exit and no stdout.",
  "Add behavior-focused regression tests and leave a current debug binary at `target/debug/expense-report`.",
  "Run formatting, clippy with warnings denied, locked tests, and a locked build before finishing.",
  "Work only inside this repository. Do not inspect parent or sibling paths, use the network, add Git remotes, or push.",
].join("\n");

function canonicalize(value) {
  if (Array.isArray(value)) return value.map(canonicalize);
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.keys(value)
        .sort()
        .map((key) => [key, canonicalize(value[key])]),
    );
  }
  return value;
}

function hashCanonical(value) {
  return crypto
    .createHash("sha256")
    .update(JSON.stringify(canonicalize(value)))
    .digest("hex");
}

function canonicalEquals(actual, expected) {
  return (
    JSON.stringify(canonicalize(actual)) ===
    JSON.stringify(canonicalize(expected))
  );
}

function expectedPromptfooTopLevel(providers) {
  return {
    commandLineOptions: {
      cache: false,
      maxConcurrency: 1,
      share: false,
      write: false,
    },
    description: "Writable downstream Codex code-quality benchmark",
    metadata: {
      benchmark: "downstream-code-quality",
      providerLabels: benchmarkModes.map(providerLabelFor),
    },
    prompts: [promptTemplate],
    providers,
    tests: "file://cases.cjs",
    tracing: { enabled: false },
  };
}

function environmentTemplate(name) {
  return `{{ env.${name} }}`;
}

function expectedProvider(mode) {
  const cliEnvironmentNames = [
    "CODE_QUALITY_BWRAP_BIN",
    "CODE_QUALITY_BWRAP_EXPECTED_SHA256",
    "CODE_QUALITY_NIX_STORE_CLOSURE",
    "CODE_QUALITY_NIX_STORE_CLOSURE_EXPECTED_SHA256",
    "CODE_QUALITY_CODEX_EXPECTED_SHA256",
    "CODE_QUALITY_CODEX_EXPECTED_VERSION",
    "CODE_QUALITY_CODEX_REAL_BIN",
    "CODE_QUALITY_CODEX_RESOURCE_BWRAP_EXPECTED_SHA256",
    "CODE_QUALITY_CODEX_RG_EXPECTED_SHA256",
    "CODE_QUALITY_NODE_BIN",
    "CODE_QUALITY_OUTPUT_MAX_BYTES",
    "CODE_QUALITY_PRLIMIT_BIN",
    "CODE_QUALITY_PRLIMIT_EXPECTED_SHA256",
    "CODE_QUALITY_SYSTEMD_RUN_BIN",
    "CODE_QUALITY_SYSTEMD_RUN_EXPECTED_SHA256",
    "CODE_QUALITY_TIMEOUT_BIN",
    "CODE_QUALITY_TIMEOUT_EXPECTED_SHA256",
    "CODE_QUALITY_TOOL_PATH",
    "CODE_QUALITY_WALL_TIMEOUT_SECONDS",
    "CODE_QUALITY_WORKSPACE_MAX_BYTES",
    "CODE_QUALITY_WORKSPACE_MAX_ENTRIES",
  ];
  const cliEnvironment = Object.fromEntries(
    cliEnvironmentNames.map((name) => [name, environmentTemplate(name)]),
  );
  return {
    config: {
      approval_policy: "never",
      cli_config: {
        features: {
          apps: false,
          auth_elicitation: false,
          browser_use: false,
          browser_use_external: false,
          browser_use_full_cdp_access: false,
          code_mode_host: false,
          computer_use: false,
          enable_fanout: false,
          enable_request_compression: true,
          fast_mode: false,
          goals: false,
          guardian_approval: true,
          hooks: false,
          image_generation: false,
          in_app_browser: false,
          mentions_v2: false,
          multi_agent: false,
          personality: false,
          plugin_sharing: false,
          plugins: mode !== "no-marketplace-skills",
          remote_compaction_v2: false,
          remote_plugin: false,
          secret_auth_storage: false,
          shell_snapshot: false,
          shell_tool: true,
          skill_mcp_dependency_install: false,
          tool_call_mcp_elicitation: false,
          tool_suggest: false,
          unified_exec: true,
          workspace_dependencies: false,
        },
        history: { persistence: "none" },
        sandbox_workspace_write: {
          exclude_slash_tmp: true,
          exclude_tmpdir_env_var: true,
          network_access: false,
          writable_roots: [],
        },
        shell_environment_policy: {
          experimental_use_profile: false,
          ignore_default_excludes: false,
          inherit: "none",
          set: {
            CARGO_HOME: "{{ workspace }}/.cargo-home",
            CARGO_TARGET_DIR: "{{ workspace }}/target",
            GIT_CONFIG_GLOBAL: "/dev/null",
            GIT_CONFIG_NOSYSTEM: "1",
            HOME: "{{ workspace }}/.home",
            LANG: "C.UTF-8",
            LC_ALL: "C.UTF-8",
            PATH: environmentTemplate("CODE_QUALITY_TOOL_PATH"),
            TMPDIR: "{{ workspace }}/.tmp",
          },
        },
        web_search: "disabled",
      },
      cli_env: {
        CODEX_HOME: "{{ codex_home }}",
        HOME: "{{ codex_home }}",
        TMPDIR: "{{ codex_tmp }}",
        ...cliEnvironment,
      },
      codex_path_override: environmentTemplate("CODE_QUALITY_CODEX_LAUNCHER"),
      deep_tracing: false,
      enable_streaming: false,
      inherit_process_env: false,
      model: "{{ env.CODE_QUALITY_CODEX_MODEL | default('gpt-5.6-terra') }}",
      model_reasoning_effort:
        "{{ env.CODE_QUALITY_CODEX_REASONING_EFFORT | default('medium') }}",
      network_access_enabled: false,
      persist_threads: false,
      sandbox_mode: "workspace-write",
      skip_git_repo_check: false,
      web_search_enabled: false,
      web_search_mode: "disabled",
      working_dir: "{{ workspace }}",
    },
    id: "openai:codex-sdk",
    label: providerLabelFor(mode),
  };
}

function promptFor(testCase) {
  if (testCase?.caseId === "rust-cli-feature") return rustFeaturePrompt;
  throw new Error(`benchmark prompt is not implemented: ${testCase?.caseId}`);
}

function renderPrompt(scenarioPrompt) {
  if (typeof scenarioPrompt !== "string" || scenarioPrompt.length === 0) {
    throw new Error("scenario prompt must be a nonempty string");
  }
  const placeholder = "{{ scenario_prompt }}";
  if (promptTemplate.split(placeholder).length !== 2) {
    throw new Error("benchmark prompt template must contain one placeholder");
  }
  return promptTemplate.replace(placeholder, scenarioPrompt);
}

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function modeFromProviderLabel(label) {
  const prefix = "openai-codex-sdk-";
  if (typeof label !== "string" || !label.startsWith(prefix)) {
    throw new Error("Promptfoo provider label is invalid");
  }
  return label.slice(prefix.length);
}

function loadPromptfooSurface(configFile = promptfooConfig) {
  const bytes = fs.readFileSync(configFile);
  const parsed = YAML.parse(bytes.toString("utf8"));
  if (
    !isPlainObject(parsed) ||
    !Array.isArray(parsed.prompts) ||
    parsed.prompts.length !== 1 ||
    parsed.prompts[0] !== promptTemplate ||
    !Array.isArray(parsed.providers) ||
    parsed.providers.length !== 3
  ) {
    throw new Error("Promptfoo benchmark surface does not match shared inputs");
  }
  if (!canonicalEquals(parsed, expectedPromptfooTopLevel(parsed.providers))) {
    throw new Error("Promptfoo benchmark top-level contract is invalid");
  }

  const providers = {};
  for (const [index, provider] of parsed.providers.entries()) {
    if (
      !isPlainObject(provider) ||
      provider.id !== "openai:codex-sdk" ||
      !isPlainObject(provider.config)
    ) {
      throw new Error("Promptfoo benchmark provider is invalid");
    }
    const mode = modeFromProviderLabel(provider.label);
    if (
      mode !== benchmarkModes[index] ||
      provider.label !== providerLabelFor(mode) ||
      providers[mode]
    ) {
      throw new Error("Promptfoo benchmark provider binding is invalid");
    }
    if (!canonicalEquals(provider, expectedProvider(mode))) {
      throw new Error("Promptfoo benchmark provider contract is invalid");
    }
    providers[mode] = canonicalize(provider);
  }
  for (const mode of benchmarkModes) {
    if (!providers[mode]) {
      throw new Error("Promptfoo benchmark provider matrix is incomplete");
    }
  }

  return {
    configSha256: crypto.createHash("sha256").update(bytes).digest("hex"),
    promptTemplate: parsed.prompts[0],
    providers,
  };
}

function inputBindingFor(row, benchmarkId = "downstream-code-quality") {
  return {
    baselineOid: row.baselineOid,
    benchmarkId,
    caseId: row.caseId,
    fixtureDigest: row.fixtureDigest,
    renderedPrompt: renderPrompt(promptFor(row)),
    schemaVersion: 2,
    taskType: row.taskType,
  };
}

function inputHashFor(row, benchmarkId) {
  return hashCanonical(inputBindingFor(row, benchmarkId));
}

module.exports = {
  canonicalize,
  hashCanonical,
  inputBindingFor,
  inputHashFor,
  loadPromptfooSurface,
  promptFor,
  promptTemplate,
  renderPrompt,
};
