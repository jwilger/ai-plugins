const fs = require('fs');
const path = require('path');

const ROOT = path.resolve(process.cwd());

function manifestPlugins(file) {
  const manifest = JSON.parse(fs.readFileSync(path.join(ROOT, file), 'utf8'));
  return manifest.plugins.map((plugin) => ({
    name: plugin.name,
    source:
      plugin.source && typeof plugin.source === 'object'
        ? plugin.source.path
        : plugin.source,
  }));
}

function titleCase(name) {
  return name
    .split('-')
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ');
}

function pluginPath(plugin) {
  const source = plugin.source || `./plugins/${plugin.name}`;
  return path.resolve(ROOT, source);
}

function skillNames(plugin) {
  const skillsDir = path.join(pluginPath(plugin), 'skills');

  if (!fs.existsSync(skillsDir)) {
    return [];
  }

  return fs
    .readdirSync(skillsDir, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name);
}

module.exports = function assertFullMarketplaceCanary(output) {
  const text = String(output || '').toLowerCase();
  const plugins = new Map([
    ...manifestPlugins('.claude-plugin/marketplace.json'),
    ...manifestPlugins('.agents/plugins/marketplace.json'),
  ].map((plugin) => [plugin.name, plugin]));
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
      reason: `Missing plugin names in canary response: ${missing.join(', ')}`,
    };
  }

  const missingSkills = [...plugins.values()].filter((plugin) => {
    const candidates = skillNames(plugin);

    if (candidates.length === 0) {
      return false;
    }

    return !candidates.some((skill) => text.includes(skill.toLowerCase()));
  });

  if (missingSkills.length > 0) {
    return {
      pass: false,
      score: 0,
      reason: `Missing representative skill(s) for plugin(s): ${missingSkills.map((plugin) => plugin.name).join(', ')}`,
    };
  }

  return {
    pass: true,
    score: 1,
    reason: 'Full marketplace canary named every plugin and representative skill',
  };
};
