#!/usr/bin/env node
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import process from "node:process";

const root = path.resolve(import.meta.dirname, "../..");
const evalHomeMarker = ".ai-plugins-eval-home";
const evalHomeMarkerContents = "ai-plugins Codex eval home\n";

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, "utf8"));
}

function parseArgs(argv) {
  const args = {
    codexHome: argv[0],
    pluginMode: "full-marketplace",
    plugins: null,
    pluginsProvided: false,
  };

  for (let index = 1; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--plugin-mode") {
      args.pluginMode = argv[++index];
    } else if (arg === "--plugins") {
      const value = argv[++index];
      if (value === undefined) {
        throw new Error("--plugins requires a comma-separated plugin list");
      }
      args.pluginsProvided = true;
      args.plugins = value
        .split(",")
        .map((plugin) => plugin.trim())
        .filter(Boolean);
    } else {
      throw new Error(`unknown argument: ${arg}`);
    }
  }

  if (!args.codexHome) {
    throw new Error(
      "Usage: node scripts/evals/prepare-codex-home.mjs <codex-home> [--plugin-mode no-plugins|targeted-plugins|full-marketplace|skills-only-marketplace] [--plugins comma,list]",
    );
  }
  if (
    ![
      "no-plugins",
      "targeted-plugins",
      "full-marketplace",
      "skills-only-marketplace",
    ].includes(args.pluginMode)
  ) {
    throw new Error(`unknown plugin mode: ${args.pluginMode}`);
  }
  if (
    args.pluginsProvided &&
    ["targeted-plugins", "skills-only-marketplace"].includes(args.pluginMode) &&
    args.plugins.length === 0
  ) {
    throw new Error(
      `${args.pluginMode} mode requires a non-empty --plugins list`,
    );
  }
  if (args.pluginMode === "targeted-plugins" && !args.pluginsProvided) {
    throw new Error("targeted-plugins mode requires --plugins");
  }
  if (
    args.pluginsProvided &&
    !["targeted-plugins", "skills-only-marketplace"].includes(args.pluginMode)
  ) {
    throw new Error(
      `--plugins is not supported with plugin mode ${args.pluginMode}`,
    );
  }

  return args;
}

function marketplacePlugins(selectedNames = null) {
  const selected = selectedNames ? new Set(selectedNames) : null;
  const manifest = readJson(
    path.join(root, ".agents/plugins/marketplace.json"),
  );
  const plugins = manifest.plugins
    .filter((plugin) => !selected || selected.has(plugin.name))
    .map((plugin) => {
      const pluginPath = path.resolve(root, plugin.source.path);
      const pluginJson = readJson(
        path.join(pluginPath, ".codex-plugin/plugin.json"),
      );

      return {
        name: plugin.name,
        version: pluginJson.version || "local",
        path: pluginPath,
      };
    });

  if (selected) {
    const found = new Set(plugins.map((plugin) => plugin.name));
    const missing = [...selected].filter((name) => !found.has(name));
    if (missing.length > 0) {
      throw new Error(`unknown targeted plugin(s): ${missing.join(", ")}`);
    }
  }

  return plugins;
}

function copyDir(source, target, { skillsOnly = false } = {}) {
  fs.rmSync(target, { recursive: true, force: true });
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.cpSync(source, target, {
    recursive: true,
    filter: (entry) => {
      if (entry.split(path.sep).includes(".git")) return false;
      if (!skillsOnly) return true;

      const relative = path.relative(source, entry);
      if (!relative) return true;
      const [rootEntry] = relative.split(path.sep);
      return rootEntry === ".codex-plugin" || rootEntry === "skills";
    },
  });
}

function escapeToml(value) {
  return String(value).replaceAll("\\", "\\\\").replaceAll('"', '\\"');
}

function writeConfig(codexHome, plugins) {
  const lines = [
    "[marketplaces.ai-plugins]",
    `last_updated = "${new Date().toISOString()}"`,
    'source_type = "local"',
    `source = "${escapeToml(root)}"`,
    "",
  ];

  for (const plugin of plugins) {
    lines.push(`[plugins."${escapeToml(plugin.name)}@ai-plugins"]`);
    lines.push("enabled = true");
    lines.push("");
  }

  fs.writeFileSync(path.join(codexHome, "config.toml"), lines.join("\n"));
}

function seedAuth(codexHome) {
  if (process.env.OPENAI_API_KEY) return;

  const authHome = authSourceHome();

  for (const filename of ["auth.json", ".credentials.json"]) {
    const source = path.join(authHome, filename);
    const target = path.join(codexHome, filename);

    if (fs.existsSync(source)) {
      fs.copyFileSync(source, target);
      fs.chmodSync(target, 0o600);
    }
  }
}

function authSourceHome() {
  return (
    process.env.CODEX_EVAL_AUTH_HOME ||
    process.env.CODEX_HOME ||
    path.join(os.homedir(), ".codex")
  );
}

