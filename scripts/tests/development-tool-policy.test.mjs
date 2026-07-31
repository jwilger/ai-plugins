import assert from "node:assert/strict";
import {
  chmod,
  mkdtemp,
  mkdir,
  readFile,
  rm,
  writeFile,
} from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  inspectToolPolicy,
  requiredToolVersions,
} from "../../plugins/development-system/bin/install-development-tool.mjs";

async function fixture() {
  const root = await mkdtemp(
    path.join(os.tmpdir(), "development-tool-policy-"),
  );
  const home = path.join(root, "home");
  const bin = path.join(root, "bin");
  await mkdir(home);
  await mkdir(bin);
  const manifest = path.join(root, "releases.json");
  await writeFile(
    manifest,
    JSON.stringify({
      schemaVersion: 2,
      tools: {
        bd: {
          version: "1.1.2",
          requiredFor: ["beads"],
          versionCommand: ["version"],
          versionPattern: "\\bbd version (\\d+\\.\\d+\\.\\d+)\\b",
          releases: {
            "x86_64-linux": {
              url: "https://example.invalid/bd.tar.gz",
              sha256: "0".repeat(64),
              binaryPath: "bd",
            },
          },
        },
      },
    }),
  );
  return {
    root,
    home,
    bin,
    environment: {
      HOME: home,
      PATH: bin,
      DEVELOPMENT_SYSTEM_TOOL_RELEASES: manifest,
      DEVELOPMENT_SYSTEM_TOOL_PLATFORM: "linux",
      DEVELOPMENT_SYSTEM_TOOL_ARCH: "x64",
    },
  };
}

async function writeBd(directory, version) {
  const executable = path.join(directory, "bd");
  await writeFile(executable, `#!/bin/sh\nprintf 'bd version ${version}\\n'\n`);
  await chmod(executable, 0o755);
  return executable;
}

test("the release manifest is the sole minimum and target version policy", async (t) => {
  const state = await fixture();
  t.after(() => rm(state.root, { recursive: true, force: true }));

  assert.deepEqual(await requiredToolVersions(state.environment), {
    bd: "1.1.2",
  });
});

test("future manifest-declared binaries use the same feature-scoped policy", async (t) => {
  const state = await fixture();
  t.after(() => rm(state.root, { recursive: true, force: true }));
  const manifestPath = state.environment.DEVELOPMENT_SYSTEM_TOOL_RELEASES;
  const manifest = JSON.parse(await readFile(manifestPath, "utf8"));
  manifest.tools.helper = {
    version: "3.4.5",
    requiredFor: ["agentic-systems"],
    versionCommand: ["--version"],
    versionPattern: "helper (\\d+\\.\\d+\\.\\d+)",
    releases: manifest.tools.bd.releases,
  };
  await writeFile(manifestPath, JSON.stringify(manifest));

  assert.deepEqual(await requiredToolVersions(state.environment), {
    bd: "1.1.2",
    helper: "3.4.5",
  });
  const policy = await inspectToolPolicy(state.environment, [
    "agentic-systems",
  ]);
  assert.deepEqual(
    policy.tools.map(({ name, targetVersion, status }) => ({
      name,
      targetVersion,
      status,
    })),
    [{ name: "helper", targetVersion: "3.4.5", status: "missing" }],
  );
});

test("tool policy distinguishes missing, outdated, and compatible bd", async (t) => {
  const state = await fixture();
  t.after(() => rm(state.root, { recursive: true, force: true }));

  let policy = await inspectToolPolicy(state.environment);
  assert.equal(policy.tools[0].name, "bd");
  assert.equal(policy.tools[0].targetVersion, "1.1.2");
  assert.equal(policy.tools[0].status, "missing");
  assert.equal(policy.tools[0].currentVersion, null);

  await writeBd(state.bin, "1.1.1");
  policy = await inspectToolPolicy(state.environment);
  assert.equal(policy.tools[0].status, "outdated");
  assert.equal(policy.tools[0].currentVersion, "1.1.1");

  await writeBd(state.bin, "1.1.2");
  policy = await inspectToolPolicy(state.environment);
  assert.equal(policy.tools[0].status, "compatible");
  assert.equal(policy.tools[0].currentVersion, "1.1.2");
});

test("a compatible user-global bd is recognized even when inherited PATH omits its directory", async (t) => {
  const state = await fixture();
  t.after(() => rm(state.root, { recursive: true, force: true }));
  const userBin = path.join(state.home, ".local", "bin");
  await mkdir(userBin, { recursive: true });
  await writeBd(userBin, "1.1.2");

  const policy = await inspectToolPolicy(state.environment);
  assert.equal(policy.tools[0].status, "compatible");
  assert.equal(policy.tools[0].source, "user-global");
  assert.equal(policy.inheritedPathIncludesDestination, false);
  assert.match(policy.pathAction, /HOME.*\.local\/bin|\.local\/bin.*PATH/);
});
