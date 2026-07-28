#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const root = path.resolve(import.meta.dirname, "..");
const inventory = JSON.parse(
  fs.readFileSync(path.join(root, ".agents/plugins/pi-support.json"), "utf8"),
);
const readmeFile = path.join(root, "plugins/development-system/README.md");
const start = "<!-- pi-support-inventory:start -->";
const end = "<!-- pi-support-inventory:end -->";

function render() {
  const sections = inventory.packages.map((entry) => {
    const skills = entry.skills.map((skill) => `\`${skill}\``).join(", ");
    const components = entry.componentEntrypoints
      .map((component) => `\`${component}\``)
      .join(", ");
    return `- **${entry.name}** (\`${entry.path}\`)\n  - extension: \`${entry.extension}\`\n  - public skills (${entry.skills.length}): ${skills}\n  - bundled component entry points: ${components}`;
  });
  return `${start}\n\n${sections.join("\n")}\n\n${end}`;
}

const source = fs.readFileSync(readmeFile, "utf8");
const pattern = new RegExp(
  `${start.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}[\\s\\S]*?${end.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}`,
);
if (!pattern.test(source))
  throw new Error("development_system.pi_support_doc_markers_missing");
const expected = source.replace(pattern, render());

if (process.argv.includes("--write")) {
  fs.writeFileSync(readmeFile, expected);
  process.stdout.write(
    "development_system.pi_support_docs synchronized=true\n",
  );
} else if (process.argv.includes("--check")) {
  if (expected !== source) {
    process.stderr.write(
      "development-system README Pi inventory is stale; run scripts/generate-pi-support-docs.mjs --write\n",
    );
    process.exit(1);
  }
  process.stdout.write(
    "development_system.pi_support_docs synchronized=true\n",
  );
} else {
  process.stderr.write("usage: generate-pi-support-docs.mjs --check|--write\n");
  process.exit(2);
}
