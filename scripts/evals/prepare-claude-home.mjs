#!/usr/bin/env node
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";

const root = path.resolve(import.meta.dirname, "../..");
const pluginId = "development-system@ai-plugins";
const markerName = ".ai-plugins-claude-eval-home";
const markerContents = "ai-plugins Claude eval home\n";

function parseArgs(argv) {
  const args = {
    evalHome: argv[0],
    pluginMode: "development-system",
  };

  for (let index = 1; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--plugin-mode") {
      args.pluginMode = argv[++index];
    } else {
      throw new Error(`unknown argument: ${arg}`);
    }
  }

  if (!args.evalHome) {
    throw new Error(
      "Usage: node scripts/evals/prepare-claude-home.mjs <eval-home> [--plugin-mode no-plugins|development-system]",
    );
  }
  if (!["no-plugins", "development-system"].includes(args.pluginMode)) {
    throw new Error(`unknown plugin mode: ${args.pluginMode}`);
  }
  return args;
}

function realPathIfExists(entry) {
  try {
    return fs.realpathSync(entry);
  } catch {
    return path.resolve(entry);
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

function authSourceHome() {
  return (
    process.env.CLAUDE_EVAL_AUTH_HOME ||
    process.env.CLAUDE_CONFIG_DIR ||
    path.join(os.homedir(), ".claude")
  );
}

function isOwnedEvalHome(evalHome) {
  if (!fs.existsSync(evalHome)) return true;
  if (!fs.statSync(evalHome).isDirectory()) return false;
  const entries = fs.readdirSync(evalHome);
  if (entries.length === 0) return true;
  const marker = path.join(evalHome, markerName);
  return (
    fs.existsSync(marker) && fs.readFileSync(marker, "utf8") === markerContents
  );
}

function assertEvalHomeCanBeRecreated(evalHome) {
  const realTarget = realPathIfExists(evalHome);
  const realAuthSource = realPathIfExists(authSourceHome());
  const realDefaultConfig = realPathIfExists(
    path.join(os.homedir(), ".claude"),
  );

  if (pathsOverlap(realTarget, realAuthSource)) {
    throw new Error(
      "refusing Claude eval home path that overlaps the auth source",
    );
  }
  if (pathsOverlap(realTarget, realDefaultConfig)) {
    throw new Error(
      "refusing Claude eval home path that overlaps the real Claude config",
    );
  }
  for (const protectedRoot of [root, os.homedir()]) {
    if (isSameOrAncestor(realTarget, realPathIfExists(protectedRoot))) {
      throw new Error(
        `refusing Claude eval home path that contains protected root: ${protectedRoot}`,
      );
    }
  }
  if (!isOwnedEvalHome(evalHome)) {
    throw new Error(
      `refusing to replace unowned Claude eval home: ${evalHome}`,
    );
  }
}

function runClaude(claudeBin, args, env, { parseJson = false } = {}) {
  const result = spawnSync(claudeBin, args, {
    cwd: root,
    env,
    encoding: "utf8",
  });
  if (result.error) {
    throw new Error(`failed to run Claude CLI: ${result.error.message}`);
  }
  if (result.status !== 0) {
    const detail = (result.stderr || result.stdout || "").trim();
    throw new Error(
      `Claude CLI failed (${args.join(" ")}): ${detail || `status ${result.status}`}`,
    );
  }
  if (!parseJson) return result.stdout;
  try {
    return JSON.parse(result.stdout);
  } catch {
    throw new Error(`Claude CLI returned invalid JSON (${args.join(" ")})`);
  }
}

function installDevelopmentSystem(configDir, pluginCacheDir) {
  const claudeBin = process.env.CLAUDE_BIN || "claude";
  const env = {
    ...process.env,
    CLAUDE_CONFIG_DIR: configDir,
    CLAUDE_CODE_PLUGIN_CACHE_DIR: pluginCacheDir,
    CLAUDE_CODE_DISABLE_AUTO_MEMORY: "1",
  };

  runClaude(
    claudeBin,
    ["plugin", "marketplace", "add", root, "--scope", "user"],
    env,
  );
  runClaude(claudeBin, ["plugin", "install", pluginId, "--scope", "user"], env);
  const installed = runClaude(claudeBin, ["plugin", "list", "--json"], env, {
    parseJson: true,
  });
  const plugin = installed.find((entry) => entry.id === pluginId);

  if (
    !plugin ||
    plugin.enabled !== true ||
    typeof plugin.installPath !== "string"
  ) {
    throw new Error(`Claude installer did not enable ${pluginId}`);
  }
  if (Array.isArray(plugin.errors) && plugin.errors.length > 0) {
    throw new Error(
      `installed Claude plugin reported component errors: ${plugin.errors.join("; ")}`,
    );
  }

  const realCache = realPathIfExists(pluginCacheDir);
  const realInstall = realPathIfExists(plugin.installPath);
  if (!isSameOrAncestor(realCache, realInstall)) {
    throw new Error(
      `Claude installer reported a plugin path outside the eval cache: ${plugin.installPath}`,
    );
  }
  runClaude(claudeBin, ["plugin", "validate", plugin.installPath], env);
  return plugin.installPath;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const evalHome = path.resolve(args.evalHome);
  assertEvalHomeCanBeRecreated(evalHome);

  fs.rmSync(evalHome, { recursive: true, force: true });
  const configDir = path.join(evalHome, "config");
  const pluginCacheDir = path.join(evalHome, "plugin-cache");
  fs.mkdirSync(configDir, { recursive: true });
  fs.mkdirSync(pluginCacheDir, { recursive: true });
  fs.writeFileSync(path.join(evalHome, markerName), markerContents, {
    mode: 0o600,
  });

  const pluginPath =
    args.pluginMode === "development-system"
      ? installDevelopmentSystem(configDir, pluginCacheDir)
      : null;

  process.stdout.write(
    `${JSON.stringify({
      pluginMode: args.pluginMode,
      configDir,
      pluginCacheDir,
      pluginPath,
    })}\n`,
  );
}

try {
  main();
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(2);
}
