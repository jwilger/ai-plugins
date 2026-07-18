#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

function fail(code) {
  process.stderr.write(`code-quality-auth:${code}\n`);
  process.exit(2);
}

function canonicalRegularFile(file, code) {
  const stat = fs.lstatSync(file, { throwIfNoEntry: false });
  if (
    !stat ||
    !stat.isFile() ||
    stat.isSymbolicLink() ||
    fs.realpathSync(file) !== path.resolve(file)
  ) {
    fail(code);
  }
  return stat;
}

if (process.argv.length !== 4) fail("invalid-arguments");

const removeMode = process.argv[2] === "--remove";
const authFile = removeMode ? undefined : path.resolve(process.argv[2]);
const runtimeManifestFile = path.resolve(process.argv[3]);
canonicalRegularFile(runtimeManifestFile, "runtime-manifest-invalid");

let authBytes;
let auth;
let runtimeManifest;
try {
  runtimeManifest = JSON.parse(fs.readFileSync(runtimeManifestFile, "utf8"));
} catch {
  fail("input-invalid");
}
if (!removeMode) {
  const authStat = canonicalRegularFile(authFile, "auth-source-invalid");
  if (
    (typeof process.getuid === "function" && authStat.uid !== process.getuid()) ||
    (authStat.mode & 0o077) !== 0
  ) {
    fail("auth-source-not-private");
  }
  try {
    authBytes = fs.readFileSync(authFile);
    auth = JSON.parse(authBytes.toString("utf8"));
  } catch {
    fail("input-invalid");
  }
  if (
    auth?.auth_mode !== "chatgpt" ||
    !auth.tokens ||
    typeof auth.tokens !== "object" ||
    Array.isArray(auth.tokens)
  ) {
    fail("auth-source-not-chatgpt");
  }
}
if (!Array.isArray(runtimeManifest?.rows) || runtimeManifest.rows.length === 0) {
  fail("runtime-manifest-invalid");
}

const destinations = [];
const uniqueDestinations = new Set();
for (const row of runtimeManifest.rows) {
  if (typeof row?.codexHome !== "string" || !path.isAbsolute(row.codexHome)) {
    fail("runtime-manifest-invalid");
  }
  const codexHome = path.resolve(row.codexHome);
  const homeStat = fs.lstatSync(codexHome, { throwIfNoEntry: false });
  if (
    !homeStat?.isDirectory() ||
    homeStat.isSymbolicLink() ||
    fs.realpathSync(codexHome) !== codexHome ||
    (homeStat.mode & 0o077) !== 0
  ) {
    fail("codex-home-invalid");
  }
  try {
    if (
      fs.readFileSync(path.join(codexHome, ".ai-plugins-eval-home"), "utf8") !==
      "ai-plugins Codex eval home\n"
    ) {
      fail("codex-home-invalid");
    }
  } catch {
    fail("codex-home-invalid");
  }
  const destination = path.join(codexHome, "auth.json");
  if (uniqueDestinations.has(destination)) {
    fail("auth-destination-exists");
  }
  const destinationStat = fs.lstatSync(destination, { throwIfNoEntry: false });
  if (!removeMode && destinationStat) fail("auth-destination-exists");
  if (
    removeMode &&
    (!destinationStat?.isFile() ||
      destinationStat.isSymbolicLink() ||
      destinationStat.uid !== homeStat.uid ||
      (destinationStat.mode & 0o077) !== 0)
  ) {
    fail("auth-destination-invalid");
  }
  uniqueDestinations.add(destination);
  destinations.push(destination);
}

if (removeMode) {
  for (const destination of destinations) fs.unlinkSync(destination);
} else {
  fs.writeFileSync(destinations[0], authBytes, { flag: "wx", mode: 0o600 });
  for (const destination of destinations.slice(1)) {
    fs.linkSync(destinations[0], destination);
  }
}

process.stdout.write(
  removeMode ? "code-quality-auth:removed\n" : "code-quality-auth:prepared\n",
);
