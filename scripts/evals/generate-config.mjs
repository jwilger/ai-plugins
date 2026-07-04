#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const root = path.resolve(import.meta.dirname, "../..");

function usage() {
  console.log(`Usage: node scripts/evals/generate-config.mjs [--suite behavior|canary] [--output path] [--stdout]

Generates promptfoo configs from the current Claude and Codex marketplace manifests.
`);
}

function parseArgs(argv) {
  const args = { suite: "behavior", stdout: false, output: null };

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

function evalMatrix() {
  return readJson("evals/matrix.json");
}

function marketplacePlugins() {
  const byName = new Map();

  for (const plugin of [
    ...readPlugins(".claude-plugin/marketplace.json"),
    ...readPlugins(".agents/plugins/marketplace.json"),
  ]) {
    const pluginPath = plugin.path?.startsWith("./")
      ? plugin.path
      : `./${plugin.path || `plugins/${plugin.name}`}`;
    byName.set(plugin.name, {
      name: plugin.name,
      path: pluginPath,
      absolutePath: path.resolve(root, pluginPath),
    });
  }

  return [...byName.values()].sort((left, right) =>
    left.name.localeCompare(right.name),
  );
}

function quote(value) {
  return JSON.stringify(String(value));
}

function fileUrl(file) {
  return `file://${path.resolve(file)}`;
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
      working_dir: ${quote(root)}
      permission_mode: dontAsk
      skills: all
      disallowed_tools:
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
      working_dir: ${quote(root)}
      sandbox_mode: read-only
      approval_policy: never
      enable_streaming: true
      deep_tracing: true
      skip_git_repo_check: false
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

function configFor(suite) {
  const plugins = marketplacePlugins();
  const matrix = evalMatrix();
  const testLoader = fileUrl(
    path.join(
      root,
      "evals/promptfoo",
      suite === "canary" ? "load-canary-cases.cjs" : "load-harness-cases.cjs",
    ),
  );
  const description =
    suite === "canary"
      ? "Full-marketplace canary for ai-plugins coding harnesses"
      : "Provider-backed behavior evals for the ai-plugins marketplace";
  const providers =
    suite === "behavior"
      ? matrix.providerVariants.flatMap((variant) =>
          matrix.pluginModes.map((pluginMode) =>
            providerFor(variant, pluginMode, plugins),
          ),
        )
      : [
          providerFor(
            matrix.providerVariants.find(
              (variant) => variant.provider === "anthropic:claude-agent-sdk",
            ),
            { id: "full-marketplace" },
            plugins,
          ),
          providerFor(
            matrix.providerVariants.find(
              (variant) => variant.provider === "openai:codex-sdk",
            ),
            { id: "full-marketplace" },
            plugins,
          ),
        ];

  return `# yaml-language-server: $schema=https://promptfoo.dev/config-schema.json
description: ${description}

prompts:
  - |
    {{scenario_prompt}}

providers:
${providers.join("\n")}

tests: ${testLoader}

defaultTest:
  options:
    provider:
      id: openai:codex-sdk
      config:
        model: "{{ env.CODEX_GRADER_MODEL | default('gpt-5.5') }}"
        model_reasoning_effort: "{{ env.CODEX_GRADER_REASONING_EFFORT | default('medium') }}"
        working_dir: ${quote(root)}
        sandbox_mode: read-only
        approval_policy: never
        enable_streaming: true
        deep_tracing: true
        skip_git_repo_check: false
        cli_env:
          CODEX_HOME: "{{ env.CODEX_EVAL_HOME | default('${path.join(root, ".dependencies/evals/codex-home-full-marketplace")}') }}"

tracing:
  enabled: true

metadata:
  suite: ${suite}
  testLoaderByPluginMode: ${suite === "behavior" ? `${testLoader}?pluginMode={{ provider.pluginMode }}` : testLoader}
  matrix:
    pluginModes:
${indentedList(matrix.pluginModes, 6, (mode) => `- id: ${mode.id}`)}
    providerVariants:
${indentedList(matrix.providerVariants, 6, (variant) => `- id: ${variant.id}\n${" ".repeat(8)}provider: ${variant.provider}`)}
  fullMarketplacePlugins:
${indentedList(plugins, 4, (plugin) => `- name: ${plugin.name}\n${" ".repeat(6)}sourcePath: ${quote(plugin.path)}`)}

commandLineOptions:
  maxConcurrency: 2
  share: false
  cache: false
  write: true
`;
}

try {
  const args = parseArgs(process.argv.slice(2));

  if (args.help) {
    usage();
    process.exit(0);
  }

  const yaml = configFor(args.suite);

  if (args.stdout || !args.output) {
    process.stdout.write(yaml);
  }

  if (args.output) {
    fs.mkdirSync(path.dirname(args.output), { recursive: true });
    fs.writeFileSync(args.output, yaml);
  }
} catch (error) {
  console.error(error.message);
  usage();
  process.exit(2);
}
