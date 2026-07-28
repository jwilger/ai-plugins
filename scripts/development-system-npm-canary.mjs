#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import process from "node:process";

const root = path.resolve(import.meta.dirname, "..");
const temporary = fs.mkdtempSync(
  path.join(os.tmpdir(), "development-system-npm-canary-"),
);

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: root,
    encoding: "utf8",
    maxBuffer: 100 * 1024 * 1024,
    ...options,
  });
  if (result.status !== 0)
    throw new Error(
      `development_system.npm_canary_command_failed command=${command} status=${result.status}\n${result.stdout}\n${result.stderr}`,
    );
  return result.stdout;
}

try {
  const packReport = JSON.parse(
    run("npm", [
      "pack",
      "./plugins/development-system",
      "--json",
      "--pack-destination",
      temporary,
    ]),
  )[0];
  if (!packReport?.filename)
    throw new Error("development_system.npm_canary_pack_report_invalid");
  const tarball = path.join(temporary, packReport.filename);
  run("tar", ["-xzf", tarball, "-C", temporary]);
  const extracted = path.join(temporary, "package");
  const evidence = JSON.parse(
    run(process.execPath, [
      "scripts/pi-package-canary.mjs",
      "--package-root",
      extracted,
    ]),
  );
  if (
    evidence.extension?.version !== packReport.version ||
    evidence.package !== "development-system"
  )
    throw new Error("development_system.npm_canary_identity_mismatch");
  process.stdout.write(
    `${JSON.stringify({ ok: true, name: packReport.name, version: packReport.version, packedBytes: packReport.size, skills: evidence.skills.length, extension: evidence.extension.extension })}\n`,
  );
} finally {
  fs.rmSync(temporary, { recursive: true, force: true });
}
