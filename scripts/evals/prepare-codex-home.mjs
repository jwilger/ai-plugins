#!/usr/bin/env node
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import process from 'node:process';

const root = path.resolve(import.meta.dirname, '../..');

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, 'utf8'));
}

function parseArgs(argv) {
  const args = {
    codexHome: argv[0],
    pluginMode: 'full-marketplace',
    plugins: null,
  };

  for (let index = 1; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === '--plugin-mode') {
      args.pluginMode = argv[++index];
    } else if (arg === '--plugins') {
      args.plugins = argv[++index]
        .split(',')
        .map((plugin) => plugin.trim())
        .filter(Boolean);
    } else {
      throw new Error(`unknown argument: ${arg}`);
    }
  }

  if (!args.codexHome) {
    throw new Error(
      'Usage: node scripts/evals/prepare-codex-home.mjs <codex-home> [--plugin-mode no-plugins|targeted-plugins|full-marketplace] [--plugins comma,list]',
    );
  }
  if (!['no-plugins', 'targeted-plugins', 'full-marketplace'].includes(args.pluginMode)) {
    throw new Error(`unknown plugin mode: ${args.pluginMode}`);
  }
  if (args.pluginMode === 'targeted-plugins' && (!args.plugins || args.plugins.length === 0)) {
    throw new Error('targeted-plugins mode requires --plugins');
  }

  return args;
}

function marketplacePlugins(selectedNames = null) {
  const selected = selectedNames ? new Set(selectedNames) : null;
  const manifest = readJson(path.join(root, '.agents/plugins/marketplace.json'));
  const plugins = manifest.plugins
    .filter((plugin) => !selected || selected.has(plugin.name))
    .map((plugin) => {
      const pluginPath = path.resolve(root, plugin.source.path);
      const pluginJson = readJson(
        path.join(pluginPath, '.codex-plugin/plugin.json'),
      );

      return {
        name: plugin.name,
        version: pluginJson.version || 'local',
        path: pluginPath,
      };
    });

  if (selected) {
    const found = new Set(plugins.map((plugin) => plugin.name));
    const missing = [...selected].filter((name) => !found.has(name));
    if (missing.length > 0) {
      throw new Error(`unknown targeted plugin(s): ${missing.join(', ')}`);
    }
  }

  return plugins;
}

function copyDir(source, target) {
  fs.rmSync(target, { recursive: true, force: true });
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.cpSync(source, target, {
    recursive: true,
    filter: (entry) => !entry.split(path.sep).includes('.git'),
  });
}

function escapeToml(value) {
  return String(value).replaceAll('\\', '\\\\').replaceAll('"', '\\"');
}

function writeConfig(codexHome, plugins) {
  const lines = [
    '[marketplaces.ai-plugins]',
    `last_updated = "${new Date().toISOString()}"`,
    'source_type = "local"',
    `source = "${escapeToml(root)}"`,
    '',
  ];

  for (const plugin of plugins) {
    lines.push(`[plugins."${escapeToml(plugin.name)}@ai-plugins"]`);
    lines.push('enabled = true');
    lines.push('');
  }

  fs.writeFileSync(path.join(codexHome, 'config.toml'), lines.join('\n'));
}

function seedAuth(codexHome) {
  if (process.env.OPENAI_API_KEY) return;

  const authHome = authSourceHome();

  for (const filename of ['auth.json', '.credentials.json']) {
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
    path.join(os.homedir(), '.codex')
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
  const realDefaultCodexHome = realPathIfExists(path.join(os.homedir(), '.codex'));
  const realAuthSourceHome = realPathIfExists(authSourceHome());

  if (
    realTarget === realDefaultCodexHome &&
    process.env.CODEX_EVAL_ALLOW_REAL_HOME !== '1'
  ) {
    console.error(
      'refusing to prepare real Codex home; set CODEX_EVAL_ALLOW_REAL_HOME=1 to override',
    );
    process.exit(2);
  }

  if (
    realTarget === realAuthSourceHome &&
    process.env.CODEX_EVAL_ALLOW_AUTH_HOME !== '1'
  ) {
    console.error(
      'refusing to prepare auth source Codex home; set CODEX_EVAL_ALLOW_AUTH_HOME=1 to override',
    );
    process.exit(2);
  }
}

function main() {
  const args = parseArgs(process.argv.slice(2));

  const resolvedHome = path.resolve(args.codexHome);
  assertEvalHomeIsIsolated(resolvedHome);

  const plugins =
    args.pluginMode === 'no-plugins'
      ? []
      : marketplacePlugins(args.pluginMode === 'targeted-plugins' ? args.plugins : null);

  fs.mkdirSync(resolvedHome, { recursive: true });
  seedAuth(resolvedHome);
  writeConfig(resolvedHome, plugins);

  for (const plugin of plugins) {
    copyDir(
      plugin.path,
      path.join(
        resolvedHome,
        'plugins/cache/ai-plugins',
        plugin.name,
        plugin.version,
      ),
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
