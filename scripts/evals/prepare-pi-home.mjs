#!/usr/bin/env node
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import process from "node:process";

const root = path.resolve(import.meta.dirname, "../..");
const [destinationArgument, mode] = process.argv.slice(2);
if (
  !destinationArgument ||
  !["no-plugins", "development-system", "full-marketplace"].includes(mode)
) {
  console.error(
    "usage: prepare-pi-home.mjs DEST no-plugins|development-system|full-marketplace",
  );
  process.exit(2);
}
const destination = path.resolve(destinationArgument);
const source = path.resolve(
  process.env.PI_EVAL_SOURCE_HOME ?? path.join(os.homedir(), ".pi/agent"),
);
const relative = path.relative(source, destination);
if (
  relative === "" ||
  (!relative.startsWith("..") && !path.isAbsolute(relative))
)
  throw new Error("Pi eval home must not overlap source auth home");

fs.rmSync(destination, { recursive: true, force: true });
fs.mkdirSync(destination, { recursive: true, mode: 0o700 });
const authSource = JSON.parse(
  fs.readFileSync(path.join(source, "auth.json"), "utf8"),
);
if (!authSource["openai-codex"])
  throw new Error(
    "Pi source home has no openai-codex subscription authentication",
  );
fs.writeFileSync(
  path.join(destination, "auth.json"),
  `${JSON.stringify({ "openai-codex": authSource["openai-codex"] }, null, 2)}\n`,
  { mode: 0o600 },
);

const inventory = JSON.parse(
  fs.readFileSync(path.join(root, ".agents/plugins/pi-support.json"), "utf8"),
);
const selected = mode === "no-plugins" ? [] : inventory.packages;
if (
  mode === "development-system" &&
  (selected.length !== 1 || selected[0].name !== "development-system")
) {
  throw new Error("Pi development-system composition is not unique");
}
const packages = selected.map((entry) => path.resolve(root, entry.path));
fs.writeFileSync(
  path.join(destination, "settings.json"),
  `${JSON.stringify({ packages, defaultProjectTrust: "never", enableInstallTelemetry: false }, null, 2)}\n`,
  { mode: 0o600 },
);
fs.writeFileSync(
  path.join(destination, "composition.json"),
  `${JSON.stringify({ mode, packages: selected.map((entry) => entry.name) }, null, 2)}\n`,
  { mode: 0o600 },
);
console.log(
  JSON.stringify({
    mode,
    packages: selected.map((entry) => entry.name),
    destination,
  }),
);
