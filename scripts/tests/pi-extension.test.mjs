import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

const root = path.resolve(import.meta.dirname, "../..");
const plugin = path.join(root, "plugins/development-system");
const cli = path.join(plugin, "bin/development-system-pi");

function git(cwd, ...args) {
  return execFileSync("git", args, { cwd, encoding: "utf8" }).trim();
}

function fixture() {
  const directory = fs.mkdtempSync(
    path.join(os.tmpdir(), "development-system-pi-"),
  );
  git(directory, "init", "--initial-branch=main");
  git(directory, "config", "user.name", "Test User");
  git(directory, "config", "user.email", "test@example.com");
  fs.writeFileSync(path.join(directory, "README.md"), "fixture\n");
  git(directory, "add", "README.md");
  git(directory, "commit", "-m", "test: fixture");
  return directory;
}

function status(project, mode = "json") {
  return JSON.parse(
    execFileSync(cli, ["status", "--project", project, "--mode", mode], {
      encoding: "utf8",
    }),
  );
}

test("headless status reports missing policy without guessing authority", () => {
  const project = fixture();
  const result = status(project, "print");
  assert.equal(result.ok, true);
  assert.equal(result.status.configured, false);
  assert.equal(result.status.deliveryMode, null);
  assert.deepEqual(result.status.enabledFeatures, []);
  assert.equal(result.status.checkout.kind, "primary");
  assert.ok(
    result.status.errors.some(
      (error) => error.code === "development_system.configuration_missing",
    ),
  );
});

test("status parses the authoritative project policy once into semantic values", () => {
  const project = fixture();
  fs.writeFileSync(
    path.join(project, ".development-system.toml"),
    `schema_version = 1
[delivery]
mode = "pull-request"
trunk_branch = "main"
[features]
worktrees = true
tiber = true
agentic_systems = false
eval_case_reporting = true
[worktrees]
root = ".worktrees"
[tiber]
max_queued = 5
`,
  );
  const result = status(project, "json");
  assert.equal(result.status.configured, true);
  assert.equal(result.status.deliveryMode, "pull-request");
  assert.deepEqual(result.status.enabledFeatures, [
    "eval-case-reporting",
    "tiber",
    "worktrees",
  ]);
  assert.equal(result.status.components.tiber.available, true);
  assert.equal(
    result.status.components["development-discipline"].available,
    true,
  );
  assert.equal(result.status.enforcement.mode, "json");
  assert.equal(result.status.enforcement.trustedApproval, false);
});

test("status identifies linked checkout and the primary checkout", () => {
  const primary = fixture();
  const linked = `${primary}-linked`;
  git(primary, "worktree", "add", "-b", "feature", linked);
  const result = status(linked);
  assert.equal(result.status.checkout.kind, "linked");
  assert.equal(result.status.checkout.primary, fs.realpathSync(primary));
  assert.equal(result.status.checkout.current, fs.realpathSync(linked));
});

test("invalid configuration returns a typed actionable error", () => {
  const project = fixture();
  fs.writeFileSync(
    path.join(project, ".development-system.toml"),
    'schema_version = 1\n[delivery]\nmode = "invented"\n',
  );
  let error;
  try {
    execFileSync(cli, ["status", "--project", project], {
      encoding: "utf8",
      stdio: "pipe",
    });
  } catch (caught) {
    error = caught;
  }
  assert.equal(error.status, 2);
  const result = JSON.parse(error.stdout);
  assert.equal(result.ok, false);
  assert.equal(result.error.code, "development_system.configuration_invalid");
  assert.match(result.error.nextAction, /setup|configuration/i);
});

test("extension registers status command and tool and cleans session state on reload", async () => {
  const registrations = { commands: new Map(), tools: [], events: new Map() };
  const pi = {
    registerCommand(name, definition) {
      registrations.commands.set(name, definition);
    },
    registerTool(definition) {
      registrations.tools.push(definition);
    },
    on(name, handler) {
      registrations.events.set(name, handler);
    },
  };
  const { default: extension } = await import(
    path.join(plugin, "extensions/development-system/index.ts")
  );
  extension(pi);
  assert.ok(registrations.commands.has("development-system-status"));
  assert.deepEqual(
    registrations.tools.map((tool) => tool.name),
    ["development_system_status"],
  );
  assert.ok(registrations.events.has("session_start"));
  assert.ok(registrations.events.has("session_shutdown"));

  const project = fixture();
  const notifications = [];
  const context = {
    cwd: project,
    mode: "tui",
    ui: { notify: (...args) => notifications.push(args), setStatus() {} },
  };
  await registrations.events.get("session_start")(
    { reason: "startup" },
    context,
  );
  assert.ok(
    notifications.some(([message]) =>
      message.includes("configuration_missing"),
    ),
  );
  await registrations.events.get("session_shutdown")(
    { reason: "reload" },
    context,
  );
});
