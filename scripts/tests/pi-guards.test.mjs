import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import {
  classifyPath,
  classifyShellCommand,
  classifyShellDelivery,
  deliveryDecision,
  worktreeTargetAllowed,
} from "../../plugins/development-system/extensions/development-system/core/guards.ts";

function repository() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "pi-guards-"));
  execFileSync("git", ["init", "--initial-branch=main", root]);
  execFileSync("git", ["-C", root, "config", "user.name", "Test"]);
  execFileSync("git", ["-C", root, "config", "user.email", "test@example.com"]);
  fs.writeFileSync(path.join(root, "README.md"), "fixture\n");
  execFileSync("git", ["-C", root, "add", "README.md"]);
  execFileSync("git", ["-C", root, "commit", "-m", "test: fixture"]);
  return root;
}

test("path guard canonicalizes absolute, relative, and symlink targets", () => {
  const root = repository();
  fs.mkdirSync(path.join(root, "src"));
  fs.symlinkSync(root, path.join(root, "src", "alias"));
  assert.equal(
    classifyPath({ rawPath: "src/file.ts", cwd: root, boundary: root }).kind,
    "inside",
  );
  assert.equal(
    classifyPath({
      rawPath: path.join(root, ".git/config"),
      cwd: root,
      boundary: root,
    }).kind,
    "protected-metadata",
  );
  assert.equal(
    classifyPath({ rawPath: "src/alias/.env", cwd: root, boundary: root }).kind,
    "protected-secret",
  );
  assert.equal(
    classifyPath({ rawPath: "../outside", cwd: root, boundary: root }).kind,
    "outside",
  );
});

test("worktree creation targets remain inside primary coordination storage", () => {
  const root = repository();
  assert.equal(
    worktreeTargetAllowed({
      rawPath: ".worktrees/feature",
      cwd: root,
      primary: root,
    }),
    true,
  );
  assert.equal(
    worktreeTargetAllowed({
      rawPath: "../feature",
      cwd: root,
      primary: root,
    }),
    false,
  );
  const outside = fs.mkdtempSync(
    path.join(os.tmpdir(), "pi-worktrees-outside-"),
  );
  fs.symlinkSync(outside, path.join(root, ".worktrees"));
  assert.equal(
    worktreeTargetAllowed({
      rawPath: ".worktrees/feature",
      cwd: root,
      primary: root,
    }),
    false,
  );
});

