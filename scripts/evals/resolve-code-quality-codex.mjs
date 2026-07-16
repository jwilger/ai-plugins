#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);

const supportedPlatforms = new Map([
  ["linux:arm64", ["linux-arm64", "aarch64-unknown-linux-musl"]],
  ["linux:x64", ["linux-x64", "x86_64-unknown-linux-musl"]],
]);

function fail(detail) {
  console.error(`code-quality Codex resolution failed: ${detail}`);
  process.exit(2);
}

function readJson(file, label) {
  try {
    return JSON.parse(fs.readFileSync(file, "utf8"));
  } catch {
    fail(`${label} is not valid JSON`);
  }
}

function canonicalRegularFile(file, label, executable = false) {
  try {
    const metadata = fs.lstatSync(file);
    if (
      !metadata.isFile() ||
      fs.realpathSync(file) !== path.resolve(file) ||
      (executable && (metadata.mode & 0o111) === 0)
    ) {
      fail(
        `${label} is not a canonical${executable ? " executable" : ""} file`,
      );
    }
    return path.resolve(file);
  } catch {
    fail(`${label} is unavailable`);
  }
}

const platform = supportedPlatforms.get(`${process.platform}:${process.arch}`);
if (!platform) {
  fail(`unsupported host ${process.platform}/${process.arch}`);
}
const [platformSuffix, expectedTarget] = platform;
const payloadSpecifier = `@openai/codex-${platformSuffix}`;

let wrapperPackageJson;
let payloadPackageJson;
try {
  wrapperPackageJson = require.resolve("@openai/codex/package.json");
  payloadPackageJson = require.resolve(`${payloadSpecifier}/package.json`);
} catch {
  fail(`required package is missing: ${payloadSpecifier}`);
}

wrapperPackageJson = canonicalRegularFile(
  wrapperPackageJson,
  "Codex wrapper package manifest",
);
payloadPackageJson = canonicalRegularFile(
  payloadPackageJson,
  "Codex native package manifest",
);
const wrapperPackage = readJson(wrapperPackageJson, "Codex wrapper manifest");
const payloadPackage = readJson(payloadPackageJson, "Codex native manifest");

if (
  wrapperPackage.name !== "@openai/codex" ||
  !/^[0-9]+\.[0-9]+\.[0-9]+$/.test(wrapperPackage.version ?? "")
) {
  fail("Codex wrapper identity is invalid");
}
if (
  payloadPackage.name !== "@openai/codex" ||
  payloadPackage.version !== `${wrapperPackage.version}-${platformSuffix}`
) {
  fail("Codex native package does not match the wrapper version and host");
}

const runtimeRoot = path.join(
  path.dirname(payloadPackageJson),
  "vendor",
  expectedTarget,
);
const runtimeManifest = canonicalRegularFile(
  path.join(runtimeRoot, "codex-package.json"),
  "Codex runtime manifest",
);
const manifest = readJson(runtimeManifest, "Codex runtime manifest");
if (
  manifest.layoutVersion !== 1 ||
  manifest.version !== wrapperPackage.version ||
  manifest.target !== expectedTarget ||
  manifest.variant !== "codex" ||
  manifest.entrypoint !== "bin/codex" ||
  manifest.resourcesDir !== "codex-resources" ||
  manifest.pathDir !== "codex-path"
) {
  fail("Codex runtime manifest does not match the package contract");
}

const codexBin = canonicalRegularFile(
  path.join(runtimeRoot, manifest.entrypoint),
  "Codex native binary",
  true,
);
const resourceBwrap = canonicalRegularFile(
  path.join(runtimeRoot, manifest.resourcesDir, "bwrap"),
  "Codex packaged bubblewrap",
  true,
);
const resourceRg = canonicalRegularFile(
  path.join(runtimeRoot, manifest.pathDir, "rg"),
  "Codex packaged ripgrep",
  true,
);

process.stdout.write(
  `${JSON.stringify({
    codexBin,
    expectedTarget,
    manifest,
    payloadPackageJson,
    payloadSpecifier,
    payloadVersion: payloadPackage.version,
    platformSuffix,
    resourceBwrap,
    resourceRg,
    runtimeManifest,
    runtimeRoot: path.resolve(runtimeRoot),
    runtimeVersion: manifest.version,
    wrapperPackageJson,
    wrapperVersion: wrapperPackage.version,
  })}\n`,
);
