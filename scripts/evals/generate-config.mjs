#!/usr/bin/env node
import fs from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";
import process from "node:process";

const root = path.resolve(import.meta.dirname, "../..");
const require = createRequire(import.meta.url);
const evalWorkspace = path.join(root, ".evals/agent-workspace");
const developmentSystemPluginName = "development-system";
const advisoryPromptPrefix =
  "Answer the scenario as an advisory behavior question. Treat each scenario as stateless: do not use, mention, or rely on prior conversations, user memory, session memory, or earlier eval runs. Use installed marketplace plugin and skill guidance when it is relevant, naming the relevant plugin or skill in the answer. You may read installed skill instruction files through the harness. When plugin or skill guidance documents a command, include the exact command name and flags instead of generic setup-path wording. Apply plugin-specific safety gates and documented commands exactly instead of replacing them with generic setup or validation advice. Do not inspect target repository state, mutate files, start evals, or run unrelated shell commands.";

function usage() {
  console.log(`Usage: node scripts/evals/generate-config.mjs [--suite behavior|canary] [--output path] [--metadata-output path] [--stdout]

Generates promptfoo configs from the current Claude and Codex marketplace manifests.
`);
}

function parseArgs(argv) {
  const args = {
    suite: "behavior",
    stdout: false,
    output: null,
    metadataOutput: null,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];

    if (arg === "--help") {
      args.help = true;
    } else if (arg === "--stdout") {
      args.stdout = true;
    } else if (arg === "--suite") {
      args.suite = argv[++index];
    } else if (arg === "--output") {
      args.output = argv[++index];
    } else if (arg === "--metadata-output") {
      args.metadataOutput = argv[++index];
    } else {
      throw new Error(`unknown argument: ${arg}`);
    }
  }

  if (!["behavior", "canary"].includes(args.suite)) {
    throw new Error(`unknown suite: ${args.suite}`);
  }

  return args;
}

function readPlugins(file) {
  const manifest = JSON.parse(fs.readFileSync(path.join(root, file), "utf8"));
  return manifest.plugins.map((plugin) => ({
    name: plugin.name,
    version: plugin.version,
    path:
      plugin.source && typeof plugin.source === "object"
        ? plugin.source.path
        : plugin.source,
  }));
}

function readJson(file) {
  return JSON.parse(fs.readFileSync(path.join(root, file), "utf8"));
}

function normalizePlugin(plugin) {
  const pluginPath = plugin.path?.startsWith("./")
    ? plugin.path
    : `./${plugin.path || `plugins/${plugin.name}`}`;
  return {
    name: plugin.name,
    version: plugin.version,
    path: pluginPath,
    absolutePath: path.resolve(root, pluginPath),
  };
}

function evalMatrix() {
  return readJson("evals/matrix.json");
}

function manifestPlugins(file) {
  return readPlugins(file)
    .map(normalizePlugin)
    .sort((left, right) =>
      left.name < right.name ? -1 : left.name > right.name ? 1 : 0,
    );
}

function piSupportedPlugins() {
  const inventory = readJson(".agents/plugins/pi-support.json");
  return inventory.packages.map((entry) => {
    const plugin = normalizePlugin({ name: entry.name, path: entry.path });
    const manifest = readJson(path.join(entry.path, "package.json"));
    return { ...plugin, version: manifest.version };
  });
}

function allMarketplacePlugins() {
  const byName = new Map();

  for (const plugin of [
    ...manifestPlugins(".claude-plugin/marketplace.json"),
    ...manifestPlugins(".agents/plugins/marketplace.json"),
  ]) {
    byName.set(plugin.name, plugin);
  }

  return [...byName.values()].sort((left, right) =>
    left.name < right.name ? -1 : left.name > right.name ? 1 : 0,
  );
}

function quote(value) {
  return JSON.stringify(String(value));
}

function fileUrl(file) {
  return `file://${path.resolve(file)}`;
}

function behaviorTestLoader() {
  if (process.env.EVAL_CASE_FILTER || process.env.EVAL_SAMPLES) {
    return fileUrl(
      process.env.EVAL_RUNTIME_LOADER_FILE ||
        path.join(root, "evals/out/generated/load-harness-cases.runtime.cjs"),
    );
  }
  return fileUrl(path.join(root, "evals/promptfoo/load-harness-cases.cjs"));
}

