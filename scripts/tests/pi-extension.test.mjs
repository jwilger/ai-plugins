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

test("status identifies linked checkout and inherits primary-checkout policy", () => {
  const primary = fixture();
  const linked = `${primary}-linked`;
  git(primary, "worktree", "add", "-b", "feature", linked);
  fs.writeFileSync(
    path.join(primary, ".development-system.toml"),
    configuredPolicy("pull-request"),
  );
  const nested = path.join(linked, "nested", "package");
  fs.mkdirSync(nested, { recursive: true });
  const result = status(nested);
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
    entries: [],
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
    appendEntry(customType, data) {
      registrations.entries.push({ type: "custom", customType, data });
    },
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

test("semantic worktree create activates a logical workspace without chat or relaunch", async () => {
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
    hasUI: true,
    ui: { setStatus() {}, confirm: async () => false },
  };
  const list = registrations.tools.find(
    (tool) => tool.name === "development_system_worktree_list",
  );
  const create = registrations.tools.find(
    (tool) => tool.name === "development_system_worktree_create",
  );

  const created = await create.execute(
    "create",
    { name: "bootstrap", branch: "fix/bootstrap" },
    undefined,
    undefined,
    context,
  );
  assert.equal(created.details.status, "created");
  assert.equal(created.details.requiresUserWorkspaceSwitch, false);
  assert.equal(
    created.details.logicalWorkspace,
    fs.realpathSync(created.details.path),
  );
  assert.match(created.details.nextAction, /no relaunch/i);
  assert.equal(registrations.userMessages.length, 0);
  assert.ok(
    registrations.entries.some(
      (entry) =>
        entry.customType === "development-system-logical-workspace-state" &&
        entry.data.path === fs.realpathSync(created.details.path),
    ),
  );

  const after = await list.execute("list", {}, undefined, undefined, context);
  assert.equal(after.details.currentKind, "linked");
  assert.equal(
    after.details.logicalWorkspace,
    fs.realpathSync(created.details.path),
  );
  assert.equal(after.details.hostCwd, project);
});

test("logical workspace routes every built-in path and shell call independently", async () => {
  const { pi, registrations } = extensionHarness();
  (await loadExtension())(pi);
  const project = fixture();
  fs.writeFileSync(
    path.join(project, ".development-system.toml"),
    configuredPolicy("direct-to-trunk", true),
  );
  const linked = path.join(project, ".worktrees", "routed");
  git(project, "worktree", "add", "-b", "routed", linked);
  const switchTool = registrations.tools.find(
    (tool) => tool.name === "development_system_worktree_switch",
  );
  const context = {
    cwd: project,
    mode: "json",
    hasUI: false,
    ui: { setStatus() {}, confirm: async () => false },
  };
  const switched = await switchTool.execute(
    "switch",
    { selector: "routed" },
    undefined,
    undefined,
    context,
  );
  assert.equal(switched.details.status, "active");
  assert.equal(registrations.userMessages.length, 0);

  const guard = registrations.events.get("tool_call");
  for (const toolName of ["read", "write", "edit", "grep", "find", "ls"]) {
    const event = {
      toolName,
      input:
        toolName === "grep" || toolName === "find" || toolName === "ls"
          ? {}
          : { path: "nested/file.txt" },
    };
    const decision = await guard(event, context);
    assert.equal(decision, undefined, `${toolName} should be routed`);
    assert.equal(
      event.input.path,
      toolName === "grep" || toolName === "find" || toolName === "ls"
        ? fs.realpathSync(linked)
        : path.join(fs.realpathSync(linked), "nested", "file.txt"),
    );
  }

  const bash = { toolName: "bash", input: { command: "pwd" } };
  assert.equal(await guard(bash, context), undefined);
  assert.match(bash.input.command, /^cd -- '.*\/\.worktrees\/routed' && pwd$/);

  const outside = {
    toolName: "read",
    input: { path: path.join(project, "README.md") },
  };
  const blocked = await guard(outside, context);
  assert.equal(blocked.block, true);
  assert.match(blocked.reason, /path_outside_blocked/);
});

test("user bash fails closed when the logical workspace disappears", async () => {
  const { pi, registrations } = extensionHarness();
  (await loadExtension())(pi);
  const project = fixture();
  fs.writeFileSync(
    path.join(project, ".development-system.toml"),
    configuredPolicy("direct-to-trunk", true),
  );
  const linked = path.join(project, ".worktrees", "removed-routing");
  git(project, "worktree", "add", "-b", "removed-routing", linked);
  const switchTool = registrations.tools.find(
    (tool) => tool.name === "development_system_worktree_switch",
  );
  const context = {
    cwd: project,
    mode: "tui",
    hasUI: true,
    ui: { setStatus() {}, confirm: async () => false },
  };
  await switchTool.execute(
    "switch",
    { selector: "removed-routing" },
    undefined,
    undefined,
    context,
  );
  git(project, "worktree", "remove", "--force", linked);

  const result = await registrations.events.get("user_bash")(
    { command: "touch must-not-run", cwd: project, excludeFromContext: false },
    context,
  );
  assert.equal(result.result.exitCode, 2);
  assert.match(result.result.output, /logical_workspace_user_bash_failed/);
  assert.equal(fs.existsSync(path.join(project, "must-not-run")), false);
});

test("worktree switch command works in headless mode without replacing the Pi session", async () => {
  const { pi, registrations } = extensionHarness();
  (await loadExtension())(pi);
  const project = fixture();
  fs.writeFileSync(
    path.join(project, ".development-system.toml"),
    configuredPolicy("direct-to-trunk", true),
  );
  const linked = path.join(project, ".worktrees", "command-target");
  git(project, "worktree", "add", "-b", "command-target", linked);
  const notifications = [];
  await registrations.commands
    .get("development-system-worktree-switch")
    .handler("command-target", {
      cwd: project,
      mode: "json",
      hasUI: false,
      waitForIdle: async () => {},
      ui: {
        notify: (message) => notifications.push(message),
        setStatus() {},
      },
    });
  assert.match(notifications.at(-1), /Logical workspace is now/);
  assert.equal(registrations.userMessages.length, 0);
  const statusTool = registrations.tools.find(
    (tool) => tool.name === "development_system_status",
  );
  const result = await statusTool.execute("status", {}, undefined, undefined, {
    cwd: project,
    mode: "json",
  });
  assert.equal(result.details.checkout.kind, "linked");
  assert.equal(result.details.logicalWorkspace, fs.realpathSync(linked));
  assert.equal(result.details.hostCwd, project);
});

test("worktree lifecycle tools reject sibling parallel operations", async () => {
  const { pi, registrations } = extensionHarness();
  (await loadExtension())(pi);
  const switchTool = registrations.tools.find(
    (tool) => tool.name === "development_system_worktree_switch",
  );
  const context = {
    cwd: fixture(),
    mode: "json",
    sessionManager: {
      getBranch: () => [
        {
          type: "message",
          message: {
            role: "assistant",
            content: [
              {
                type: "toolCall",
                name: "development_system_worktree_switch",
              },
              { type: "toolCall", name: "write" },
            ],
          },
        },
      ],
    },
  };

  await assert.rejects(
    () =>
      switchTool.execute(
        "switch",
        { selector: "anything" },
        undefined,
        undefined,
        context,
      ),
    /worktree_lifecycle_tool_must_be_isolated/,
  );
});

test("finish routes to primary before removing a clean worktree and preserves its branch", async () => {
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
  const switchTool = registrations.tools.find(
    (tool) => tool.name === "development_system_worktree_switch",
  );
  const finishTool = registrations.tools.find(
    (tool) => tool.name === "development_system_worktree_finish",
  );
  const context = {
    cwd: project,
    mode: "json",
    hasUI: false,
    ui: { setStatus() {} },
  };
  await switchTool.execute(
    "switch",
    { selector: "finished-target" },
    undefined,
    undefined,
    context,
  );
  const finished = await finishTool.execute(
    "finish",
    {},
    undefined,
    undefined,
    context,
  );

  assert.equal(finished.details.status, "removed");
  assert.equal(finished.details.logicalWorkspace, fs.realpathSync(project));
  assert.equal(finished.details.branchPreserved, true);
  assert.equal(fs.existsSync(linked), false);
  assert.equal(registrations.userMessages.length, 0);
  assert.equal(
    git(project, "branch", "--list", "finished-target", "--format=%(refname)"),
    "refs/heads/finished-target",
  );
});

test("finish preserves a worktree that still owns Pi's host cwd", async () => {
  const { pi, registrations } = extensionHarness();
  (await loadExtension())(pi);
  const project = fixture();
  fs.writeFileSync(
    path.join(project, ".development-system.toml"),
    configuredPolicy("direct-to-trunk", true),
  );
  git(project, "add", ".development-system.toml");
  git(project, "commit", "-m", "test: configure fixture");
  const linked = path.join(project, ".worktrees", "host-target");
  git(project, "worktree", "add", "-b", "host-target", linked);
  const finishTool = registrations.tools.find(
    (tool) => tool.name === "development_system_worktree_finish",
  );
  const context = {
    cwd: linked,
    mode: "json",
    hasUI: false,
    sessionManager: { getBranch: () => [] },
    ui: { setStatus() {}, notify() {} },
  };
  await registrations.events.get("session_start")(
    { reason: "startup" },
    context,
  );

  await assert.rejects(
    () => finishTool.execute("finish", {}, undefined, undefined, context),
    /worktree_finish_host_checkout_migration_required/,
  );
  assert.equal(fs.existsSync(linked), true);
});

test("worktree switch rejects ambiguous selectors and stale detached identity", async () => {
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
  const context = {
    cwd: project,
    mode: "tui",
    hasUI: true,
    waitForIdle: async () => {},
    ui: {
      notify: (message) => notifications.push(message),
      setStatus() {},
      confirm: async () => true,
    },
  };

  await command.handler("same", context);
  assert.match(notifications.at(-1), /ambiguous/i);

  context.ui.confirm = async () => {
    git(first, "checkout", "--detach");
    return true;
  };
  await command.handler("first-same", context);
  assert.match(notifications.at(-1), /detached_head/);
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
    assert.ok(updates.length >= 1);
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

test("review assignment tool streams redacted child progress to its parent", async () => {
  const { pi, registrations } = extensionHarness();
  (await loadExtension())(pi);
  const project = fixture();
  fs.writeFileSync(
    path.join(project, ".development-system.toml"),
    configuredPolicy("direct-to-trunk", false),
  );
  const child = path.join(project, "pi-child-fixture");
  const events = [
    { type: "agent_start" },
    {
      type: "tool_execution_start",
      toolCallId: "tool-1",
      toolName: "read",
      args: { path: "/private/client/secret.txt" },
    },
    {
      type: "message_end",
      message: {
        role: "assistant",
        content: [{ type: "text", text: '{"status":"clean"}' }],
        stopReason: "stop",
      },
    },
    { type: "agent_settled" },
  ];
  fs.writeFileSync(
    child,
    `#!/usr/bin/env bash\n${events
      .map(
        (event, index) =>
          `printf '%s\\n' '${JSON.stringify(event)}'${index === 1 ? "\nsleep 0.3" : ""}`,
      )
      .join("\n")}\n`,
  );
  fs.chmodSync(child, 0o755);
  const previous = process.env.DEVELOPMENT_SYSTEM_PI_BIN;
  process.env.DEVELOPMENT_SYSTEM_PI_BIN = child;
  const updates = [];
  try {
    const tool = registrations.tools.find(
      (candidate) =>
        candidate.name === "development_system_run_review_assignment",
    );
    const result = await tool.execute(
      "review",
      { assignment: "Inspect the bounded scope", model_role: "bounded-helper" },
      undefined,
      (update) => updates.push(update),
      { cwd: project, mode: "json" },
    );
    assert.equal(result.details.status, "completed");
    assert.ok(
      updates.some((update) => update.details.state === "tool-running"),
    );
    assert.ok(updates.some((update) => update.details.currentTool === "read"));
    const observable = JSON.stringify(updates);
    assert.equal(observable.includes("/private/client/secret.txt"), false);
    assert.equal(observable.includes("Inspect the bounded scope"), false);
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
  const linked = `${project}-delivery-linked`;
  git(project, "worktree", "add", "-b", "feat/delivery", linked);
  const directFromLinked = await guard(
    {
      toolName: "bash",
      input: { command: "git push origin HEAD:main" },
    },
    { cwd: linked, mode: "tui", ui: { confirm: async () => true } },
  );
  assert.equal(directFromLinked, undefined);
  const wrongTargetFromLinked = await guard(
    {
      toolName: "bash",
      input: { command: "git push origin HEAD:other" },
    },
    { cwd: linked, mode: "tui", ui: { confirm: async () => true } },
  );
  assert.match(wrongTargetFromLinked.reason, /direct_trunk_branch_required/);
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
