import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import {
  LOGICAL_WORKSPACE_ENTRY,
  LogicalWorkspaceAuthority,
} from "../../plugins/development-system/extensions/development-system/adapters/logical-workspace.ts";

function git(cwd, ...args) {
  return execFileSync("git", args, { cwd, encoding: "utf8" }).trim();
}

function repository() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "pi-logical-workspace-"));
  git(root, "init", "--initial-branch=main");
  git(root, "config", "user.name", "Test");
  git(root, "config", "user.email", "test@example.invalid");
  fs.writeFileSync(path.join(root, "README.md"), "fixture\n");
  fs.writeFileSync(
    path.join(root, ".development-system.toml"),
    `schema_version = 2
[delivery]
mode = "direct-to-trunk"
trunk_branch = "main"
[features]
worktrees = true
beads = false
agentic_systems = false
eval_case_reporting = false
[worktrees]
root = ".worktrees"
[beads]
workflow = "development-change-direct"
`,
  );
  git(root, "add", ".");
  git(root, "commit", "-m", "test: fixture");
  const linked = path.join(root, ".worktrees", "feature");
  git(root, "worktree", "add", "-b", "feature", linked);
  return { root: fs.realpathSync(root), linked: fs.realpathSync(linked) };
}

function context(cwd, entries = []) {
  return {
    cwd,
    sessionManager: { getBranch: () => entries },
    ui: { setStatus() {}, notify() {} },
  };
}

test("logical workspace activation persists a non-chat session entry", async () => {
  const { root, linked } = repository();
  const entries = [];
  const authority = new LogicalWorkspaceAuthority((customType, data) =>
    entries.push({ type: "custom", customType, data }),
  );

  await authority.restore(context(root));
  const activated = await authority.activate(context(root), linked);

  assert.equal(activated.path, linked);
  assert.equal(activated.kind, "linked");
  assert.equal(authority.path(root), linked);
  assert.equal(entries.length, 1);
  assert.equal(entries[0].customType, LOGICAL_WORKSPACE_ENTRY);
  assert.equal(entries[0].data.path, linked);
});

test("latest active-branch logical workspace entry restores after reload", async () => {
  const { root, linked } = repository();
  const written = [];
  const first = new LogicalWorkspaceAuthority((customType, data) =>
    written.push({ type: "custom", customType, data }),
  );
  await first.restore(context(root));
  await first.activate(context(root), linked);

  fs.writeFileSync(path.join(linked, "advance.txt"), "advance\n");
  git(linked, "add", "advance.txt");
  git(linked, "commit", "-m", "test: advance");

  const restored = new LogicalWorkspaceAuthority(() => {});
  const result = await restored.restore(context(root, written));
  assert.equal(result.status, "restored");
  assert.equal(restored.path(root), linked);

  await first.activatePrimary(context(root));
  const primary = new LogicalWorkspaceAuthority(() => {});
  await primary.restore(context(root, written));
  assert.equal(primary.path(root), root);
});

test("stale registration or branch identity is rejected on restoration", async () => {
  const { root, linked } = repository();
  const entries = [];
  const authority = new LogicalWorkspaceAuthority((customType, data) =>
    entries.push({ type: "custom", customType, data }),
  );
  await authority.restore(context(root));
  await authority.activate(context(root), linked);
  git(root, "worktree", "remove", linked);
  git(root, "worktree", "add", linked, "feature");
  await assert.rejects(
    () => authority.resolve(root),
    /logical_workspace_stale_registration/,
  );

  const restored = new LogicalWorkspaceAuthority(() => {});
  const result = await restored.restore(context(root, entries));
  assert.equal(result.status, "rejected");
  assert.match(result.code, /logical_workspace_stale/);
  assert.equal(restored.path(root), root);
});

test("primary finish transition serializes cleanup against workspace switches", async () => {
  const { root, linked } = repository();
  const authority = new LogicalWorkspaceAuthority(() => {});
  await authority.restore(context(root));
  await authority.activate(context(root), linked);
  let release;
  const cleanup = new Promise((resolve) => {
    release = resolve;
  });
  const finishing = authority.finish(
    context(root),
    async () => "prepared",
    async () => {
      await cleanup;
      return "finished";
    },
  );
  await new Promise((resolve) => setImmediate(resolve));

  await assert.rejects(
    () => authority.activate(context(root), linked),
    /logical_workspace_transition_busy/,
  );
  release();
  assert.equal(await finishing, "finished");
  assert.equal(authority.path(root), root);
});

test("failed activation leaves the previous workspace authoritative", async () => {
  const { root, linked } = repository();
  const authority = new LogicalWorkspaceAuthority(() => {});
  await authority.restore(context(root));
  await authority.activate(context(root), linked);

  await assert.rejects(
    () => authority.activate(context(root), path.join(root, "not-registered")),
    /logical_workspace_not_registered/,
  );
  assert.equal(authority.path(root), linked);
});
