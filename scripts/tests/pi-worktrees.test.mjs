import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import {
  createWorktree,
  listWorktrees,
  removeWorktree,
} from "../../plugins/development-system/extensions/development-system/adapters/worktrees.ts";
import {
  parseWorktreeBranch,
  parseWorktreeName,
} from "../../plugins/development-system/extensions/development-system/core/worktrees.ts";

function repository() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "pi-worktree-tool-"));
  execFileSync("git", ["init", "--initial-branch=main", root]);
  execFileSync("git", ["-C", root, "config", "user.name", "Test"]);
  execFileSync("git", [
    "-C",
    root,
    "config",
    "user.email",
    "test@example.invalid",
  ]);
  execFileSync("git", ["-C", root, "config", "commit.gpgSign", "false"]);
  fs.writeFileSync(path.join(root, "README.md"), "fixture\n");
  fs.writeFileSync(
    path.join(root, ".development-system.toml"),
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
  execFileSync("git", ["-C", root, "add", "."]);
  execFileSync("git", ["-C", root, "commit", "-m", "test: fixture"]);
  return root;
}

test("primary checkout lists and creates a canonical session-switchable worktree", async () => {
  const root = repository();
  const before = await listWorktrees(root);
  assert.equal(before.currentKind, "primary");
  assert.equal(before.worktrees.length, 1);

  const created = await createWorktree(root, {
    name: "observable-subagents",
    branch: "feat/observable-subagents",
  });

  assert.equal(created.status, "created");
  assert.equal(
    created.path,
    path.join(root, ".worktrees", "observable-subagents"),
  );
  assert.equal(created.requiresRelaunch, false);
  assert.equal(created.requiresUserWorkspaceSwitch, true);
  assert.equal(
    created.switchCommand,
    "/development-system-worktree-switch feat/observable-subagents",
  );
  assert.match(created.relaunchCommand, /^cd -- '.*' && exec pi$/);
  assert.match(created.nextAction, /preserve this conversation/i);
  assert.equal(
    execFileSync("git", ["-C", created.path, "branch", "--show-current"], {
      encoding: "utf8",
    }).trim(),
    "feat/observable-subagents",
  );
});

test("existing worktree paths and branches return actionable collisions", async () => {
  const root = repository();
  const created = await createWorktree(root, {
    name: "one",
    branch: "feat/one",
  });
  assert.equal(created.status, "created");

  const pathCollision = await createWorktree(root, {
    name: "one",
    branch: "feat/two",
  });
  assert.equal(pathCollision.status, "collision");
  assert.equal(pathCollision.code, "development_system.worktree_path_exists");
  assert.match(pathCollision.nextAction, /Switch this Pi conversation/);

  const branchCollision = await createWorktree(root, {
    name: "two",
    branch: "feat/one",
  });
  assert.equal(branchCollision.status, "collision");
  assert.equal(
    branchCollision.code,
    "development_system.worktree_branch_exists",
  );
  assert.match(branchCollision.nextAction, /Switch this Pi conversation/);
});

test("cleanup removes a clean linked worktree while preserving its branch", async () => {
  const root = repository();
  const created = await createWorktree(root, {
    name: "finished",
    branch: "feat/finished",
  });
  assert.equal(created.status, "created");

  const expected = (await listWorktrees(root)).worktrees.find(
    (worktree) => worktree.path === created.path,
  );
  const removed = await removeWorktree(root, expected);

  assert.equal(removed.status, "removed");
  assert.equal(removed.path, created.path);
  assert.equal(removed.branch, "feat/finished");
  assert.equal(fs.existsSync(created.path), false);
  assert.equal(
    execFileSync(
      "git",
      ["-C", root, "branch", "--list", "feat/finished", "--format=%(refname)"],
      { encoding: "utf8" },
    ).trim(),
    "refs/heads/feat/finished",
  );
});

test("cleanup refuses a dirty linked worktree without deleting user state", async () => {
  const root = repository();
  const created = await createWorktree(root, {
    name: "dirty",
    branch: "feat/dirty",
  });
  assert.equal(created.status, "created");
  fs.writeFileSync(path.join(created.path, "user-state.txt"), "keep me\n");

  const expected = (await listWorktrees(root)).worktrees.find(
    (worktree) => worktree.path === created.path,
  );
  await assert.rejects(
    () => removeWorktree(root, expected),
    /worktree_cleanup_dirty/,
  );
  assert.equal(
    fs.readFileSync(path.join(created.path, "user-state.txt"), "utf8"),
    "keep me\n",
  );
  assert.ok(
    (await listWorktrees(root)).worktrees.some(
      (worktree) => worktree.path === created.path,
    ),
  );
});

