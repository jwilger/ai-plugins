#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const { coverageKinds, loadBehaviorCases } = require("../../evals/promptfoo/fixtures.cjs");

const REQUIRED_KINDS = [
  "natural-trigger",
  "scope-boundary",
  "core-behavior",
  "adversarial-safety",
  "baseline-ablation",
];

function parseArgs(argv) {
  const args = { root: path.resolve(import.meta.dirname, "../..") };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--root") {
      args.root = path.resolve(argv[++index]);
    } else {
      throw new Error(`unknown argument: ${arg}`);
    }
  }
  return args;
}

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, "utf8"));
}

function marketplacePlugins(root) {
  const manifests = [
    path.join(root, ".claude-plugin/marketplace.json"),
    path.join(root, ".agents/plugins/marketplace.json"),
  ];
  const plugins = new Map();

  for (const manifestPath of manifests) {
    if (!fs.existsSync(manifestPath)) continue;
    const manifest = readJson(manifestPath);
    for (const plugin of manifest.plugins || []) {
      const source =
        plugin.source && typeof plugin.source === "object"
          ? plugin.source.path
          : plugin.source;
      const pluginPath = source?.startsWith("./") ? source : `./${source}`;
      plugins.set(plugin.name, path.resolve(root, pluginPath));
    }
  }

  return [...plugins.entries()]
    .map(([name, pluginPath]) => ({ name, path: pluginPath }))
    .sort((left, right) => left.name.localeCompare(right.name));
}

function skillDirectories(plugin) {
  const skillsRoot = path.join(plugin.path, "skills");
  if (!fs.existsSync(skillsRoot)) return [];

  return fs
    .readdirSync(skillsRoot, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => ({
      plugin: plugin.name,
      skill: entry.name,
      path: path.join(skillsRoot, entry.name, "SKILL.md"),
    }))
    .filter((skill) => fs.existsSync(skill.path));
}

function explicitDecisionFor(cases, plugin, skill) {
  return cases.some((testCase) => {
    const decision = testCase.coverageDecision;
    return (
      decision &&
      decision.plugin === plugin &&
      decision.skill === skill &&
      ["value-proven", "pruned", "deferred"].includes(decision.decision) &&
      typeof decision.reason === "string" &&
      decision.reason.length > 20
    );
  });
}

function coverageFor(cases, plugin, skill) {
  const kinds = new Set();
  for (const testCase of cases) {
    if (
      (testCase.plugins || []).includes(plugin) &&
      (testCase.skills || []).includes(skill)
    ) {
      for (const kind of coverageKinds(testCase)) {
        kinds.add(kind);
      }
    }
  }
  return kinds;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const plugins = marketplacePlugins(args.root);
  const skills = plugins.flatMap(skillDirectories);
  const cases = loadBehaviorCases({ root: args.root });
  const failures = [];

  for (const skill of skills) {
    if (explicitDecisionFor(cases, skill.plugin, skill.skill)) {
      continue;
    }
    const kinds = coverageFor(cases, skill.plugin, skill.skill);
    const missing = REQUIRED_KINDS.filter((kind) => !kinds.has(kind));
    if (missing.length > 0) {
      failures.push(
        `${skill.plugin}:${skill.skill} missing coverage kinds: ${missing.join(", ")}`,
      );
    }
  }

  if (failures.length > 0) {
    console.error(failures.join("\n"));
    process.exit(1);
  }

  console.log(`coverage complete for ${skills.length} skills`);
}

try {
  main();
} catch (error) {
  console.error(error.message);
  process.exit(2);
}
