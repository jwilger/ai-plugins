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

function extensionHarness() {
  const registrations = { commands: new Map(), tools: [], events: new Map() };
  const pi = {
    registerCommand(name, definition) {
      registrations.commands.set(name, definition);
    },
    registerTool(definition) {
      registrations.tools.push(definition);
    },
    on(name, handler) {
      const previous = registrations.events.get(name);
      registrations.events.set(
        name,
        previous
          ? async (event, context) => {
              const first = await previous(event, context);
              return (await handler(event, context)) ?? first;
            }
          : handler,
      );
    },
    getAllTools() {
      return registrations.tools;
    },
    getActiveTools() {
      return registrations.tools.map((tool) => tool.name);
    },
    getCommands() {
      return [...registrations.commands].map(([name]) => ({ name }));
    },
    appendEntry() {},
    sendMessage() {},
  };
  return { pi, registrations };
}

async function loadExtension() {
  return (
    await import(path.join(plugin, "extensions/development-system/index.ts"))
  ).default;
}

const configuredPolicy = (mode, worktrees = true) => `schema_version = 1
[delivery]
mode = "${mode}"
trunk_branch = "main"
[features]
worktrees = ${worktrees}
tiber = true
agentic_systems = false
eval_case_reporting = false
[worktrees]
root = ".worktrees"
[tiber]
max_queued = 5
`;

test("extension registers status command and tool and cleans session state on reload", async () => {
  const { pi, registrations } = extensionHarness();
  (await loadExtension())(pi);
  assert.ok(registrations.commands.has("development-system-status"));
  assert.deepEqual(
    registrations.tools.map((tool) => tool.name),
    [
      "goal_complete",
      "goal_blocked",
      "development_system_status",
      "development_system_setup_preview",
      "development_system_run_review_assignment",
    ],
  );
  assert.ok(registrations.events.has("session_start"));
  assert.ok(registrations.events.has("session_shutdown"));
  const project = fixture();
  const notifications = [];
  const context = {
    cwd: project,
    mode: "tui",
    ui: { notify: (...args) => notifications.push(args), setStatus() {} },
    sessionManager: { getBranch: () => [] },
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

test("setup stops after preview outside TUI and applies exactly once after TUI confirmation", async () => {
  const { pi, registrations } = extensionHarness();
  (await loadExtension())(pi);
  const project = fixture();
  const notifications = [];
  const command = registrations.commands.get(
    "development-system-setup",
  ).handler;
  await command("--delivery pull-request", {
    cwd: project,
    mode: "json",
    ui: {
      notify: (message) => notifications.push(message),
      confirm: async () => true,
    },
  });
  assert.equal(
    fs.existsSync(path.join(project, ".development-system.toml")),
    false,
  );
  assert.ok(
    notifications.some((message) =>
      message.includes("setup_confirmation_required"),
    ),
  );
  await command("--delivery pull-request", {
    cwd: project,
    mode: "tui",
    ui: {
      notify: (message) => notifications.push(message),
      confirm: async () => true,
    },
  });
  assert.equal(
    fs.existsSync(path.join(project, ".development-system.toml")),
    true,
  );
  assert.match(
    fs.readFileSync(path.join(project, ".development-system.toml"), "utf8"),
    /mode = "pull-request"/,
  );
  assert.equal(Number(git(project, "rev-list", "--count", "HEAD")), 2);
});

test("setup rejects a confirmation after bound repository preconditions change", async () => {
  const project = fixture();
  const { createSetupPreview, applySetupPreview } = await import(
    path.join(plugin, "extensions/development-system/adapters/setup.ts")
  );
  const preview = await createSetupPreview(plugin, project, "");
  fs.writeFileSync(path.join(project, "README.md"), "changed\n");
  git(project, "add", "README.md");
  git(project, "commit", "-m", "test: change precondition");
  await assert.rejects(
    () => applySetupPreview(plugin, preview),
    /setup_confirmation_stale/,
  );
  assert.equal(
    fs.existsSync(path.join(project, ".development-system.toml")),
    false,
  );
});

test("extension blocks coordination writes, secret reads, and unclassified shell mutation", async () => {
  const { pi, registrations } = extensionHarness();
  (await loadExtension())(pi);
  const project = fixture();
  fs.writeFileSync(
    path.join(project, ".development-system.toml"),
    configuredPolicy("direct-to-trunk"),
  );
  const context = {
    cwd: project,
    mode: "tui",
    ui: { confirm: async () => false },
  };
  const guard = registrations.events.get("tool_call");
  const write = await guard(
    { toolName: "write", input: { path: "README.md" } },
    context,
  );
  assert.equal(write.block, true);
  assert.match(write.reason, /coordination_write_blocked/);
  const secret = await guard(
    { toolName: "read", input: { path: ".env" } },
    context,
  );
  assert.equal(secret.block, true);
  assert.match(secret.reason, /protected_secret/);
  const shell = await guard(
    { toolName: "bash", input: { command: "python script.py" } },
    context,
  );
  assert.equal(shell.block, true);
  assert.match(shell.reason, /coordination_shell_blocked/);
  assert.equal(
    await guard(
      { toolName: "bash", input: { command: "git status --short" } },
      context,
    ),
    undefined,
  );
});

test("extension enforces delivery mode and case-specific destructive TUI approval", async () => {
  const { pi, registrations } = extensionHarness();
  (await loadExtension())(pi);
  const project = fixture();
  fs.writeFileSync(
    path.join(project, ".development-system.toml"),
    configuredPolicy("local-only", false),
  );
  const guard = registrations.events.get("tool_call");
  const blocked = await guard(
    { toolName: "bash", input: { command: "git push origin main" } },
    { cwd: project, mode: "tui", ui: { confirm: async () => true } },
  );
  assert.equal(blocked.block, true);
  assert.match(blocked.reason, /local_only_publication_blocked/);
  fs.writeFileSync(
    path.join(project, ".development-system.toml"),
    configuredPolicy("direct-to-trunk", false),
  );
  const denied = await guard(
    {
      toolName: "bash",
      input: { command: "git push --force-with-lease origin main" },
    },
    { cwd: project, mode: "json", ui: { confirm: async () => true } },
  );
  assert.match(denied.reason, /destructive_approval_required/);
  const allowed = await guard(
    {
      toolName: "bash",
      input: { command: "git push --force-with-lease origin main" },
    },
    { cwd: project, mode: "tui", ui: { confirm: async () => true } },
  );
  assert.equal(allowed, undefined);
});
