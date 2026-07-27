import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import {
  classifyPath,
  classifyShellCommand,
  deliveryDecision,
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

test("shell classifier permits bounded read-only work and fails closed on ambiguity", () => {
  assert.deepEqual(classifyShellCommand("git status --short"), {
    kind: "read-only",
  });
  assert.equal(classifyShellCommand("git push origin main").kind, "delivery");
  assert.equal(
    classifyShellCommand("git push --force-with-lease origin main").kind,
    "destructive-delivery",
  );
  assert.equal(classifyShellCommand("printf x > file").kind, "mutation");
  assert.equal(classifyShellCommand("git status && touch x").kind, "ambiguous");
  assert.equal(classifyShellCommand("python script.py").kind, "ambiguous");
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