function realPathIfExists(entry) {
  try {
    return fs.realpathSync(entry);
  } catch {
    return path.resolve(entry);
  }
}

function assertEvalHomeIsIsolated(resolvedHome) {
  const realTarget = realPathIfExists(resolvedHome);
  const realDefaultCodexHome = realPathIfExists(
    path.join(os.homedir(), ".codex"),
  );
  const realAuthSourceHome = realPathIfExists(authSourceHome());

  if (
    realTarget === realDefaultCodexHome &&
    process.env.CODEX_EVAL_ALLOW_REAL_HOME !== "1"
  ) {
    console.error(
      "refusing to prepare real Codex home; set CODEX_EVAL_ALLOW_REAL_HOME=1 to override",
    );
    process.exit(2);
  }

  if (
    realTarget === realAuthSourceHome &&
    process.env.CODEX_EVAL_ALLOW_AUTH_HOME !== "1"
  ) {
    console.error(
      "refusing to prepare auth source Codex home; set CODEX_EVAL_ALLOW_AUTH_HOME=1 to override",
    );
    process.exit(2);
  }
}

function isSameOrAncestor(ancestor, descendant) {
  const relative = path.relative(ancestor, descendant);
  return (
    relative === "" ||
    (!relative.startsWith(`..${path.sep}`) &&
      relative !== ".." &&
      !path.isAbsolute(relative))
  );
}

function pathsOverlap(first, second) {
  return isSameOrAncestor(first, second) || isSameOrAncestor(second, first);
}

function isOwnedEvalHome(resolvedHome) {
  if (!fs.existsSync(resolvedHome)) return true;
  if (!fs.statSync(resolvedHome).isDirectory()) return false;

  const entries = fs.readdirSync(resolvedHome);
  if (entries.length === 0) return true;

  const marker = path.join(resolvedHome, evalHomeMarker);
  if (
    fs.existsSync(marker) &&
    fs.readFileSync(marker, "utf8") === evalHomeMarkerContents
  ) {
    return true;
  }

  return false;
}

function isEmptyDirectory(entry) {
  return (
    fs.existsSync(entry) &&
    fs.statSync(entry).isDirectory() &&
    fs.readdirSync(entry).length === 0
  );
}

function assertEvalHomeCanBeRecreated(resolvedHome) {
  const realTarget = realPathIfExists(resolvedHome);
  const realAuthSourceHome = realPathIfExists(authSourceHome());
  const realDefaultCodexHome = realPathIfExists(
    path.join(os.homedir(), ".codex"),
  );

  if (
    process.env.CODEX_EVAL_ALLOW_AUTH_HOME !== "1" &&
    pathsOverlap(realTarget, realAuthSourceHome)
  ) {
    throw new Error(
      "refusing Codex eval home path that overlaps the auth source",
    );
  }
  if (
    process.env.CODEX_EVAL_ALLOW_REAL_HOME !== "1" &&
    pathsOverlap(realTarget, realDefaultCodexHome)
  ) {
    throw new Error(
      "refusing Codex eval home path that overlaps the real Codex home",
    );
  }

  for (const protectedRoot of [root, os.homedir()]) {
    if (isSameOrAncestor(realTarget, realPathIfExists(protectedRoot))) {
      throw new Error(
        `refusing Codex eval home path that contains protected root: ${protectedRoot}`,
      );
    }
  }

  if (!isOwnedEvalHome(resolvedHome)) {
    throw new Error(
      `refusing to replace unowned Codex eval home: ${resolvedHome}`,
    );
  }
}

function main() {
  const args = parseArgs(process.argv.slice(2));

  const resolvedHome = path.resolve(args.codexHome);
  const initializeInPlace = isEmptyDirectory(resolvedHome);
  assertEvalHomeIsIsolated(resolvedHome);
  assertEvalHomeCanBeRecreated(resolvedHome);

  const plugins =
    args.pluginMode === "no-plugins" ? [] : marketplacePlugins(args.plugins);

  if (!initializeInPlace) {
    fs.rmSync(resolvedHome, { recursive: true, force: true });
  }
  fs.mkdirSync(resolvedHome, { recursive: true });
  fs.writeFileSync(
    path.join(resolvedHome, evalHomeMarker),
    evalHomeMarkerContents,
    { mode: 0o600 },
  );
  seedAuth(resolvedHome);
  writeConfig(resolvedHome, plugins);

  for (const plugin of plugins) {
    copyDir(
      plugin.path,
      path.join(
        resolvedHome,
        "plugins/cache/ai-plugins",
        plugin.name,
        plugin.version,
      ),
      { skillsOnly: args.pluginMode === "skills-only-marketplace" },
    );
  }

  console.log(
    `prepared ${resolvedHome} with ${plugins.length} ai-plugins (${args.pluginMode})`,
  );
}

try {
  main();
} catch (error) {
  console.error(error.message);
  process.exit(2);
}
