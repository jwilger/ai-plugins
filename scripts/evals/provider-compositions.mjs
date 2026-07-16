#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

const supportedProviders = new Set([
  "anthropic:claude-agent-sdk",
  "openai:codex-sdk",
]);
const supportedPluginModes = new Set([
  "no-plugins",
  "targeted-plugins",
  "full-marketplace",
]);
const pluginNamePattern = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;

function sameStringLists(left, right) {
  return JSON.stringify(left) === JSON.stringify(right);
}

function parseExpectedProviderLabels(value) {
  if (!Array.isArray(value) || value.length === 0) {
    throw new Error(
      "generated eval metadata is missing non-empty providerLabels",
    );
  }
  if (value.some((label) => typeof label !== "string" || label.length === 0)) {
    throw new Error(
      "generated eval metadata contains an invalid provider label",
    );
  }
  const uniqueLabels = new Set(value);
  if (uniqueLabels.size !== value.length) {
    throw new Error(
      "generated eval metadata contains duplicate configured provider labels",
    );
  }
  return uniqueLabels;
}

export function parseProviderCompositions(value, options = {}) {
  if (!Array.isArray(value)) {
    throw new Error("generated eval metadata is missing providerCompositions");
  }
  if (value.length === 0) {
    throw new Error("providerCompositions must contain at least one provider");
  }

  const labels = new Set();
  const codexPluginSetsByMode = new Map();
  const providerCompositions = value.map((composition) => {
    if (
      !composition ||
      typeof composition !== "object" ||
      typeof composition.label !== "string" ||
      composition.label.length === 0 ||
      typeof composition.provider !== "string" ||
      typeof composition.providerVariant !== "string" ||
      composition.providerVariant.length === 0 ||
      typeof composition.pluginMode !== "string" ||
      composition.pluginMode.length === 0 ||
      !Array.isArray(composition.plugins)
    ) {
      throw new Error(
        "generated eval metadata contains an invalid provider composition",
      );
    }
    if (!supportedProviders.has(composition.provider)) {
      throw new Error(
        `unsupported provider in provider composition: ${composition.provider}`,
      );
    }
    if (!supportedPluginModes.has(composition.pluginMode)) {
      throw new Error(
        `unsupported plugin mode in provider composition: ${composition.pluginMode}`,
      );
    }
    if (
      composition.label !==
      `${composition.providerVariant}-${composition.pluginMode}`
    ) {
      throw new Error(
        `provider composition label does not match its variant and mode: ${composition.label}`,
      );
    }
    if (labels.has(composition.label)) {
      throw new Error(
        `generated eval metadata contains duplicate provider label: ${composition.label}`,
      );
    }
    labels.add(composition.label);

    if (
      composition.plugins.some(
        (plugin) =>
          typeof plugin !== "string" || !pluginNamePattern.test(plugin),
      )
    ) {
      throw new Error(
        `invalid plugin list for provider composition ${composition.label}`,
      );
    }
    const canonicalPlugins = [...new Set(composition.plugins)].sort();
    if (!sameStringLists(canonicalPlugins, composition.plugins)) {
      throw new Error(
        `non-canonical plugin list for provider composition ${composition.label}`,
      );
    }
    if (
      composition.pluginMode === "no-plugins" &&
      composition.plugins.length !== 0
    ) {
      throw new Error("no-plugins provider composition must be empty");
    }
    if (
      composition.pluginMode !== "no-plugins" &&
      composition.plugins.length === 0
    ) {
      throw new Error(
        `${composition.pluginMode.replace("-plugins", "")} provider composition must not be empty`,
      );
    }

    const parsed = {
      label: composition.label,
      provider: composition.provider,
      providerVariant: composition.providerVariant,
      pluginMode: composition.pluginMode,
      plugins: [...composition.plugins],
    };
    if (parsed.provider === "openai:codex-sdk") {
      const pluginSets = codexPluginSetsByMode.get(parsed.pluginMode) || [];
      pluginSets.push(parsed.plugins);
      codexPluginSetsByMode.set(parsed.pluginMode, pluginSets);
    }
    return parsed;
  });

  const codexPluginSelections = [];
  for (const [pluginMode, pluginSets] of codexPluginSetsByMode) {
    const expected = pluginSets[0];
    if (pluginSets.some((plugins) => !sameStringLists(plugins, expected))) {
      throw new Error(
        `inconsistent Codex provider compositions for ${pluginMode}`,
      );
    }
    codexPluginSelections.push({
      pluginMode,
      plugins: [...expected],
    });
  }

  if (Object.hasOwn(options, "expectedProviderLabels")) {
    const expectedLabels = parseExpectedProviderLabels(
      options.expectedProviderLabels,
    );
    const missing = [...expectedLabels]
      .filter((label) => !labels.has(label))
      .sort();
    const extra = [...labels]
      .filter((label) => !expectedLabels.has(label))
      .sort();
    if (missing.length > 0 || extra.length > 0) {
      const details = [];
      if (missing.length > 0) {
        details.push(`missing: ${missing.join(", ")}`);
      }
      if (extra.length > 0) {
        details.push(`extra: ${extra.join(", ")}`);
      }
      throw new Error(
        `provider composition labels do not match configured providers: ${details.join("; ")}`,
      );
    }
  }

  return { providerCompositions, codexPluginSelections };
}

function printCodexPluginSelections(metadataFile) {
  const metadata = JSON.parse(fs.readFileSync(metadataFile, "utf8"));
  const { codexPluginSelections } = parseProviderCompositions(
    metadata.providerCompositions,
    { expectedProviderLabels: metadata.providerLabels },
  );
  for (const selection of codexPluginSelections) {
    process.stdout.write(
      `${selection.pluginMode}\t${selection.plugins.join(",")}\n`,
    );
  }
}

const invokedPath =
  process.argv[1] && pathToFileURL(path.resolve(process.argv[1]));
if (invokedPath?.href === import.meta.url) {
  if (!process.argv[2]) {
    throw new Error("provider composition metadata file is required");
  }
  printCodexPluginSelections(process.argv[2]);
}
