#!/usr/bin/env node
import { spawnSync } from "node:child_process";
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
    pluginMode: "development-system",
    seedAuth: true,
    installViaCli: false,
  };

  for (let index = 1; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--plugin-mode") {
      args.pluginMode = argv[++index];
    } else if (arg === "--no-seed-auth") {
      args.seedAuth = false;
    } else if (arg === "--install-via-cli") {
      args.installViaCli = true;
    } else {
      throw new Error(`unknown argument: ${arg}`);
    }
  }

  if (!args.codexHome) {
    throw new Error(
      "Usage: node scripts/evals/prepare-codex-home.mjs <codex-home> [--plugin-mode no-plugins|development-system] [--no-seed-auth] [--install-via-cli]",
    );
  }
  if (!["no-plugins", "development-system"].includes(args.pluginMode)) {
    throw new Error(`unknown plugin mode: ${args.pluginMode}`);
  }
  if (args.installViaCli && args.pluginMode !== "development-system") {
    throw new Error(
      `--install-via-cli is not supported with plugin mode ${args.pluginMode}`,
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
      throw new Error(`unknown selected plugin(s): ${missing.join(", ")}`);
    }
  }

  return plugins;
}

function copyDir(source, target) {
  fs.rmSync(target, { recursive: true, force: true });
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.cpSync(source, target, {
    recursive: true,
    filter: (entry) => {
      if (entry.split(path.sep).includes(".git")) return false;
      return true;
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

function runCodex(codexHome, args) {
  const codexBin =
    process.env.CODEX_EVAL_CODEX_BIN ||
    path.join(root, "node_modules/.bin/codex");
  const result = spawnSync(codexBin, args, {
    encoding: "utf8",
    env: { ...process.env, CODEX_HOME: codexHome },
  });

  if (result.error) {
    throw new Error(`failed to execute Codex CLI: ${result.error.message}`);
  }
  if (result.status !== 0) {
    const detail = result.stderr.trim() || result.stdout.trim();
    throw new Error(
      `Codex CLI ${args.join(" ")} failed with status ${result.status}${detail ? `: ${detail}` : ""}`,
    );
  }

  try {
    return JSON.parse(result.stdout);
  } catch {
    throw new Error(`Codex CLI ${args.join(" ")} did not return JSON`);
  }
}

function installDevelopmentSystemWithCli(codexHome) {
  runCodex(codexHome, ["plugin", "marketplace", "add", root, "--json"]);
  const installation = runCodex(codexHome, [
    "plugin",
    "add",
    "development-system@ai-plugins",
    "--json",
  ]);
  const listing = runCodex(codexHome, ["plugin", "list", "--json"]);
  const installed = listing.installed?.find(
    (plugin) => plugin.pluginId === "development-system@ai-plugins",
  );

  if (!installed?.installed || !installed.enabled) {
    throw new Error(
      "Codex CLI did not report development-system as installed and enabled",
    );
  }

  const installedPath = path.resolve(installation.installedPath || "");
  if (
    !installation.installedPath ||
    !isSameOrAncestor(
      realPathIfExists(codexHome),
      realPathIfExists(installedPath),
    )
  ) {
    throw new Error(
      "Codex CLI reported a development-system install path outside the eval home",
    );
  }
  if (!fs.statSync(installedPath).isDirectory()) {
    throw new Error(
      "Codex CLI did not materialize the development-system plugin cache",
    );
  }
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
    args.pluginMode === "no-plugins"
      ? []
      : marketplacePlugins(["development-system"]);

  if (!initializeInPlace) {
    fs.rmSync(resolvedHome, { recursive: true, force: true });
  }
  fs.mkdirSync(resolvedHome, { recursive: true });
  fs.writeFileSync(
    path.join(resolvedHome, evalHomeMarker),
    evalHomeMarkerContents,
    { mode: 0o600 },
  );
  if (args.seedAuth) seedAuth(resolvedHome);

  if (args.installViaCli) {
    installDevelopmentSystemWithCli(resolvedHome);
  } else {
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
      );
    }
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
