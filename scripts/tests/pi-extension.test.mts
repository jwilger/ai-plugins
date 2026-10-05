// Behavior tests for the pi adapter in plugins/development-system/pi.
// Run with: node --test scripts/tests/pi-extension.test.mts
import assert from "node:assert/strict";
import { accessSync, constants, readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { test } from "node:test";

import developmentSystem, { mcpRegistrations, warningsFrom } from "../../plugins/development-system/pi/extension.ts";
import type {
  ExecOptions,
  ExecResult,
  McpStdioServerConfig,
  NotifyLevel,
  PiContext,
  PiExtensionApi,
} from "../../plugins/development-system/pi/pi-api.ts";

const pluginRoot = resolve(import.meta.dirname, "../../plugins/development-system");

interface ExecCall {
  command: string;
  args: string[];
  options: ExecOptions | undefined;
}

type Exec = (command: string, args: string[], options?: ExecOptions) => Promise<ExecResult>;

function result(stdout: string, code = 0, stderr = ""): ExecResult {
  return { stdout, stderr, code, killed: false };
}

function harness(exec: Exec) {
  const servers = new Map<string, McpStdioServerConfig>();
  const commands = new Map<string, (args: string, ctx: PiContext) => Promise<void>>();
  const sessionStart: Array<(event: { type: "session_start" }, ctx: PiContext) => void> = [];
  const execCalls: ExecCall[] = [];
  const notices: Array<{ message: string; type: NotifyLevel | undefined }> = [];
  const pi: PiExtensionApi = {
    registerMcpServer: (name, config) => void servers.set(name, config),
    on: (_event, handler) => {
      sessionStart.push(handler);
      return () => undefined;
    },
    registerCommand: (name, options) => void commands.set(name, options.handler),
    exec: (command, args, options) => {
      execCalls.push({ command, args, options });
      return exec(command, args, options);
    },
  };
  const ctx: PiContext = {
    cwd: "/work/project",
    ui: { notify: (message, type) => void notices.push({ message, type }) },
  };
  developmentSystem(pi);
  return { servers, commands, sessionStart, execCalls, notices, ctx };
}

const settle = () => new Promise((done) => setImmediate(done));

test("registers every plugin MCP server with its plugin-root launcher", () => {
  const { servers } = harness(async () => result(""));
  const manifest = JSON.parse(readFileSync(join(pluginRoot, "mcp.json"), "utf8"));

  assert.deepEqual([...servers.keys()].sort(), Object.keys(manifest.mcpServers).sort());
  for (const [name, declared] of Object.entries<{ command: string; args: string[] }>(manifest.mcpServers)) {
    const registered = servers.get(name);
    assert.ok(registered);
    assert.equal(registered.command, join(pluginRoot, declared.command));
    accessSync(registered.command, constants.X_OK);
    assert.deepEqual(registered.args, declared.args);
    assert.equal(registered.exposure, "direct");
  }
});

test("rejects MCP declarations the adapter cannot launch faithfully", () => {
  assert.throws(
    () => mcpRegistrations("/plugin", { mcpServers: { remote: { type: "streamable-http", url: "https://x" } } }),
    /development_system\.pi_unsupported_mcp_transport server=remote/,
  );
  assert.throws(
    () => mcpRegistrations("/plugin", { mcpServers: { escape: { type: "stdio", command: "./../outside" } } }),
    /development_system\.pi_mcp_command_escapes_plugin server=escape/,
  );
});

test("session start runs the shared CLI check for pi and surfaces each warning", async () => {
  const { sessionStart, execCalls, notices, ctx } = harness(async () =>
    result(
      [
        "development_system.warning mcp_server_override harness=pi server=tiber",
        "unrelated installer chatter",
        "development_system.warning supply_chain_recommendation harness=pi",
      ].join("\n"),
    ),
  );

  sessionStart.forEach((handler) => handler({ type: "session_start" }, ctx));
  await settle();

  assert.deepEqual(execCalls, [
    {
      command: join(pluginRoot, "bin/development-system"),
      args: ["session-start", "--harness", "pi", "--project", "/work/project"],
      options: { cwd: "/work/project", timeout: 900_000 },
    },
  ]);
  assert.deepEqual(notices, [
    { message: "development-system: mcp_server_override harness=pi server=tiber", type: "warning" },
    { message: "development-system: supply_chain_recommendation harness=pi", type: "warning" },
  ]);
});

test("session start does not wait for binary repair before returning", () => {
  const { sessionStart, ctx } = harness(() => new Promise(() => undefined));

  const returned = sessionStart.map((handler) => handler({ type: "session_start" }, ctx));

  assert.deepEqual(returned, [undefined]);
});

test("a failed session-start check is reported, not swallowed", async () => {
  const { sessionStart, notices, ctx } = harness(async () =>
    result("", 2, "development_system.binary_install_verification_failed\n"),
  );

  sessionStart.forEach((handler) => handler({ type: "session_start" }, ctx));
  await settle();

  assert.deepEqual(notices, [
    { message: "development-system: development_system.binary_install_verification_failed", type: "error" },
  ]);
});

test("/development-system doctor runs the pi conflict check", async () => {
  const { commands, execCalls, notices, ctx } = harness(async () => result(""));

  await commands.get("development-system")?.("doctor", ctx);

  assert.deepEqual(execCalls, [
    {
      command: join(pluginRoot, "bin/development-system"),
      args: ["doctor", "--harness", "pi", "--project", "/work/project"],
      options: { cwd: "/work/project", timeout: 60_000 },
    },
  ]);
  assert.deepEqual(notices, [{ message: "development-system: no conflicts found", type: "info" }]);
});

test("/development-system rejects unknown subcommands without running anything", async () => {
  const { commands, execCalls, notices, ctx } = harness(async () => result(""));

  await commands.get("development-system")?.("explode", ctx);

  assert.deepEqual(execCalls, []);
  assert.deepEqual(notices, [
    { message: "development-system: unknown subcommand 'explode'; usage: /development-system doctor", type: "error" },
  ]);
});

test("warningsFrom keeps only CLI warning lines", () => {
  assert.deepEqual(warningsFrom("development_system.warning a=1\nnoise\n\ndevelopment_system.warning b=2\n"), [
    "a=1",
    "b=2",
  ]);
});
