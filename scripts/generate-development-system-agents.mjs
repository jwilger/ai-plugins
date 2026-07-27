#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const root = path.resolve(import.meta.dirname, "..");
const component = path.join(
  root,
  "plugins/development-system/components/development-discipline",
);
const sourceFile = path.join(component, "agent-sources/review-agents.json");
const source = JSON.parse(fs.readFileSync(sourceFile, "utf8"));
const write = process.argv.includes("--write");
if (!write && !process.argv.includes("--check")) {
  console.error(
    "usage: generate-development-system-agents.mjs --check|--write",
  );
  process.exit(2);
}
if (source.schemaVersion !== 1 || !Array.isArray(source.agents))
  throw new Error("invalid review agent source schema");

function markdown(agent, file) {
  const raw = `---\nname: ${agent.name}\ndescription: ${agent.description}\nmodel: ${agent.claude.model}\ntools: ${agent.claude.tools}\n---\n\n${agent.instructions}${agent.claude.addendum ? `\n\n${agent.claude.addendum}` : ""}\n`;
  const formatted = spawnSync("prettier", ["--stdin-filepath", file], {
    input: raw,
    encoding: "utf8",
  });
  if (formatted.status !== 0)
    throw new Error(`prettier failed for ${file}: ${formatted.stderr}`);
  return formatted.stdout;
}

function tomlString(value) {
  if (value.includes('"""'))
    throw new Error("agent instructions cannot contain TOML triple quotes");
  return `"""\n${value}\n"""`;
}

function codex(agent) {
  return `name = ${JSON.stringify(agent.name)}\ndescription = ${JSON.stringify(agent.description)}\nmodel = ${JSON.stringify(agent.codex.model)}\nmodel_reasoning_effort = ${JSON.stringify(agent.codex.reasoning)}\nsandbox_mode = ${JSON.stringify(agent.codex.sandbox)}\ndeveloper_instructions = ${tomlString(agent.instructions + (agent.codex.addendum ? `\n\n${agent.codex.addendum}` : ""))}\n`;
}

let drift = false;
for (const agent of source.agents) {
  if (!/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(agent.id))
    throw new Error(`invalid agent id ${agent.id}`);
  for (const [extension, content] of [
    ["md", markdown(agent, `${agent.id}.md`)],
    ["toml", codex(agent)],
  ]) {
    const file = path.join(component, "agents", `${agent.id}.${extension}`);
    const current = fs.existsSync(file) ? fs.readFileSync(file, "utf8") : null;
    if (current === content) continue;
    drift = true;
    if (write) fs.writeFileSync(file, content);
    else console.error(`generated agent drift: ${path.relative(root, file)}`);
  }
}
if (drift && !write) process.exit(1);
console.log(
  `development_system.agents generated=${source.agents.length * 2} synchronized=${!drift || write}`,
);
