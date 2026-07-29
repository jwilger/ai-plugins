import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { SessionManager } from "@earendil-works/pi-coding-agent";
import {
  prepareWorktreeSession,
  switchWorktreeSession,
} from "../../plugins/development-system/extensions/development-system/adapters/worktree-session.ts";

function fixture() {
  const root = fs.mkdtempSync(
    path.join(os.tmpdir(), "development-system-switch-"),
  );
  const source = path.join(root, "source");
  const target = path.join(root, "target");
  fs.mkdirSync(source);
  fs.mkdirSync(target);
  return { root, source, target };
}

function assistant(text) {
  return {
    role: "assistant",
    content: [{ type: "text", text }],
    api: "test",
    provider: "test",
    model: "test",
    usage: {
      input: 1,
      output: 1,
      cacheRead: 0,
      cacheWrite: 0,
      totalTokens: 2,
      cost: {
        input: 0,
        output: 0,
        cacheRead: 0,
        cacheWrite: 0,
        total: 0,
      },
    },
    stopReason: "stop",
    timestamp: Date.now(),
  };
}

test("prepared worktree session preserves the active conversation branch and uses private storage", () => {
  const { root, source, target } = fixture();
  try {
    const manager = SessionManager.create(
      source,
      path.join(root, "source-sessions"),
    );
    manager.appendMessage({
      role: "user",
      content: "active branch",
      timestamp: Date.now(),
    });
    manager.appendMessage(assistant("active answer"));
    const active = manager.appendCustomEntry("development-system-goal-state", {
      goalId: "goal-in-worktree",
      status: "active",
    });
    manager.appendMessage({
      role: "user",
      content: "abandoned branch",
      timestamp: Date.now(),
    });
    manager.branch(active);

    const prepared = prepareWorktreeSession(manager, target);
    const switched = SessionManager.open(prepared);

    assert.equal(switched.getCwd(), fs.realpathSync(target));
    assert.equal(switched.getLeafId(), active);
    assert.deepEqual(
      switched.buildSessionContext().messages.map((message) => message.role),
      ["user", "assistant"],
    );
    const goalEntry = switched
      .getBranch()
      .find(
        (entry) =>
          entry.type === "custom" &&
          entry.customType === "development-system-goal-state",
      );
    assert.equal(goalEntry.data.goalId, "goal-in-worktree");
    assert.equal(fs.statSync(prepared).mode & 0o777, 0o600);
    assert.equal(prepared.startsWith(target), false);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test("switch uses Pi session replacement and never touches stale source context after success", async () => {
  const { root, source, target } = fixture();
  try {
    const manager = SessionManager.inMemory(source);
    manager.appendMessage({
      role: "user",
      content: "continue in worktree",
      timestamp: Date.now(),
    });
    let switchedPath = "";
    let replacementNotice = "";
    let sourceNotifyCalls = 0;
    const context = {
      cwd: source,
      mode: "tui",
      hasUI: true,
      sessionManager: manager,
      ui: {
        notify() {
          sourceNotifyCalls += 1;
        },
      },
      async switchSession(sessionPath, options) {
        switchedPath = sessionPath;
        await options.withSession({
          cwd: fs.realpathSync(target),
          ui: {
            notify(message) {
              replacementNotice = message;
            },
          },
        });
        return { cancelled: false };
      },
    };

    const result = await switchWorktreeSession(context, target);

    assert.equal(result.status, "switched");
    assert.equal(result.target, fs.realpathSync(target));
    assert.equal(SessionManager.open(switchedPath).getCwd(), result.target);
    assert.match(replacementNotice, /workspace switched/i);
    assert.equal(sourceNotifyCalls, 0);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test("headless and replacement failures fail closed while retaining recoverable session state", async () => {
  const { root, source, target } = fixture();
  try {
    const manager = SessionManager.inMemory(source);
    const notices = [];
    const headless = await switchWorktreeSession(
      {
        cwd: source,
        mode: "json",
        hasUI: false,
        sessionManager: manager,
        ui: { notify() {} },
      },
      target,
    );
    assert.deepEqual(headless, {
      status: "unsupported",
      code: "development_system.worktree_switch_requires_local_tui",
      target: fs.realpathSync(target),
    });

    let prepared = "";
    const failed = await switchWorktreeSession(
      {
        cwd: source,
        mode: "tui",
        hasUI: true,
        sessionManager: manager,
        ui: { notify: (message) => notices.push(message) },
        async switchSession(sessionPath) {
          prepared = sessionPath;
          throw new Error("replacement failed");
        },
      },
      target,
    );
    assert.equal(failed.status, "failed");
    assert.equal(failed.sessionPath, prepared);
    assert.equal(fs.existsSync(prepared), true);
    assert.match(notices.at(-1), /retained.*replacement failed/i);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});