test("cleanup rejects detached HEAD and identity changes without removal", async () => {
  const root = repository();
  const detachedPath = path.join(root, ".worktrees", "detached");
  execFileSync("git", [
    "-C",
    root,
    "worktree",
    "add",
    "--detach",
    detachedPath,
  ]);
  const detached = (await listWorktrees(root)).worktrees.find(
    (worktree) => worktree.path === detachedPath,
  );
  await assert.rejects(
    () => removeWorktree(root, detached),
    /worktree_cleanup_detached_head/,
  );
  assert.equal(fs.existsSync(detachedPath), true);

  const created = await createWorktree(root, {
    name: "identity-change",
    branch: "feat/identity-change",
  });
  assert.equal(created.status, "created");
  const expected = (await listWorktrees(root)).worktrees.find(
    (worktree) => worktree.path === created.path,
  );
  fs.writeFileSync(path.join(created.path, "advance.txt"), "advance\n");
  execFileSync("git", ["-C", created.path, "add", "advance.txt"]);
  execFileSync("git", [
    "-C",
    created.path,
    "commit",
    "-m",
    "test: advance identity",
  ]);

  await assert.rejects(
    () => removeWorktree(root, expected),
    /worktree_cleanup_identity_changed/,
  );
  assert.equal(fs.existsSync(created.path), true);
});

test("semantic worktree inputs reject traversal, options, controls, and malformed refs", () => {
  for (const input of [
    "../escape",
    "nested/path",
    "-option",
    "bad\nname",
    "bad\0name",
    "",
  ]) {
    assert.throws(() => parseWorktreeName(input), /worktree_name_invalid/);
  }
  for (const input of [
    "-option",
    "../escape",
    "feat//double",
    "feat/@{bad",
    "bad\nbranch",
    "bad\0branch",
    "",
  ]) {
    assert.throws(() => parseWorktreeBranch(input), /worktree_branch_invalid/);
  }
});

test("configured worktree root symlink escape is rejected without mutation", async () => {
  const root = repository();
  const outside = fs.mkdtempSync(
    path.join(os.tmpdir(), "pi-worktree-outside-"),
  );
  fs.symlinkSync(outside, path.join(root, ".worktrees"));

  await assert.rejects(
    () =>
      createWorktree(root, {
        name: "escape",
        branch: "feat/escape",
      }),
    /worktree_root_symlink_escape/,
  );
  assert.equal(fs.readdirSync(outside).length, 0);
});

test("partial Git failure preserves user and diagnostic state without cleanup", async () => {
  const root = repository();
  const lock = path.join(root, ".git", "refs", "heads", "feat", "fail.lock");
  fs.mkdirSync(lock, { recursive: true });

  const result = await createWorktree(root, {
    name: "failed",
    branch: "feat/fail",
  });

  assert.equal(result.status, "failed");
  assert.equal(result.code, "development_system.worktree_git_failed");
  assert.match(result.nextAction, /No cleanup was performed/);
  assert.equal(fs.existsSync(lock), true);
  assert.equal(fs.existsSync(path.join(root, "README.md")), true);
  assert.equal(fs.existsSync(path.join(root, ".worktrees", "failed")), true);
  assert.equal(
    execFileSync("git", ["-C", root, "status", "--porcelain"], {
      encoding: "utf8",
    }),
    "",
  );
});

test("concurrent same-target creation deterministically preserves one worktree", async () => {
  const root = repository();
  const [first, second] = await Promise.all([
    createWorktree(root, { name: "race", branch: "feat/race" }),
    createWorktree(root, { name: "race", branch: "feat/race" }),
  ]);

  assert.deepEqual([first.status, second.status].sort(), [
    "collision",
    "created",
  ]);
  const inventory = await listWorktrees(root);
  assert.equal(
    inventory.worktrees.filter((worktree) => worktree.branch === "feat/race")
      .length,
    1,
  );
});