test("shell classifier allows exploration while identifying direct repository mutation", () => {
  assert.deepEqual(classifyShellCommand("git status --short"), {
    kind: "read-only",
  });
  assert.equal(classifyShellCommand("git push origin main").kind, "delivery");
  assert.equal(
    classifyShellCommand("git push --force-with-lease origin main").kind,
    "destructive-delivery",
  );
  assert.deepEqual(
    classifyShellCommand(
      "git worktree add .worktrees/observable-subagents -b feat/observable-subagents",
    ),
    {
      kind: "worktree-creation",
      targetPath: ".worktrees/observable-subagents",
      branch: "feat/observable-subagents",
    },
  );
  assert.deepEqual(
    classifyShellCommand(
      "git worktree add -b feat/observable-subagents .worktrees/observable-subagents",
    ),
    {
      kind: "worktree-creation",
      targetPath: ".worktrees/observable-subagents",
      branch: "feat/observable-subagents",
    },
  );
  assert.equal(
    classifyShellCommand(
      "git worktree add .worktrees/observable-subagents -b feat/observable-subagents origin/main",
    ).kind,
    "mutation",
  );
  assert.deepEqual(
    classifyShellCommand("git -C .worktrees/feature status --short --branch"),
    { kind: "read-only-discovery", targetPath: ".worktrees/feature" },
  );
  assert.deepEqual(
    classifyShellCommand(
      "cd .worktrees/feature && git status --short --branch",
    ),
    { kind: "read-only-discovery", targetPath: ".worktrees/feature" },
  );
  assert.equal(
    classifyShellCommand("scripts/agent-checkout-guard.sh").kind,
    "read-only",
  );
  assert.equal(
    classifyShellCommand("git branch --show-current").kind,
    "read-only",
  );
  assert.equal(
    classifyShellCommand("git worktree list --porcelain").kind,
    "read-only",
  );
  assert.equal(classifyShellCommand("git branch new-branch").kind, "mutation");
  assert.equal(
    classifyShellCommand("cd .worktrees/feature && touch x").kind,
    "mutation",
  );
  assert.equal(classifyShellCommand("printf x > file").kind, "mutation");
  assert.equal(classifyShellCommand("git status && touch x").kind, "mutation");
  assert.equal(classifyShellCommand("python script.py").kind, "read-only");
  assert.equal(
    classifyShellCommand("git -c core.hooksPath=/dev/null commit -m bypass")
      .kind,
    "mutation",
  );
  assert.equal(
    classifyShellCommand("env SAFE=1 git reset --hard HEAD~1").kind,
    "mutation",
  );
  assert.equal(classifyShellCommand("command git clean -fd").kind, "mutation");
  assert.equal(
    classifyShellCommand("git -c color.ui=false status --short").kind,
    "read-only",
  );
  assert.equal(
    classifyShellCommand("git -c color.ui=false push origin main").kind,
    "delivery",
  );
  assert.equal(
    classifyShellCommand("command git push --force origin main").kind,
    "destructive-delivery",
  );
  assert.equal(
    classifyShellCommand("git status; git push origin main").kind,
    "delivery",
  );
  assert.equal(
    classifyShellCommand("git commit -m increment; git push origin main").kind,
    "mutation",
  );
  assert.equal(
    classifyShellDelivery(
      "git commit -m increment; git push --force origin main",
    ).kind,
    "destructive-delivery",
  );
  assert.equal(
    classifyShellCommand("X=1 git commit -m prefixed").kind,
    "mutation",
  );
  assert.equal(
    classifyShellDelivery("X=1 git push --force origin main").kind,
    "destructive-delivery",
  );
  assert.equal(
    classifyShellCommand("(git reset --hard HEAD~1)").kind,
    "mutation",
  );
  assert.equal(classifyShellCommand('git c""ommit -m bypass').kind, "mutation");
  assert.equal(
    classifyShellCommand("FOO='a b' git commit -m bypass").kind,
    "mutation",
  );
  assert.equal(
    classifyShellDelivery('git pu""sh origin main').kind,
    "delivery",
  );
  assert.equal(
    classifyShellDelivery("git push '--force' origin main").kind,
    "destructive-delivery",
  );
  assert.equal(
    classifyShellDelivery("/usr/bin/git push --force origin main").kind,
    "destructive-delivery",
  );
  assert.equal(
    classifyShellCommand('/bin/sh -c "git reset --hard HEAD~1"').kind,
    "mutation",
  );
  assert.equal(classifyShellCommand('/bin/rm -rf "$PWD"').kind, "mutation");
  assert.equal(
    classifyShellDelivery('bash -lc "git push --force origin main"').kind,
    "destructive-delivery",
  );
  assert.equal(
    classifyShellDelivery("exec git --no-replace-objects push origin main")
      .kind,
    "delivery",
  );
  assert.equal(
    classifyShellDelivery("git push -fu origin main").kind,
    "destructive-delivery",
  );
  assert.equal(
    classifyShellDelivery("! time -p git push --force origin main").kind,
    "destructive-delivery",
  );
  for (const command of [
    "truncate -s 0 valuable",
    "unlink valuable",
    "ln -sf replacement valuable",
    "git pull --rebase",
    "git unknown-alias",
    "sed -i s/old/new/ README.md",
    "find . -delete",
    "rsync -a source/ destination/",
    "git diff --output=README.md",
    "git show --textconv HEAD:file",
    "git cat-file --filters --path=README.md HEAD:README.md",
    "git -c filter.pwn.smudge=touch cat-file --filters HEAD:file",
    "git -c diff.external=touch diff",
    "git --paginate status",
    "env -C . git -c core.hooksPath=/dev/null commit --allow-empty -m bypass",
    "env -S 'git commit --allow-empty -m bypass'",
    "sudo -u root git commit --allow-empty -m bypass",
    "nohup command git reset --hard HEAD~1",
    "printf '%s\\n' valuable | xargs rm",
    "find . -exec rm {} +",
    "dd if=/dev/zero of=README.md count=1",
    "fallocate -l 0 README.md",
    "patch -p1 change.patch",
    "curl -sSLo README.md https://example.invalid/file",
    "env --split-string='git reset --hard HEAD~1'",
    "r\\\nm -rf target",
    "timeout 10 git reset --hard HEAD~1",
  ])
    assert.equal(classifyShellCommand(command).kind, "mutation", command);
  for (const command of [
    "git diff -- README.md",
    "git show HEAD:README.md",
    "git grep pattern",
    "git ls-files",
  ])
    assert.equal(classifyShellCommand(command).kind, "read-only", command);
});

test("delivery decisions never infer a missing mode or destructive approval", () => {
  assert.equal(
    deliveryDecision({
      mode: null,
      branch: "main",
      trunk: "main",
      destructive: false,
    }).code,
    "development_system.delivery_mode_missing",
  );
  assert.equal(
    deliveryDecision({
      mode: "local-only",
      branch: "main",
      trunk: "main",
      destructive: false,
    }).code,
    "development_system.local_only_publication_blocked",
  );
  assert.equal(
    deliveryDecision({
      mode: "pull-request",
      branch: "main",
      trunk: "main",
      destructive: false,
    }).code,
    "development_system.pull_request_branch_required",
  );
  assert.equal(
    deliveryDecision({
      mode: "direct-to-trunk",
      branch: "feature",
      trunk: "main",
      destructive: false,
    }).code,
    "development_system.direct_trunk_branch_required",
  );
  assert.equal(
    deliveryDecision({
      mode: "direct-to-trunk",
      branch: "main",
      trunk: "main",
      destructive: true,
    }).code,
    "development_system.destructive_approval_required",
  );
  assert.equal(
    deliveryDecision({
      mode: "direct-to-trunk",
      branch: "main",
      trunk: "main",
      destructive: false,
    }),
    null,
  );
});
