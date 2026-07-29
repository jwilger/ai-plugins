#!/usr/bin/env node
import assert from "node:assert/strict";
import { spawn, execFileSync } from "node:child_process";
import { createHash, randomUUID } from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";

const root = path.resolve(import.meta.dirname, "../..");
const extension = path.join(
  root,
  "plugins/development-system/extensions/development-system/index.ts",
);
const piExecutable = path.join(root, "node_modules/.bin/pi");
const liveTool = process.argv.includes("--live-tool");
const model = process.env.PI_EVAL_MODEL ?? "gpt-5.6-terra";
const thinking = process.env.PI_EVAL_REASONING_EFFORT ?? "medium";
const sourcePiHome = path.resolve(
  process.env.PI_EVAL_SOURCE_HOME ?? path.join(os.homedir(), ".pi/agent"),
);
const sourceAuthPath = path.join(sourcePiHome, "auth.json");

function digest(value) {
  return createHash("sha256").update(value).digest("hex");
}

function secretStrings(value) {
  if (typeof value === "string") return value.length >= 16 ? [value] : [];
  if (Array.isArray(value)) return value.flatMap(secretStrings);
  if (!value || typeof value !== "object") return [];
  return Object.values(value).flatMap(secretStrings);
}

function git(cwd, ...args) {
  return execFileSync("git", args, { cwd, encoding: "utf8" }).trim();
}

function shellQuote(value) {
  return `'${value.replaceAll("'", `'\"'\"'`)}'`;
}

async function waitFor(readValue, predicate, description, timeoutMs = 15_000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (predicate(readValue())) return;
    await delay(50);
  }
  throw new Error(`development_system.tui_scenario_timeout ${description}`);
}

async function stopTui(child) {
  if (child.exitCode === null) {
    try {
      child.stdin.write("\u0004");
    } catch {
      // The PTY may already have closed while its process group remains alive.
    }
    await Promise.race([
      new Promise((resolve) => child.once("close", resolve)),
      delay(2_000),
    ]);
  }
  const groupAlive = () => {
    try {
      process.kill(-child.pid, 0);
      return true;
    } catch (error) {
      if (error.code === "ESRCH") return false;
      throw error;
    }
  };
  if (groupAlive()) {
    process.kill(-child.pid, "SIGTERM");
    await delay(500);
  }
  if (groupAlive()) {
    process.kill(-child.pid, "SIGKILL");
    await delay(500);
  }
  if (groupAlive())
    throw new Error("development_system.tui_scenario_process_group_alive");
}

function sessionEntries(rootDirectory) {
  return fs
    .readdirSync(rootDirectory, { recursive: true })
    .filter((entry) => String(entry).endsWith(".jsonl"))
    .flatMap((entry) =>
      fs
        .readFileSync(path.join(rootDirectory, entry), "utf8")
        .split("\n")
        .filter(Boolean)
        .map((line) => JSON.parse(line)),
    );
}

function launchTui({
  repository,
  agentDirectory,
  sessionDirectory,
  session,
  sessionId,
  initialPrompt,
}) {
  const sessionArgument = session
    ? ` --session ${shellQuote(session)}`
    : ` --session-id ${shellQuote(sessionId)}`;
  const modelArguments = initialPrompt
    ? ` --approve --exclude-tools bash --provider openai-codex --model ${shellQuote(model)} --thinking ${shellQuote(thinking)} ${shellQuote(initialPrompt)}`
    : "";
  const command =
    `cd -- ${shellQuote(repository)} && exec ${shellQuote(piExecutable)} ` +
    "--offline --no-extensions --no-skills --no-context-files " +
    `--session-dir ${shellQuote(sessionDirectory)} ` +
    `--extension ${shellQuote(extension)}${sessionArgument}${modelArguments}`;
  const child = spawn("script", ["-qec", command, "/dev/null"], {
    cwd: repository,
    env: { ...process.env, PI_CODING_AGENT_DIR: agentDirectory },
    detached: true,
    stdio: ["pipe", "pipe", "pipe"],
  });
  let output = "";
  const collect = (chunk) => {
    output += chunk.toString("utf8");
    if (Buffer.byteLength(output) > 2 * 1024 * 1024) child.kill("SIGTERM");
  };
  child.stdout.on("data", collect);
  child.stderr.on("data", collect);
  return { child, output: () => output };
}

const temporary = fs.mkdtempSync(
  path.join(os.tmpdir(), "development-system-worktree-tui-"),
);
const repository = path.join(temporary, "repo");
const agentDirectory = path.join(temporary, "pi-agent");
const sessionDirectory = path.join(temporary, "sessions");
fs.mkdirSync(repository, { recursive: true });
fs.mkdirSync(agentDirectory, { recursive: true, mode: 0o700 });
fs.mkdirSync(sessionDirectory, { recursive: true, mode: 0o700 });

