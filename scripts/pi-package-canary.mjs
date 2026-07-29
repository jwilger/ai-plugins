#!/usr/bin/env node
import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import { pathToFileURL } from "node:url";

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

async function inspectPackage(piBinary, installation, cwd, temporaryRoot) {
  const packageHome = path.join(temporaryRoot, "package-home");
  fs.mkdirSync(packageHome, { recursive: true, mode: 0o700 });
  run(piBinary, ["install", installation.source], {
    cwd,
    env: {
      ...process.env,
      ...installation.env,
      PI_CODING_AGENT_DIR: packageHome,
      PI_OFFLINE: "1",
    },
  });
  const packageRoot = installation.resolvePackageRoot(packageHome);
  const manifest = JSON.parse(
    fs.readFileSync(path.join(packageRoot, "package.json"), "utf8"),
  );
  const [expectedExtension] = manifest.pi?.extensions ?? [];
  assert.ok(expectedExtension, "Pi package extension manifest is missing");
  const settings = JSON.parse(
    fs.readFileSync(path.join(packageHome, "settings.json"), "utf8"),
  );
  assert.equal(
    settings.packages?.length,
    1,
    "Pi settings did not preserve one installed package source",
  );
  if (installation.expectedSettingsSource) {
    assert.equal(
      settings.packages[0],
      installation.expectedSettingsSource,
      "Pi settings did not preserve the installed package source identity",
    );
  }
  const sourceEvidence = {
    ...installation.evidenceSource,
    observedSettings:
      !installation.expectedSettingsSource ||
      settings.packages[0] === installation.expectedSettingsSource,
  };
  if (installation.expectedCommit) {
    const checkout = run("git", [
      "-C",
      packageRoot,
      "rev-parse",
      "HEAD",
    ]).trim();
    assert.equal(
      checkout,
      installation.expectedCommit,
      "Pi Git package checkout did not honor the requested commit",
    );
    const generatedLock = JSON.parse(
      fs.readFileSync(path.join(packageRoot, "package-lock.json"), "utf8"),
    );
    assert.equal(
      generatedLock.packages?.[""]?.name,
      manifest.name,
      "Pi Git package npm reconciliation did not produce the root lock entry",
    );
    sourceEvidence.requestedCommit = installation.expectedCommit;
    sourceEvidence.checkout = checkout;
    sourceEvidence.npmReconciled = true;
  }
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
  assert.ok(
    packageCommands.some(
      (command) =>
        command.name === "development-system-worktree-switch" &&
        command.source === "extension",
    ),
    "development-system worktree session-switch command did not load",
  );
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
    fs.realpathSync(path.join(packageRoot, expectedExtension)),
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
    source: sourceEvidence,
    skills: skillNames,
    extension: provenance,
    piVersion: run(piBinary, ["--version"]).trim(),
  };
}

async function main() {
  const clean = process.argv.includes("--clean-checkout");
  const gitSource = process.argv.includes("--git-source");
  const packageRootIndex = process.argv.indexOf("--package-root");
  if (packageRootIndex >= 0 && !process.argv[packageRootIndex + 1])
    throw new Error("--package-root requires a path");
  const explicitPackageRoot =
    packageRootIndex >= 0
      ? path.resolve(process.argv[packageRootIndex + 1])
      : null;
  if (
    [clean, gitSource, Boolean(explicitPackageRoot)].filter(Boolean).length > 1
  )
    throw new Error(
      "--clean-checkout, --git-source, and --package-root are mutually exclusive",
    );
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
    let installation;
    if (gitSource) {
      const remoteRoot = path.join(temporaryRoot, "remotes");
      const remoteParent = path.join(remoteRoot, "jwilger");
      const remoteRepository = path.join(remoteParent, "ai-plugins");
      fs.mkdirSync(remoteParent, { recursive: true });
      run("git", [
        "clone",
        "--quiet",
        "--bare",
        repositoryRoot,
        remoteRepository,
      ]);
      fs.symlinkSync("ai-plugins", `${remoteRepository}.git`);
      const commit = run("git", [
        "-C",
        repositoryRoot,
        "rev-parse",
        "HEAD",
      ]).trim();
      const defaultCommit = run("git", [
        "-C",
        repositoryRoot,
        "rev-parse",
        "HEAD^",
      ]).trim();
      assert.notEqual(defaultCommit, commit);
      run("git", [
        "--git-dir",
        remoteRepository,
        "update-ref",
        "refs/heads/main",
        defaultCommit,
      ]);
      run("git", [
        "--git-dir",
        remoteRepository,
        "update-ref",
        "refs/heads/review-target",
        commit,
      ]);
      run("git", [
        "--git-dir",
        remoteRepository,
        "symbolic-ref",
        "HEAD",
        "refs/heads/main",
      ]);
      const source = `git:github.com/jwilger/ai-plugins@${commit}`;
      const rewriteBase = `${pathToFileURL(remoteRoot).href.replace(/\/$/, "")}/`;
      installation = {
        source,
        evidenceSource: { type: "git", spec: source },
        expectedSettingsSource: source,
        expectedCommit: commit,
        env: {
          GIT_CONFIG_COUNT: "2",
          GIT_CONFIG_KEY_0: `url.${rewriteBase}.insteadOf`,
          GIT_CONFIG_VALUE_0: "https://github.com/",
          GIT_CONFIG_KEY_1: "protocol.file.allow",
          GIT_CONFIG_VALUE_1: "always",
        },
        resolvePackageRoot: (packageHome) =>
          path.join(packageHome, "git/github.com/jwilger/ai-plugins"),
      };
    } else {
      const packageRoot =
        explicitPackageRoot ?? path.join(root, "plugins/development-system");
      installation = {
        source: packageRoot,
        evidenceSource: { type: "local", spec: packageRoot },
        env: {},
        resolvePackageRoot: () => packageRoot,
      };
    }
    const piBinary = path.join(root, "node_modules/.bin/pi");
    const evidence = await inspectPackage(
      piBinary,
      installation,
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
