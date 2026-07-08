#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const root = path.resolve(import.meta.dirname, "../..");
const defaultOutput = path.join(root, "evals/out/status.json");

function readFlag(name, fallback = "") {
  const index = process.argv.indexOf(name);
  if (index === -1 || index + 1 >= process.argv.length) {
    return fallback;
  }

  return process.argv[index + 1];
}

const status = {
  generatedAt: new Date().toISOString(),
  suite: "agentic-systems-engineering",
  state: readFlag("--state", "unknown"),
  reason: readFlag("--reason", ""),
  providerCredentials: readFlag("--provider-credentials", "unknown"),
};

const output = path.resolve(readFlag("--output", defaultOutput));
fs.mkdirSync(path.dirname(output), { recursive: true });
fs.writeFileSync(output, `${JSON.stringify(status, null, 2)}\n`);
console.log(`wrote ${path.relative(root, output)}`);
