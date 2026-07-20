#!/usr/bin/env node
import fs from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";
import process from "node:process";

const root = path.resolve(import.meta.dirname, "../..");
const require = createRequire(import.meta.url);
const { selectedBehaviorPluginNames } = require(
  path.join(root, "evals/promptfoo/fixtures.cjs"),
);
const evalWorkspace = path.join(root, ".dependencies/evals/agent-workspace");
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
  const pluginLines =
    pluginMode.id === "no-plugins"
      ? ""
      : `      plugins:
${indentedList(plugins, 8, (plugin) => `- type: local\n${" ".repeat(10)}path: ${quote(plugin.absolutePath)}`)}
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
      disallowed_tools:
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
      cli_env:
        CODEX_HOME: "{{ env.CODEX_EVAL_HOME_${pluginMode.id.replaceAll("-", "_").toUpperCase()} | default('${path.join(root, `.dependencies/evals/codex-home-${homeSuffix}`)}') }}"`;
}

function providerFor(variant, pluginMode, plugins) {
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

function namedPlugins(pluginNames, marketplacePlugins, harnessName) {
  const byName = new Map(
    marketplacePlugins.map((plugin) => [plugin.name, plugin]),
  );
  const missing = pluginNames.filter((pluginName) => !byName.has(pluginName));

  if (missing.length > 0) {
    throw new Error(
      `selected behavior plugin(s) unavailable to ${harnessName}: ${missing.join(", ")}`,
    );
  }

  return pluginNames.map((pluginName) => byName.get(pluginName));
}

function pluginsForProvider(variant, pluginMode, pluginSets) {
  if (pluginMode.id === "no-plugins") {
    return [];
  }

  const harness =
    variant.provider === "anthropic:claude-agent-sdk"
      ? pluginSets.claude
      : variant.provider === "openai:codex-sdk"
        ? pluginSets.codex
        : null;
  if (!harness) {
    throw new Error(`unsupported provider variant: ${variant.id}`);
  }

  if (pluginMode.id === "targeted-plugins") {
    return harness.targeted;
  }
  if (pluginMode.id === "full-marketplace") {
    return harness.full;
  }
  throw new Error(`unsupported plugin mode: ${pluginMode.id}`);
}

function providerMatches(entry, term) {
  if (term === entry.label) {
    return true;
  }
  if (term === entry.variant.id) {
    return entry.pluginMode.id === "full-marketplace";
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
  const usesTargetedMode =
    suite === "behavior" &&
    matrix.pluginModes.some(
      (pluginMode) => pluginMode.id === "targeted-plugins",
    );
  const targetedPluginNames = usesTargetedMode
    ? selectedBehaviorPluginNames({
        root,
        caseFilter: process.env.EVAL_CASE_FILTER,
      })
    : [];
  const pluginSets = {
    claude: {
      full: claudePlugins,
      targeted: namedPlugins(targetedPluginNames, claudePlugins, "Claude Code"),
    },
    codex: {
      full: codexPlugins,
      targeted: namedPlugins(targetedPluginNames, codexPlugins, "Codex"),
    },
  };
  const testLoader =
    suite === "canary"
      ? fileUrl(path.join(root, "evals/promptfoo/load-canary-cases.cjs"))
      : behaviorTestLoader();
  const description =
    suite === "canary"
      ? "Full-marketplace canary for ai-plugins coding harnesses"
      : "Provider-backed behavior evals for the ai-plugins marketplace";
  const providerEntries =
    suite === "behavior"
      ? matrix.providerVariants.flatMap((variant) =>
          matrix.pluginModes.map((pluginMode) =>
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
            { id: "full-marketplace" },
            pluginsForProvider(variant, { id: "full-marketplace" }, pluginSets),
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
            CODEX_HOME: "{{ env.CODEX_EVAL_HOME_FULL_MARKETPLACE | default(env.CODEX_EVAL_HOME) | default('${path.join(root, ".dependencies/evals/codex-home-full-marketplace")}') }}"

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
  fullMarketplacePlugins:
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