function indentedList(items, indent, render) {
  return items.map((item) => `${" ".repeat(indent)}${render(item)}`).join("\n");
}

function providerEnv(value, fallback) {
  return `"{{ env.${value} | default('${fallback}') }}"`;
}

function claudeProvider(variant, pluginMode, plugins) {
  const evalHome = path.join(root, `.evals/claude-home-${pluginMode.id}`);
  const envSuffix = pluginMode.id.replaceAll("-", "_").toUpperCase();
  const plugin = plugins[0];
  const pluginPath = plugin
    ? path.join(
        evalHome,
        "plugin-cache/cache/ai-plugins",
        plugin.name,
        plugin.version,
      )
    : null;
  const pluginLines =
    pluginMode.id === "no-plugins"
      ? ""
      : `      plugins:
        - type: local
          path: "{{ env.CLAUDE_EVAL_PLUGIN_PATH_${envSuffix} | default('${pluginPath}') }}"
      include_hook_events: true
`;

  return `  - id: ${variant.provider}
    label: ${variant.id}-${pluginMode.id}
    pluginMode: ${pluginMode.id}
    providerVariant: ${variant.id}
    config:
      apiKeyRequired: false
      model: ${providerEnv(variant.modelEnv, variant.defaultModel)}
      working_dir: ${quote(evalWorkspace)}
      permission_mode: dontAsk
      skills: all
      setting_sources: []
      persist_session: false
      env:
        CLAUDE_CONFIG_DIR: "{{ env.CLAUDE_EVAL_RUNTIME_CONFIG_DIR_${envSuffix} | default('${path.join(evalHome, "config")}') }}"
        CLAUDE_CODE_PLUGIN_CACHE_DIR: "{{ env.CLAUDE_EVAL_PLUGIN_CACHE_DIR_${envSuffix} | default('${path.join(evalHome, "plugin-cache")}') }}"
        CLAUDE_CODE_DISABLE_AUTO_MEMORY: "1"
${pluginMode.id === "development-system" ? `        DEVELOPMENT_SYSTEM_EVAL_SESSION_START_MARKER: "{{ env.CLAUDE_EVAL_SESSION_START_MARKER_CLAUDE | default('${path.join(root, ".evals/session-start-claude-development-system")}') }}"\n` : ""}      disallowed_tools:
        - Bash
        - Write
        - Edit
        - MultiEdit
${pluginLines}`.trimEnd();
}

function codexProvider(variant, pluginMode) {
  const homeSuffix = pluginMode.id;
  return `  - id: ${variant.provider}
    label: ${variant.id}-${pluginMode.id}
    pluginMode: ${pluginMode.id}
    providerVariant: ${variant.id}
    config:
      model: ${providerEnv(variant.modelEnv, variant.defaultModel)}
      model_reasoning_effort: ${providerEnv(variant.reasoningEffortEnv, variant.defaultReasoningEffort)}
      working_dir: ${quote(evalWorkspace)}
      sandbox_mode: read-only
      approval_policy: never
      enable_streaming: true
      deep_tracing: false
      skip_git_repo_check: true
${pluginMode.id === "development-system" ? `      codex_path_override: "${path.join(root, "scripts/evals/codex-with-trusted-hooks.sh")}"\n` : ""}      cli_env:
        CODEX_HOME: "{{ env.CODEX_EVAL_HOME_${pluginMode.id.replaceAll("-", "_").toUpperCase()} | default('${path.join(root, `.evals/codex-home-${homeSuffix}`)}') }}"${
          pluginMode.id === "development-system"
            ? `
        CODEX_EVAL_REAL_BIN: "{{ env.CODEX_EVAL_REAL_BIN | default('${path.join(root, "node_modules/.bin/codex")}') }}"
        DEVELOPMENT_SYSTEM_EVAL_SESSION_START_MARKER: "{{ env.CODEX_EVAL_SESSION_START_MARKER | default('${path.join(root, ".evals/session-start-codex-development-system")}') }}"`
            : ""
        }`;
}

