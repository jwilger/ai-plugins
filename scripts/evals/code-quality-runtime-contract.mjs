import crypto from "node:crypto";

export const evalHomeMarkerName = ".ai-plugins-eval-home";
export const evalHomeMarkerContents = "ai-plugins Codex eval home\n";
export const executionSurfaceName = ".ai-plugins-execution-surface.json";
export const runtimeMarketplaceMount = "/runtime/marketplace";

export const credentialNames = new Set(["auth.json", ".credentials.json"]);
export const identifierPattern = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;
export const sha256Pattern = /^[0-9a-f]{64}$/;
export const versionPattern = /^[A-Za-z0-9]+(?:[A-Za-z0-9._+-]*[A-Za-z0-9])?$/;

const executionSurfaceKeys = [
  "boundarySha256",
  "codexBinarySha256",
  "codexVersion",
  "model",
  "reasoningEffort",
  "schemaVersion",
  "toolchainCompositionSha256",
];

export function canonicalize(value) {
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

export function canonicalJson(value, indentation = 0) {
  return JSON.stringify(canonicalize(value), null, indentation);
}

export function sha256(value) {
  return crypto.createHash("sha256").update(value).digest("hex");
}

export function hashCanonical(value) {
  return sha256(canonicalJson(value));
}

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function exactKeys(value, keys) {
  return (
    isPlainObject(value) &&
    JSON.stringify(Object.keys(value).sort()) ===
      JSON.stringify([...keys].sort())
  );
}

function boundedText(value, pattern, maximum = 256) {
  return (
    typeof value === "string" &&
    value.length > 0 &&
    value.length <= maximum &&
    pattern.test(value)
  );
}

export function parseExecutionSurface(value) {
  if (
    !exactKeys(value, executionSurfaceKeys) ||
    value.schemaVersion !== 1 ||
    !sha256Pattern.test(value.boundarySha256) ||
    !sha256Pattern.test(value.codexBinarySha256) ||
    !sha256Pattern.test(value.toolchainCompositionSha256) ||
    !boundedText(
      value.codexVersion,
      /^codex-cli [0-9]+\.[0-9]+\.[0-9]+(?:[+-][A-Za-z0-9.-]+)?$/,
    ) ||
    !boundedText(value.model, /^[A-Za-z0-9][A-Za-z0-9._:+/-]*$/) ||
    !boundedText(
      value.reasoningEffort,
      /^(?:none|minimal|low|medium|high|xhigh)$/,
      16,
    )
  ) {
    throw new Error("execution-surface-invalid");
  }
  return canonicalize(value);
}

export function executionSurfaceFromEnvironment(environment = process.env) {
  return parseExecutionSurface({
    boundarySha256: environment.CODE_QUALITY_BOUNDARY_SHA256,
    codexBinarySha256: environment.CODE_QUALITY_CODEX_EXPECTED_SHA256,
    codexVersion: environment.CODE_QUALITY_CODEX_EXPECTED_VERSION,
    model: environment.CODE_QUALITY_CODEX_MODEL,
    reasoningEffort: environment.CODE_QUALITY_CODEX_REASONING_EFFORT,
    schemaVersion: 1,
    toolchainCompositionSha256:
      environment.CODE_QUALITY_TOOLCHAIN_COMPOSITION_SHA256,
  });
}

export function runtimeConfigForPlugins(plugins) {
  if (
    !Array.isArray(plugins) ||
    plugins.some(
      (plugin) =>
        !isPlainObject(plugin) || !identifierPattern.test(plugin.name || ""),
    )
  ) {
    throw new Error("runtime-config-plugins-invalid");
  }
  const lines = [
    "[marketplaces.ai-plugins]",
    'source_type = "local"',
    `source = "${runtimeMarketplaceMount}"`,
    "",
  ];
  for (const plugin of plugins) {
    lines.push(`[plugins."${plugin.name}@ai-plugins"]`);
    lines.push("enabled = true");
    lines.push("");
  }
  return `${lines.join("\n")}\n`;
}

function validateMarketplaceEntry(entry) {
  if (
    !isPlainObject(entry) ||
    !identifierPattern.test(entry.name || "") ||
    typeof entry.version !== "string" ||
    !versionPattern.test(entry.version) ||
    !isPlainObject(entry.source) ||
    entry.source.source !== "local" ||
    entry.source.path !== `./plugins/${entry.name}`
  ) {
    throw new Error("codex-marketplace-binding-invalid");
  }
}

export function selectMarketplacePlugins(contract, marketplace, mode) {
  if (
    !isPlainObject(contract) ||
    !isPlainObject(marketplace) ||
    !Array.isArray(contract.conditions) ||
    !Array.isArray(marketplace.plugins)
  ) {
    throw new Error("codex-marketplace-binding-invalid");
  }
  const condition = contract.conditions.find((entry) => entry.id === mode);
  if (!condition) throw new Error("runtime-condition-invalid");
  const allPlugins = new Map();
  for (const entry of marketplace.plugins) {
    validateMarketplaceEntry(entry);
    if (allPlugins.has(entry.name)) {
      throw new Error("codex-marketplace-binding-invalid");
    }
    allPlugins.set(entry.name, entry);
  }
  const selectedNames =
    condition.plugins === "codex-marketplace-skills-at-run-start"
      ? new Set(allPlugins.keys())
      : new Set(condition.plugins || []);
  for (const name of selectedNames) {
    if (!allPlugins.has(name)) {
      throw new Error("runtime-condition-plugin-missing");
    }
  }
  return marketplace.plugins
    .filter((entry) => selectedNames.has(entry.name))
    .map((entry) => ({
      name: entry.name,
      sourcePath: entry.source.path,
      version: entry.version,
    }));
}

export function sanitizedMarketplaceManifest(plugins) {
  return {
    interface: { displayName: "ai-plugins" },
    name: "ai-plugins",
    plugins: plugins.map((plugin) => ({
      name: plugin.name,
      source: {
        path: `./plugins/${plugin.name}`,
        source: "local",
      },
      version: plugin.version,
    })),
  };
}

export function sanitizedPluginManifest(plugin) {
  if (
    !isPlainObject(plugin) ||
    !identifierPattern.test(plugin.name || "") ||
    typeof plugin.version !== "string" ||
    !versionPattern.test(plugin.version)
  ) {
    throw new Error("projected-plugin-manifest-invalid");
  }
  return {
    name: plugin.name,
    skills: "./skills/",
    version: plugin.version,
  };
}
