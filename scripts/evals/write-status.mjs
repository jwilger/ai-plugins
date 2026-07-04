#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const root = path.resolve(import.meta.dirname, "../..");
const outDir = path.join(root, "evals/out");

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

fs.mkdirSync(outDir, { recursive: true });
fs.writeFileSync(
  path.join(outDir, "status.json"),
  `${JSON.stringify(status, null, 2)}\n`,
);
console.log(`wrote ${path.relative(root, path.join(outDir, "status.json"))}`);
