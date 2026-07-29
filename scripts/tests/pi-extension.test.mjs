import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { SessionManager } from "@earendil-works/pi-coding-agent";

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

test("status identifies linked checkout and inherits primary-checkout policy", () => {
  const primary = fixture();
  const linked = `${primary}-linked`;
  git(primary, "worktree", "add", "-b", "feature", linked);
  fs.writeFileSync(
    path.join(primary, ".development-system.toml"),
    configuredPolicy("pull-request"),
  );
  const result = status(linked);
  assert.equal(result.status.checkout.kind, "linked");
  assert.equal(result.status.checkout.primary, fs.realpathSync(primary));
  assert.equal(result.status.checkout.current, fs.realpathSync(linked));
  assert.equal(result.status.configured, true);
  assert.equal(result.status.deliveryMode, "pull-request");
  assert.ok(result.status.enabledFeatures.includes("worktrees"));
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
  const registrations = {
    commands: new Map(),
    tools: [],
    events: new Map(),
    userMessages: [],
  };
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
    sendUserMessage(message, options) {
      registrations.userMessages.push({ message, options });
    },
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
      "development_system_goal_status",
      "goal_complete",
      "goal_blocked",
      "development_system_status",
      "development_system_policy_read",
      "development_system_pi_reference",
      "development_system_worktree_list",
      "development_system_worktree_switch",
      "development_system_worktree_finish",
      "development_system_worktree_create",
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

test("semantic worktree tools bootstrap a primary checkout and report session-switch handoff", async () => {
  const { pi, registrations } = extensionHarness();
  (await loadExtension())(pi);
  const project = fixture();
  fs.writeFileSync(
    path.join(project, ".development-system.toml"),
    configuredPolicy("direct-to-trunk"),
  );
  const context = { cwd: project, mode: "tui", hasUI: true };
  const list = registrations.tools.find(
    (tool) => tool.name === "development_system_worktree_list",
  );
  const create = registrations.tools.find(
    (tool) => tool.name === "development_system_worktree_create",
  );

  const before = await list.execute("list", {}, undefined, undefined, context);
  assert.equal(before.details.currentKind, "primary");
  assert.equal(before.details.requiresRelaunch, false);
  assert.equal(before.details.requiresUserWorkspaceSwitch, false);
  assert.match(before.content[0].text, /switchCommand/);

  const created = await create.execute(
    "create",
    { name: "bootstrap", branch: "fix/bootstrap" },
    undefined,
    undefined,
    context,
  );
  assert.equal(created.details.status, "created");
  assert.equal(created.details.requiresRelaunch, false);
  assert.equal(
    created.details.switchCommand,
    "/development-system-worktree-switch fix/bootstrap",
  );
  assert.equal(created.details.requiresUserWorkspaceSwitch, false);
  assert.equal(created.details.switchQueued, true);
  assert.match(created.details.nextAction, /queued/i);
  assert.equal(registrations.userMessages.length, 1);
  assert.match(
    registrations.userMessages[0].message,
    /^\/development-system-worktree-switch --automatic /,
  );
  assert.deepEqual(registrations.userMessages[0].options, {
    deliverAs: "followUp",
  });
  assert.equal(
    git(created.details.path, "branch", "--show-current"),
    "fix/bootstrap",
  );
});

test("local TUI command switches the active Pi conversation into a registered worktree", async () => {
  const { pi, registrations } = extensionHarness();
  (await loadExtension())(pi);
  const project = fixture();
  fs.writeFileSync(
    path.join(project, ".development-system.toml"),
    configuredPolicy("direct-to-trunk", true),
  );
  const linked = path.join(project, ".worktrees", "switch-target");
  git(project, "worktree", "add", "-b", "switch-target", linked);
  const manager = SessionManager.inMemory(project);
  manager.appendMessage({
    role: "user",
    content: "continue the ticket",
    timestamp: Date.now(),
  });
  let switchedPath = "";
  let replacementNotice = "";
  const notifications = [];
  const command = registrations.commands.get(
    "development-system-worktree-switch",
  );
  assert.ok(command);

  await command.handler("switch-target", {
    cwd: project,
    mode: "tui",
    hasUI: true,
    sessionManager: manager,
    waitForIdle: async () => {},
    ui: {
      confirm: async () => true,
      notify: (message) => notifications.push(message),
    },
    async switchSession(sessionPath, options) {
      switchedPath = sessionPath;
      await options.withSession({
        cwd: fs.realpathSync(linked),
        ui: { notify: (message) => (replacementNotice = message) },
      });
      return { cancelled: false };
    },
  });

  const switched = SessionManager.open(switchedPath);
  assert.equal(switched.getCwd(), fs.realpathSync(linked));
  assert.equal(
    switched.buildSessionContext().messages[0].content,
    "continue the ticket",
  );
  assert.match(replacementNotice, /workspace switched/i);
  assert.equal(notifications.length, 0);
});

test("model-callable worktree switch queues command-context replacement without another confirmation", async () => {
  const { pi, registrations } = extensionHarness();
  (await loadExtension())(pi);
  const project = fixture();
  fs.writeFileSync(
    path.join(project, ".development-system.toml"),
    configuredPolicy("direct-to-trunk", true),
  );
  const linked = path.join(project, ".worktrees", "automatic-target");
  git(project, "worktree", "add", "-b", "automatic-target", linked);
  const tool = registrations.tools.find(
    (candidate) => candidate.name === "development_system_worktree_switch",
  );
  const queued = await tool.execute(
    "switch",
    { selector: "automatic-target" },
    undefined,
    undefined,
    { cwd: project, mode: "tui", hasUI: true },
  );
  assert.equal(queued.details.status, "queued");
  assert.equal(queued.details.requiresUserWorkspaceSwitch, false);
  const queuedCommand = registrations.userMessages.at(-1).message;
  assert.match(queuedCommand, /^\/development-system-worktree-switch /);

  let switched = false;
  await registrations.commands
    .get("development-system-worktree-switch")
    .handler(
      queuedCommand.replace(/^\/development-system-worktree-switch /, ""),
      {
        cwd: project,
        mode: "tui",
        hasUI: true,
        sessionManager: SessionManager.inMemory(project),
        waitForIdle: async () => {},
        ui: {
          confirm: async () => {
            throw new Error("automatic switch must not ask for confirmation");
          },
          notify() {},
        },
        async switchSession(sessionPath, options) {
          switched = true;
          await options.withSession({
            cwd: fs.realpathSync(linked),
            ui: { notify() {} },
          });
          return { cancelled: false };
        },
      },
    );
  assert.equal(switched, true);
});

test("finish tool returns to primary and removes the clean worktree without deleting its branch", async () => {
  const { pi, registrations } = extensionHarness();
  (await loadExtension())(pi);
  const project = fixture();
  fs.writeFileSync(
    path.join(project, ".development-system.toml"),
    configuredPolicy("direct-to-trunk", true),
  );
  git(project, "add", ".development-system.toml");
  git(project, "commit", "-m", "test: configure fixture");
  const linked = path.join(project, ".worktrees", "finished-target");
  git(project, "worktree", "add", "-b", "finished-target", linked);
  const tool = registrations.tools.find(
    (candidate) => candidate.name === "development_system_worktree_finish",
  );
  const queued = await tool.execute("finish", {}, undefined, undefined, {
    cwd: linked,
    mode: "tui",
    hasUI: true,
  });
  assert.equal(queued.details.status, "queued");
  assert.equal(queued.details.branchPreserved, true);
  const queuedCommand = registrations.userMessages.at(-1).message;
  assert.match(queuedCommand, /^\/development-system-worktree-finish /);

  let replacementNotice = "";
  await registrations.commands
    .get("development-system-worktree-finish")
    .handler(
      queuedCommand.replace(/^\/development-system-worktree-finish /, ""),
      {
        cwd: linked,
        mode: "tui",
        hasUI: true,
        sessionManager: SessionManager.inMemory(linked),
        waitForIdle: async () => {},
        ui: {
          confirm: async () => {
            throw new Error("automatic finish must not ask for confirmation");
          },
          notify() {},
        },
        async switchSession(sessionPath, options) {
          await options.withSession({
            cwd: fs.realpathSync(project),
            ui: {
              notify(message) {
                replacementNotice += `${message}\n`;
              },
            },
          });
          return { cancelled: false };
        },
      },
    );

  assert.equal(fs.existsSync(linked), false);
  assert.match(replacementNotice, /Removed finished worktree/);
  assert.equal(
    git(project, "branch", "--list", "finished-target", "--format=%(refname)"),
    "refs/heads/finished-target",
  );
});

test("worktree switch command fails closed outside local TUI and on ambiguous selectors", async () => {
  const { pi, registrations } = extensionHarness();
  (await loadExtension())(pi);
  const project = fixture();
  fs.writeFileSync(
    path.join(project, ".development-system.toml"),
    configuredPolicy("direct-to-trunk", true),
  );
  const first = path.join(project, ".worktrees", "same");
  const second = path.join(project, ".worktrees", "other", "same");
  git(project, "worktree", "add", "-b", "first-same", first);
  git(project, "worktree", "add", "-b", "second-same", second);
  const command = registrations.commands.get(
    "development-system-worktree-switch",
  );
  const notifications = [];
  let switched = false;
  const context = {
    cwd: project,
    mode: "json",
    hasUI: false,
    sessionManager: SessionManager.inMemory(project),
    waitForIdle: async () => {},
    ui: { notify: (message) => notifications.push(message) },
    async switchSession() {
      switched = true;
      return { cancelled: false };
    },
  };

  await command.handler("first-same", context);
  assert.equal(switched, false);
  assert.match(notifications.at(-1), /requires.*local Pi TUI/i);

  context.mode = "tui";
  context.hasUI = true;
  context.ui.confirm = async () => true;
  await command.handler("same", context);
  assert.equal(switched, false);
  assert.match(notifications.at(-1), /ambiguous/i);
});

test("worktree switch revalidates selected Git identity after confirmation", async () => {
  const { pi, registrations } = extensionHarness();
  (await loadExtension())(pi);
  const project = fixture();
  fs.writeFileSync(
    path.join(project, ".development-system.toml"),
    configuredPolicy("direct-to-trunk", true),
  );
  const linked = path.join(project, ".worktrees", "race-target");
  git(project, "worktree", "add", "-b", "race-target", linked);
  const notifications = [];
  let switched = false;
  const command = registrations.commands.get(
    "development-system-worktree-switch",
  );

  await command.handler("race-target", {
    cwd: project,
    mode: "tui",
    hasUI: true,
    sessionManager: SessionManager.inMemory(project),
    waitForIdle: async () => {},
    ui: {
      notify: (message) => notifications.push(message),
      confirm: async () => {
        fs.writeFileSync(path.join(linked, "changed.txt"), "changed\n");
        git(linked, "add", "changed.txt");
        git(linked, "commit", "-m", "test: move target");
        return true;
      },
    },
    async switchSession() {
      switched = true;
      return { cancelled: false };
    },
  });

  assert.equal(switched, false);
  assert.match(notifications.at(-1), /identity_changed/);
});

test("authoritative policy tool reads the protected config without opening metadata access", async () => {
  const { pi, registrations } = extensionHarness();
  (await loadExtension())(pi);
  const project = fixture();
  const source = configuredPolicy("pull-request");
  fs.writeFileSync(path.join(project, ".development-system.toml"), source);
  const tool = registrations.tools.find(
    (candidate) => candidate.name === "development_system_policy_read",
  );

  const result = await tool.execute("policy", {}, undefined, undefined, {
    cwd: project,
    mode: "json",
  });

  assert.equal(result.content[0].text, source);
  assert.equal(result.details.policy.delivery.mode, "pull-request");
  assert.equal(
    result.details.path,
    path.join(fs.realpathSync(project), ".development-system.toml"),
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

test("setup preview and apply preserve every unspecified existing policy value", async () => {
  const project = fixture();
  const { createSetupPreview, applySetupPreview } = await import(
    path.join(plugin, "extensions/development-system/adapters/setup.ts")
  );
  const source = `schema_version = 1

[delivery]
mode = "pull-request"
trunk_branch = "stable"

[features]
worktrees = true
tiber = true
agentic_systems = true
eval_case_reporting = true

[worktrees]
root = ".custom-worktrees"

[tiber]
max_queued = 3

[pi.review_models]
strong_reviewer = "custom/reviewer"
`;
  fs.writeFileSync(path.join(project, ".development-system.toml"), source);
  git(project, "add", ".development-system.toml");
  git(project, "commit", "-m", "test: configure policy");

  const preserving = await createSetupPreview(
    plugin,
    project,
    "--enable worktrees",
  );
  assert.equal(preserving.existingConfig, true);
  assert.equal(preserving.proposedConfig, source);
  assert.match(preserving.preview, /delivery pull-request/);
  assert.match(preserving.preview, /agentic_systems=true/);
  assert.match(preserving.preview, /eval_case_reporting=true/);

  const changing = await createSetupPreview(plugin, project, "--disable tiber");
  await applySetupPreview(plugin, changing);
  const updated = fs.readFileSync(
    path.join(project, ".development-system.toml"),
    "utf8",
  );
  assert.match(updated, /mode = "pull-request"/);
  assert.match(updated, /trunk_branch = "stable"/);
  assert.match(updated, /worktrees = true/);
  assert.match(updated, /tiber = false/);
  assert.match(updated, /agentic_systems = true/);
  assert.match(updated, /eval_case_reporting = true/);
  assert.match(updated, /root = "\.custom-worktrees"/);
  assert.match(updated, /max_queued = 3/);
  assert.match(updated, /strong_reviewer = "custom\/reviewer"/);
  assert.equal(Number(git(project, "rev-list", "--count", "HEAD")), 3);
});

test("existing setup update rolls back file and index when its commit fails", async () => {
  const project = fixture();
  const { createSetupPreview, applySetupPreview } = await import(
    path.join(plugin, "extensions/development-system/adapters/setup.ts")
  );
  const source = configuredPolicy("direct-to-trunk");
  fs.writeFileSync(path.join(project, ".development-system.toml"), source);
  git(project, "add", ".development-system.toml");
  git(project, "commit", "-m", "test: configure policy");
  const preview = await createSetupPreview(
    plugin,
    project,
    "--enable agentic-systems",
  );
  const hook = path.join(project, ".git", "hooks", "pre-commit");
  fs.writeFileSync(hook, "#!/usr/bin/env bash\nexit 1\n");
  fs.chmodSync(hook, 0o755);

  await assert.rejects(
    () => applySetupPreview(plugin, preview),
    /setup_commit_failed/,
  );

  assert.equal(
    fs.readFileSync(path.join(project, ".development-system.toml"), "utf8"),
    source,
  );
  assert.equal(git(project, "status", "--porcelain"), "");
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

test("extension allows primary exploration while blocking tracked writes and Git mutation", async () => {
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
  assert.equal(
    await guard(
      { toolName: "read", input: { path: ".development-system.toml" } },
      context,
    ),
    undefined,
  );
  const secret = await guard(
    { toolName: "read", input: { path: ".env" } },
    context,
  );
  assert.equal(secret.block, true);
  assert.match(secret.reason, /protected_secret/);
  const outside = await guard(
    {
      toolName: "read",
      input: { path: "/tmp/installed-pi/docs/extensions.md" },
    },
    context,
  );
  assert.equal(outside.block, true);
  assert.match(outside.reason, /development_system_pi_reference/);
  assert.equal(
    await guard(
      {
        toolName: "bash",
        input: {
          command:
            "git worktree add .worktrees/observable-subagents -b feat/observable-subagents",
        },
      },
      context,
    ),
    undefined,
  );
  const escapedWorktree = await guard(
    {
      toolName: "bash",
      input: {
        command:
          "git worktree add ../observable-subagents -b feat/observable-subagents",
      },
    },
    context,
  );
  assert.equal(escapedWorktree.block, true);
  assert.match(escapedWorktree.reason, /coordination_worktree_target_blocked/);
  const injectedWorktree = await guard(
    {
      toolName: "bash",
      input: {
        command: "git worktree add .worktrees/injected -b --force",
      },
    },
    context,
  );
  assert.equal(injectedWorktree.block, true);
  assert.match(injectedWorktree.reason, /coordination_worktree_target_blocked/);
  assert.equal(
    await guard(
      { toolName: "bash", input: { command: "python script.py --help" } },
      context,
    ),
    undefined,
  );
  assert.equal(
    await guard(
      {
        toolName: "bash",
        input: {
          command:
            "env | sort | grep '^PI_' || true; git status --short --branch; find . -maxdepth 2 -type f | head",
        },
      },
      context,
    ),
    undefined,
  );
  const commit = await guard(
    { toolName: "bash", input: { command: "git commit -m 'wrong checkout'" } },
    context,
  );
  assert.equal(commit.block, true);
  assert.match(commit.reason, /coordination_shell_blocked/);
  assert.equal(
    await guard(
      { toolName: "bash", input: { command: "git status --short" } },
      context,
    ),
    undefined,
  );
  assert.equal(
    await guard(
      {
        toolName: "bash",
        input: { command: "scripts/agent-checkout-guard.sh" },
      },
      context,
    ),
    undefined,
  );
  const linked = path.join(project, ".worktrees", "inspection");
  git(project, "worktree", "add", "-b", "inspection", linked);
  assert.equal(
    await guard(
      {
        toolName: "bash",
        input: { command: `git -C ${linked} status --short --branch` },
      },
      context,
    ),
    undefined,
  );
  assert.equal(
    await guard(
      {
        toolName: "bash",
        input: { command: `cd ${linked} && git status --short --branch` },
      },
      context,
    ),
    undefined,
  );
  const spoofedMutation = await guard(
    {
      toolName: "bash",
      input: { command: `git -C ${linked} branch spoofed-mutation` },
    },
    context,
  );
  assert.equal(spoofedMutation.block, true);
  assert.match(spoofedMutation.reason, /coordination_shell_blocked/);
  const stillPrimary = await guard(
    { toolName: "write", input: { path: "still-primary.txt" } },
    context,
  );
  assert.equal(stillPrimary.block, true);
  assert.match(stillPrimary.reason, /coordination_write_blocked/);
});

test("review assignment tool returns structured cancellation evidence to its parent", async () => {
  const { pi, registrations } = extensionHarness();
  (await loadExtension())(pi);
  const project = fixture();
  fs.writeFileSync(
    path.join(project, ".development-system.toml"),
    configuredPolicy("direct-to-trunk", false),
  );
  const child = path.join(project, "pi-child-fixture");
  fs.writeFileSync(child, "#!/usr/bin/env bash\nsleep 30\n");
  fs.chmodSync(child, 0o755);
  const previous = process.env.DEVELOPMENT_SYSTEM_PI_BIN;
  process.env.DEVELOPMENT_SYSTEM_PI_BIN = child;
  const controller = new AbortController();
  const updates = [];
  setTimeout(() => controller.abort(), 25);
  try {
    const tool = registrations.tools.find(
      (candidate) =>
        candidate.name === "development_system_run_review_assignment",
    );
    const result = await tool.execute(
      "review",
      { assignment: "Inspect the bounded scope", model_role: "bounded-helper" },
      controller.signal,
      (update) => updates.push(update),
      { cwd: project },
    );
    assert.equal(updates.length, 1);
    assert.equal(updates[0].details.state, "starting-fresh-child");
    assert.match(updates[0].content[0].text, /"status":"running"/);
    assert.equal(result.details.status, "failed");
    assert.equal(
      result.details.code,
      "development_system.review_child_cancelled",
    );
    assert.equal(result.details.lifecycle.state, "cancelled");
    assert.equal(result.details.reason, "parent-abort");
    assert.match(result.content[0].text, /terminationRequested/);
  } finally {
    if (previous === undefined) delete process.env.DEVELOPMENT_SYSTEM_PI_BIN;
    else process.env.DEVELOPMENT_SYSTEM_PI_BIN = previous;
  }
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
  for (const command of [
    "git -c color.ui=false push origin main",
    "command git push origin main",
    "git status; git push origin main",
    "X=1 git push --force origin main",
  ]) {
    const wrapped = await guard(
      { toolName: "bash", input: { command } },
      { cwd: project, mode: "tui", ui: { confirm: async () => true } },
    );
    assert.equal(wrapped.block, true);
    assert.match(wrapped.reason, /local_only_publication_blocked/);
  }
  fs.writeFileSync(
    path.join(project, ".development-system.toml"),
    configuredPolicy("direct-to-trunk", true),
  );
  const compoundMutation = await guard(
    {
      toolName: "bash",
      input: {
        command:
          "git -c core.hooksPath=/dev/null commit -am increment; git push origin main",
      },
    },
    { cwd: project, mode: "tui", ui: { confirm: async () => true } },
  );
  assert.equal(compoundMutation.block, true);
  assert.match(compoundMutation.reason, /coordination_shell_blocked/);

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
