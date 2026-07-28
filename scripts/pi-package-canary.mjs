#!/usr/bin/env node
import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import process from "node:process";

const repositoryRoot = path.resolve(import.meta.dirname, "..");
const expectedSkills = JSON.parse(
  fs.readFileSync(
    path.join(repositoryRoot, ".agents/plugins/pi-support.json"),
    "utf8",
  ),
).packages[0].skills;

function run(command, args, options = {}) {
  const result = spawnSync(command, args, { encoding: "utf8", ...options });
  if (result.status !== 0)
    throw new Error(
      `${command} ${args.join(" ")} failed (${result.status}):\n${result.stdout}\n${result.stderr}`,
    );
  return result.stdout;
}

function jsonlRequest(piBinary, cwd, agentDirectory, marker) {
  return new Promise((resolve, reject) => {
    const child = spawn(
      piBinary,
      ["--mode", "rpc", "--no-session", "--no-context-files"],
      {
        cwd,
        env: {
          ...process.env,
          PI_CODING_AGENT_DIR: agentDirectory,
          PI_OFFLINE: "1",
          DEVELOPMENT_SYSTEM_PI_EVAL_MARKER: marker,
        },
        stdio: ["pipe", "pipe", "pipe"],
      },
    );
    let buffer = "";
    let stderr = "";
    const records = [];
    child.stdout.on("data", (chunk) => {
      buffer += chunk.toString("utf8");
      for (;;) {
        const newline = buffer.indexOf("\n");
        if (newline < 0) break;
        const line = buffer.slice(0, newline).replace(/\r$/, "");
        buffer = buffer.slice(newline + 1);
        if (line) records.push(JSON.parse(line));
      }
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk.toString("utf8");
    });
    child.on("error", reject);
    child.on("close", (code) =>
      code === 0
        ? resolve(records)
        : reject(new Error(`Pi RPC failed (${code}): ${stderr}`)),
    );
    child.stdin.end(`${JSON.stringify({ type: "get_commands" })}\n`);
  });
}

async function inspectPackage(piBinary, packageRoot, cwd, temporaryRoot) {
  const packageHome = path.join(temporaryRoot, "package-home");
  fs.mkdirSync(packageHome, { recursive: true, mode: 0o700 });
  run(piBinary, ["install", packageRoot], {
    cwd,
    env: { ...process.env, PI_CODING_AGENT_DIR: packageHome, PI_OFFLINE: "1" },
  });
  const marker = path.join(temporaryRoot, "extension-provenance.json");
  const records = await jsonlRequest(piBinary, cwd, packageHome, marker);
  const response = records.find(
    (record) => record.type === "response" && record.command === "get_commands",
  );
  assert.equal(response?.success, true, "Pi get_commands response missing");
  const packageCommands = response.data.commands.filter(
    (command) =>
      command.sourceInfo?.origin === "package" &&
      command.sourceInfo?.baseDir === fs.realpathSync(packageRoot),
  );
  const skillNames = packageCommands
    .filter((command) => command.source === "skill")
    .map((command) => command.name.replace(/^skill:/, ""))
    .sort();
  assert.deepEqual(
    skillNames,
    [...expectedSkills].sort(),
    "loaded Pi skills differ from support inventory",
  );
  assert.equal(
    new Set(skillNames).size,
    8,
    "Pi package has a public skill collision",
  );
  const extension = packageCommands.find(
    (command) => command.name === "development-system-status",
  );
  assert.ok(extension, "development-system extension command did not load");
  assert.equal(
    packageCommands.filter((command) => command.name === "goal").length,
    1,
    "development-system must supply exactly one unsuffixed goal command",
  );
  const provenance = JSON.parse(fs.readFileSync(marker, "utf8"));
  assert.deepEqual(provenance.reservedPublicNames, [
    "goal",
    "goal_complete",
    "goal_blocked",
  ]);
  assert.equal(provenance.goalCollision, false);
  assert.equal(
    fs.realpathSync(provenance.extension),
    fs.realpathSync(
      path.join(packageRoot, "extensions/development-system/index.ts"),
    ),
  );

  const noPackageHome = path.join(temporaryRoot, "no-package-home");
  fs.mkdirSync(noPackageHome, { recursive: true, mode: 0o700 });
  const absentMarker = path.join(temporaryRoot, "no-package-marker");
  const baselineRecords = await jsonlRequest(
    piBinary,
    cwd,
    noPackageHome,
    absentMarker,
  );
  const baselineResponse = baselineRecords.find(
    (record) => record.type === "response" && record.command === "get_commands",
  );
  assert.ok(
    !baselineResponse.data.commands.some(
      (command) => command.sourceInfo?.baseDir === fs.realpathSync(packageRoot),
    ),
  );
  assert.equal(
    baselineResponse.data.commands.some((command) => command.name === "goal"),
    false,
    "no-package baseline unexpectedly supplies the reserved goal command",
  );
  assert.equal(
    fs.existsSync(absentMarker),
    false,
    "no-package mode executed package extension",
  );
  return {
    skills: skillNames,
    extension: provenance,
    piVersion: run(piBinary, ["--version"]).trim(),
  };
}

async function main() {
  const clean = process.argv.includes("--clean-checkout");
  const temporaryRoot = fs.mkdtempSync(
    path.join(os.tmpdir(), "development-system-pi-canary-"),
  );
  try {
    let root = repositoryRoot;
    if (clean) {
      root = path.join(temporaryRoot, "checkout");
      fs.mkdirSync(root);
      const archive = spawnSync(
        "git",
        ["-C", repositoryRoot, "archive", "HEAD"],
        { encoding: null, maxBuffer: 100 * 1024 * 1024 },
      );
      if (archive.status !== 0)
        throw new Error(archive.stderr.toString("utf8"));
      const unpack = spawnSync("tar", ["-x", "-C", root], {
        input: archive.stdout,
        maxBuffer: 100 * 1024 * 1024,
      });
      if (unpack.status !== 0) throw new Error(unpack.stderr.toString("utf8"));
      run(path.join(root, "scripts/bootstrap-pi-package.sh"), [], {
        cwd: root,
      });
    }
    const packageRoot = path.join(root, "plugins/development-system");
    const piBinary = path.join(root, "node_modules/.bin/pi");
    const evidence = await inspectPackage(
      piBinary,
      packageRoot,
      root,
      temporaryRoot,
    );
    process.stdout.write(
      `${JSON.stringify({ ok: true, package: "development-system", ...evidence })}\n`,
    );
  } finally {
    fs.rmSync(temporaryRoot, { recursive: true, force: true });
  }
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