function piProvider(variant, pluginMode) {
  const suffix = pluginMode.id.replaceAll("-", "_").toUpperCase();
  return `  - id: "file://${path.join(root, "scripts/evals/pi-provider.mjs")}"
    label: ${variant.id}-${pluginMode.id}
    pluginMode: ${pluginMode.id}
    providerVariant: ${variant.id}
    config:
      provider: openai-codex
      model: ${providerEnv(variant.modelEnv, variant.defaultModel)}
      thinking: ${providerEnv(variant.reasoningEffortEnv, variant.defaultReasoningEffort)}
      working_dir: ${quote(evalWorkspace)}
      package_mode: ${pluginMode.id}
      agent_dir: "{{ env.PI_EVAL_HOME_${suffix} | default('${path.join(root, `.evals/pi-home-${pluginMode.id}`)}') }}"`;
}

function providerFor(variant, pluginMode, plugins) {
  if (variant.provider === "file://scripts/evals/pi-provider.mjs") {
    return piProvider(variant, pluginMode);
  }
  if (variant.provider === "anthropic:claude-agent-sdk") {
    return claudeProvider(variant, pluginMode, plugins);
  }
  if (variant.provider === "openai:codex-sdk") {
    return codexProvider(variant, pluginMode);
  }
  throw new Error(`unsupported provider variant: ${variant.id}`);
}

function providerEntry(variant, pluginMode, plugins) {
  return {
    label: `${variant.id}-${pluginMode.id}`,
    variant,
    pluginMode,
    plugins,
    config: providerFor(variant, pluginMode, plugins),
  };
}

function pluginsForProvider(variant, pluginMode, pluginSets) {
  if (pluginMode.id === "no-plugins") {
    return [];
  }

  const harness =
    variant.provider === "file://scripts/evals/pi-provider.mjs"
      ? pluginSets.pi
      : variant.provider === "anthropic:claude-agent-sdk"
        ? pluginSets.claude
        : variant.provider === "openai:codex-sdk"
          ? pluginSets.codex
          : null;
  if (!harness) {
    throw new Error(`unsupported provider variant: ${variant.id}`);
  }

  if (pluginMode.id === "development-system") {
    return harness.filter(
      (plugin) => plugin.name === developmentSystemPluginName,
    );
  }
  if (
    pluginMode.id === "full-marketplace" &&
    variant.provider === "file://scripts/evals/pi-provider.mjs"
  ) {
    return harness;
  }
  throw new Error(`unsupported plugin mode: ${pluginMode.id}`);
}

function providerMatches(entry, term) {
  if (term === entry.label) {
    return true;
  }
  if (term === entry.variant.id) {
    return entry.pluginMode.id === "development-system";
  }
  if (term === entry.variant.provider || term === entry.pluginMode.id) {
    return true;
  }
  return entry.label.includes(term) || entry.variant.provider.includes(term);
}

function filteredProviderEntries(entries) {
  const filter = process.env.EVAL_PROVIDER_FILTER;
  if (!filter) {
    return entries;
  }

  const terms = filter
    .split(",")
    .map((term) => term.trim())
    .filter(Boolean);
  const filtered = entries.filter((entry) =>
    terms.some((term) => providerMatches(entry, term)),
  );

  if (filtered.length === 0) {
    throw new Error(`no providers match EVAL_PROVIDER_FILTER=${filter}`);
  }

  return filtered;
}

function uniqueById(items) {
  const byId = new Map();
  for (const item of items) {
    byId.set(item.id, item);
  }
  return [...byId.values()];
}

