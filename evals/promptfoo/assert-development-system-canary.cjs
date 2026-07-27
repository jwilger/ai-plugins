const fs = require("fs");
const path = require("path");

const ROOT = path.resolve(process.cwd());

function manifestPlugins(file) {
  const manifest = JSON.parse(fs.readFileSync(path.join(ROOT, file), "utf8"));
  return manifest.plugins.map((plugin) => ({
    name: plugin.name,
    source:
      plugin.source && typeof plugin.source === "object"
        ? plugin.source.path
        : plugin.source,
  }));
}

function titleCase(name) {
  return name
    .split("-")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function pluginPath(plugin) {
  const source = plugin.source || `./plugins/${plugin.name}`;
  return path.resolve(ROOT, source);
}

function skillNames(plugin) {
  const skillsDir = path.join(pluginPath(plugin), "skills");

  if (!fs.existsSync(skillsDir)) {
    return [];
  }

  return fs
    .readdirSync(skillsDir, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name);
}

function providerId(context) {
  const provider = context?.provider;
  if (!provider) return "";
  if (typeof provider.id === "function") return provider.id();
  if (typeof provider.id === "string") return provider.id;
  return "";
}

function expectedPlugins(context) {
  const id = providerId(context);
  const developmentSystemOnly = (plugins) =>
    plugins.filter((plugin) => plugin.name === "development-system");

  if (id.includes("anthropic:claude-agent-sdk")) {
    return developmentSystemOnly(
      manifestPlugins(".claude-plugin/marketplace.json"),
    );
  }

  if (id.includes("openai:codex-sdk")) {
    return developmentSystemOnly(
      manifestPlugins(".agents/plugins/marketplace.json"),
    );
  }

  return developmentSystemOnly([
    ...manifestPlugins(".claude-plugin/marketplace.json"),
    ...manifestPlugins(".agents/plugins/marketplace.json"),
  ]);
}

module.exports = function assertDevelopmentSystemCanary(output, context = {}) {
  const text = String(output || "").toLowerCase();
  const plugins = new Map(
    expectedPlugins(context).map((plugin) => [plugin.name, plugin]),
  );
  const missing = [...plugins.keys()].filter((name) => {
    const accepted = [name, titleCase(name)].map((candidate) =>
      candidate.toLowerCase(),
    );
    return !accepted.some((candidate) => text.includes(candidate));
  });

  if (missing.length > 0) {
    return {
      pass: false,
      score: 0,
      reason: `Missing plugin names in canary response: ${missing.join(", ")}`,
    };
  }

  const missingSkills = [...plugins.values()].filter((plugin) => {
    const candidates = skillNames(plugin);

    if (candidates.length === 0) {
      return false;
    }

    return !candidates.some((skill) => {
      const accepted = [skill, titleCase(skill)].map((candidate) =>
        candidate.toLowerCase(),
      );
      return accepted.some((candidate) => text.includes(candidate));
    });
  });

  if (missingSkills.length > 0) {
    return {
      pass: false,
      score: 0,
      reason: `Missing representative skill(s) for plugin(s): ${missingSkills.map((plugin) => plugin.name).join(", ")}`,
    };
  }

  return {
    pass: true,
    score: 1,
    reason:
      "Installed development-system canary named the plugin and a representative skill",
  };
};