const processes = [];
let sourceAuthBefore = null;
let exactAuthSecrets = [];
let liveOutput = () => "";
try {
  assert.ok(fs.existsSync(piExecutable), `pinned Pi missing: ${piExecutable}`);
  if (liveTool) {
    sourceAuthBefore = fs.readFileSync(sourceAuthPath);
    const sourceAuth = JSON.parse(sourceAuthBefore.toString("utf8"));
    exactAuthSecrets = secretStrings(sourceAuth["openai-codex"]);
    execFileSync(
      process.execPath,
      [
        path.join(root, "scripts/evals/prepare-pi-home.mjs"),
        agentDirectory,
        "no-plugins",
      ],
      { cwd: root, stdio: "inherit" },
    );
  }
  git(repository, "init", "--initial-branch=main");
  git(repository, "config", "user.name", "TUI Scenario");
  git(repository, "config", "user.email", "tui@example.invalid");
  git(repository, "config", "commit.gpgSign", "false");
  fs.writeFileSync(path.join(repository, "README.md"), "fixture\n");
  fs.writeFileSync(
    path.join(repository, ".development-system.toml"),
    `schema_version = 1
[delivery]
mode = "direct-to-trunk"
trunk_branch = "main"
[features]
worktrees = true
tiber = false
agentic_systems = false
eval_case_reporting = false
[worktrees]
root = ".worktrees"
[tiber]
max_queued = 5
`,
  );
  git(repository, "add", ".");
  git(repository, "commit", "-m", "test: fixture");
  const createdWorktree = path.join(repository, ".worktrees", "tui-target");
  const switchedWorktree = path.join(repository, ".worktrees", "tui-switched");
  git(repository, "worktree", "add", "-b", "tui-switched", switchedWorktree);
  const createdProof = path.join(createdWorktree, "logical-proof.txt");
  const switchedProof = path.join(switchedWorktree, "logical-proof.txt");

  const sessionId = randomUUID();
  const initialPrompt = liveTool
    ? "Call development_system_worktree_create with name tui-target and branch tui-target as the only tool call in your first tool-using response. After that tool result, use the write tool in a later response with relative path logical-proof.txt and exact content logical-workspace. Do not batch create with write. Do not use bash or raw Git, and do not finish the worktree. End your final response with LIVE_TOOL_COMPLETE."
    : undefined;
  const tui = launchTui({
    repository,
    agentDirectory,
    sessionDirectory,
    sessionId,
    initialPrompt,
  });
  processes.push(tui.child);
  liveOutput = tui.output;
  await waitFor(
    tui.output,
    (output) => output.includes("component_tools_active"),
    "first-start",
  );
  let semanticCreateObserved = false;
  if (liveTool) {
    await waitFor(
      tui.output,
      (output) =>
        fs.existsSync(createdProof) && output.includes("LIVE_TOOL_COMPLETE"),
      "live-semantic-create-and-write",
      10 * 60_000,
    );
    const entries = sessionEntries(temporary);
    const toolCalls = entries.flatMap((entry) =>
      entry.type === "message" &&
      entry.message?.role === "assistant" &&
      Array.isArray(entry.message.content)
        ? entry.message.content.filter((item) => item?.type === "toolCall")
        : [],
    );
    semanticCreateObserved = toolCalls.some(
      (call) => call.name === "development_system_worktree_create",
    );
    assert.equal(semanticCreateObserved, true);
    assert.equal(
      toolCalls.some((call) => call.name === "bash"),
      false,
    );
    assert.ok(
      entries.some(
        (entry) =>
          entry.type === "message" &&
          entry.message?.role === "toolResult" &&
          entry.message.toolName === "development_system_worktree_create" &&
          entry.message.isError !== true,
      ),
    );
  } else {
    tui.child.stdin.write(
      "/development-system-worktree-create tui-target tui-target\r",
    );
    await waitFor(
      tui.output,
      (output) => output.includes("Create and activate logical workspace?"),
      "create-confirmation",
    );
    tui.child.stdin.write("\r");
    await waitFor(
      tui.output,
      (output) => output.includes("Created and activated logical workspace"),
      "logical-activation",
    );
    tui.child.stdin.write("!printf logical-workspace > logical-proof.txt\r");
    await waitFor(
      () => createdProof,
      (candidate) => fs.existsSync(candidate),
      "routed-user-bash-after-create",
    );
  }
  assert.equal(fs.readFileSync(createdProof, "utf8"), "logical-workspace");
  assert.equal(
    fs.existsSync(path.join(repository, "logical-proof.txt")),
    false,
  );
  if (liveTool) await delay(1_000);

  fs.rmSync(createdProof);
  if (liveTool) {
    tui.child.stdin.write(
      "Call development_system_worktree_switch with selector tui-switched. Do not use bash. End your final response with LIVE_SWITCH_COMPLETE.\r",
    );
    await waitFor(
      tui.output,
      (output) =>
        output.includes("LIVE_SWITCH_COMPLETE") &&
        output.includes("workspace: tui-switched"),
      "live-semantic-switch",
      10 * 60_000,
    );
    await delay(1_000);
  } else {
    const switchNoticeCount = tui
      .output()
      .split("Logical workspace is now").length;
    tui.child.stdin.write("/development-system-worktree-switch tui-switched\r");
    await waitFor(
      tui.output,
      (output) => output.includes("Switch logical workspace?"),
      "switch-confirmation",
    );
    tui.child.stdin.write("\r");
    await waitFor(
      tui.output,
      (output) =>
        output.split("Logical workspace is now").length > switchNoticeCount,
      "logical-switch",
    );
  }
  tui.child.stdin.write("!printf logical-workspace > logical-proof.txt\r");
  await waitFor(
    () => switchedProof,
    (candidate) => fs.existsSync(candidate),
    "routed-user-bash-after-switch",
  );
  await delay(1_000);

  const componentNoticeCount = tui
    .output()
    .split("component_tools_active").length;
  tui.child.stdin.write("/reload\r");
  await waitFor(
    tui.output,
    (output) =>
      output.split("component_tools_active").length > componentNoticeCount &&
      output.includes("workspace: tui-switched"),
    "reload-restored-logical-workspace",
  );
  await delay(1_000);
  fs.rmSync(switchedProof);
  if (liveTool) {
    tui.child.stdin.write(
      "Call development_system_worktree_finish now. Do not use bash. End your final response with LIVE_FINISH_COMPLETE.\r",
    );
    await waitFor(
      () => ({
        output: tui.output(),
        removed: !fs.existsSync(switchedWorktree),
      }),
      (state) => state.removed && state.output.includes("LIVE_FINISH_COMPLETE"),
      "live-semantic-finish",
      10 * 60_000,
    );
  } else {
    tui.child.stdin.write("/development-system-worktree-finish\r");
    await waitFor(
      tui.output,
      (output) => output.includes("Finish and remove this worktree?"),
      "finish-confirmation",
    );
    tui.child.stdin.write("\r");
    await waitFor(
      tui.output,
      (output) => output.includes("Removed finished worktree"),
      "finish-removal",
    );
  }
  await stopTui(tui.child);

  assert.equal(fs.existsSync(switchedWorktree), false);
  assert.equal(fs.existsSync(createdWorktree), true);
  assert.equal(
    git(repository, "branch", "--list", "tui-switched", "--format=%(refname)"),
    "refs/heads/tui-switched",
  );
  assert.doesNotMatch(tui.output(), /--automatic/);
  let privateTransitionMessages = 0;
  if (liveTool) {
    const entries = sessionEntries(temporary);
    const toolCalls = entries.flatMap((entry) =>
      entry.type === "message" &&
      entry.message?.role === "assistant" &&
      Array.isArray(entry.message.content)
        ? entry.message.content.filter((item) => item?.type === "toolCall")
        : [],
    );
    for (const toolName of [
      "development_system_worktree_create",
      "development_system_worktree_switch",
      "development_system_worktree_finish",
    ]) {
      assert.ok(toolCalls.some((call) => call.name === toolName));
      assert.ok(
        entries.some(
          (entry) =>
            entry.type === "message" &&
            entry.message?.role === "toolResult" &&
            entry.message.toolName === toolName &&
            entry.message.isError !== true,
        ),
      );
    }
    assert.equal(
      toolCalls.some((call) => call.name === "bash"),
      false,
    );
    privateTransitionMessages = entries.filter(
      (entry) =>
        entry.type === "message" &&
        entry.message?.role === "user" &&
        /development-system-worktree-(?:switch|finish) --automatic/.test(
          JSON.stringify(entry.message.content),
        ),
    ).length;
  }
  assert.equal(privateTransitionMessages, 0);
  console.log(
    JSON.stringify({
      status: "passed",
      mode: "local-tui",
      semanticCreateTool: semanticCreateObserved,
      semanticCreateCommand: !liveTool,
      logicalWorkspaceActivated: true,
      existingWorktreeSwitched: true,
      routedShellEffect: true,
      hostPrimaryUnchanged: true,
      sessionStateRestored: true,
      cleanupRemovedWorktree: true,
      branchPreserved: true,
      privateTransitionMessages,
      sourceLoginUnchanged: liveTool ? true : undefined,
      exactAuthLeakCheck: liveTool ? "passed" : undefined,
      piExecutable,
    }),
  );
} finally {
  const cleanupErrors = [];
  for (const process of processes) {
    try {
      await stopTui(process);
    } catch (error) {
      cleanupErrors.push(error);
    }
  }
  try {
    if (liveTool && sourceAuthBefore) {
      const sourceAuthAfter = fs.readFileSync(sourceAuthPath);
      assert.equal(digest(sourceAuthAfter), digest(sourceAuthBefore));
      const inspectedOutput = liveOutput();
      for (const secret of exactAuthSecrets)
        assert.equal(
          inspectedOutput.includes(secret),
          false,
          "live TUI output contained copied authentication material",
        );
    }
  } catch (error) {
    cleanupErrors.push(error);
  }
  try {
    fs.rmSync(temporary, { recursive: true, force: true });
  } catch (error) {
    cleanupErrors.push(error);
  }
  if (cleanupErrors.length > 0)
    throw new AggregateError(
      cleanupErrors,
      "development_system.tui_scenario_cleanup_failed",
    );
}