function configFor(suite) {
  const allPlugins = allMarketplacePlugins();
  const claudePlugins = manifestPlugins(".claude-plugin/marketplace.json");
  const codexPlugins = manifestPlugins(".agents/plugins/marketplace.json");
  const matrix = evalMatrix();
  const developmentSystemPlugin = (plugins, harnessName) => {
    const selected = plugins.filter(
      (plugin) => plugin.name === developmentSystemPluginName,
    );
    if (selected.length !== 1) {
      throw new Error(
        `${harnessName} marketplace must contain exactly one ${developmentSystemPluginName} plugin`,
      );
    }
    return selected;
  };
  const pluginSets = {
    pi: matrix.providerVariants.some(
      (variant) => variant.provider === "file://scripts/evals/pi-provider.mjs",
    )
      ? developmentSystemPlugin(piSupportedPlugins(), "Pi")
      : [],
    claude: developmentSystemPlugin(claudePlugins, "Claude Code"),
    codex: developmentSystemPlugin(codexPlugins, "Codex"),
  };
  const testLoader =
    suite === "canary"
      ? fileUrl(path.join(root, "evals/promptfoo/load-canary-cases.cjs"))
      : behaviorTestLoader();
  const description =
    suite === "canary"
      ? "Installed development-system canary for ai-plugins coding harnesses"
      : "Provider-backed behavior evals for the ai-plugins marketplace";
  const providerEntries =
    suite === "behavior"
      ? matrix.providerVariants.flatMap((variant) =>
          matrix.pluginModes
            .filter((pluginMode) =>
              (
                variant.pluginModes ?? matrix.pluginModes.map((mode) => mode.id)
              ).includes(pluginMode.id),
            )
            .map((pluginMode) =>
              providerEntry(
                variant,
                pluginMode,
                pluginsForProvider(variant, pluginMode, pluginSets),
              ),
            ),
        )
      : matrix.providerVariants.map((variant) =>
          providerEntry(
            variant,
            { id: "development-system" },
            pluginsForProvider(
              variant,
              { id: "development-system" },
              pluginSets,
            ),
          ),
        );
  const providers = filteredProviderEntries(providerEntries);
  const providerVariants = uniqueById(providers.map((entry) => entry.variant));
  const pluginModes = uniqueById(providers.map((entry) => entry.pluginMode));
  const providerLabels = providers.map((entry) => entry.label);
  const providerCompositions = providers.map((entry) => ({
    label: entry.label,
    provider: entry.variant.provider,
    providerVariant: entry.variant.id,
    pluginMode: entry.pluginMode.id,
    plugins: entry.plugins.map((plugin) => plugin.name),
  }));
  const metadata = {
    suite,
    usesCodexGrader: true,
    providerLabels,
    providerCompositions,
  };

  const yaml = `# yaml-language-server: $schema=https://promptfoo.dev/config-schema.json
description: ${description}

prompts:
  - |
    ${advisoryPromptPrefix}

    {{scenario_prompt}}

providers:
${providers.map((entry) => entry.config).join("\n")}

tests: ${testLoader}

defaultTest:
  options:
    provider:
      text:
        id: openai:codex-sdk
        config:
          model: "{{ env.CODEX_GRADER_MODEL | default('gpt-5.6-sol') }}"
          model_reasoning_effort: "{{ env.CODEX_GRADER_REASONING_EFFORT | default('high') }}"
          working_dir: ${quote(evalWorkspace)}
          sandbox_mode: read-only
          approval_policy: never
          enable_streaming: true
          deep_tracing: false
          skip_git_repo_check: true
          cli_env:
            CODEX_HOME: "{{ env.CODEX_EVAL_HOME_DEVELOPMENT_SYSTEM | default(env.CODEX_EVAL_HOME) | default('${path.join(root, ".evals/codex-home-development-system")}') }}"

tracing:
  enabled: false

metadata:
  suite: ${suite}
  testLoaderByPluginMode: ${suite === "behavior" ? `${testLoader}?pluginMode={{ provider.pluginMode }}` : testLoader}
  providerLabels: ${JSON.stringify(providerLabels)}
  providerCompositions: ${JSON.stringify(providerCompositions)}
  matrix:
    pluginModes:
${indentedList(pluginModes, 6, (mode) => `- id: ${mode.id}`)}
    providerVariants:
${indentedList(providerVariants, 6, (variant) => `- id: ${variant.id}\n${" ".repeat(8)}provider: ${variant.provider}`)}
  marketplacePlugins:
${indentedList(allPlugins, 4, (plugin) => `- name: ${plugin.name}\n${" ".repeat(6)}sourcePath: ${quote(plugin.path)}`)}

commandLineOptions:
  maxConcurrency: 1
  share: false
  cache: false
  write: true
`;

  return { yaml, metadata };
}

try {
  const args = parseArgs(process.argv.slice(2));

  if (args.help) {
    usage();
    process.exit(0);
  }

  const { yaml, metadata } = configFor(args.suite);

  if (args.stdout || !args.output) {
    process.stdout.write(yaml);
  }

  if (args.output) {
    fs.mkdirSync(path.dirname(args.output), { recursive: true });
    fs.writeFileSync(args.output, yaml);
  }

  if (args.metadataOutput) {
    fs.mkdirSync(path.dirname(args.metadataOutput), { recursive: true });
    fs.writeFileSync(
      args.metadataOutput,
      `${JSON.stringify(metadata, null, 2)}\n`,
    );
  }
} catch (error) {
  console.error(error.message);
  usage();
  process.exit(2);
}
