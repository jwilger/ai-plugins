#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { posix } from "node:path";

const supersessionStatus =
  /^Superseded by \[[A-Za-z0-9 ._-]+\]\(([A-Za-z0-9._/-]+)\)\.?$/i;

const isAdrPath = (path) =>
  path.startsWith("docs/adr/") &&
  path.endsWith(".md") &&
  !path.endsWith("/README.md");

function statusRecord(document) {
  // Restrict automatic exceptions to the generated, unambiguous leading status
  // field. Other historical formats require review, never history rewriting.
  const heading =
    /^(?:\uFEFF)?# [^\r\n]+\r?\n(?:[ \t]*\r?\n)*## Status[ \t]*\r?\n/.exec(
      document,
    );
  if (!heading) return null;
  const start = heading[0].length;
  const remainder = document.slice(start);
  const boundary = /^#{1,6}(?:[ \t]+|$)/m.exec(remainder);
  const end = boundary ? start + boundary.index : document.length;
  const status = document.slice(start, end).trim();
  if (
    !/^(?:Proposed|Accepted|Rejected|Deprecated|Withdrawn|Superseded)$/i.test(
      status,
    ) &&
    !supersessionStatus.test(status)
  )
    return null;
  return {
    prefix: document.slice(0, start),
    status,
    suffix: document.slice(end),
  };
}

function isSupersession(before, after, oldPath, newPath) {
  const oldRecord = statusRecord(before);
  const newRecord = statusRecord(after);
  if (
    !oldRecord ||
    !newRecord ||
    oldRecord.prefix !== newRecord.prefix ||
    oldRecord.suffix !== newRecord.suffix
  )
    return false;
  const reference = supersessionStatus.exec(newRecord.status);
  return (
    reference !== null &&
    !posix.isAbsolute(reference[1]) &&
    posix.normalize(posix.join(posix.dirname(oldPath), reference[1])) ===
      newPath
  );
}

const options = new Map();
try {
  const args = process.argv.slice(2);
  const allowed = new Set([
    "--rules",
    "--project",
    "--base",
    "--head",
    "--trunk",
    "--delivery",
  ]);
  for (let index = 0; index < args.length; index += 2) {
    const name = args[index];
    const value = args[index + 1];
    if (
      !allowed.has(name) ||
      options.has(name) ||
      !value ||
      value.startsWith("--")
    ) {
      throw new Error(`invalid option or value: ${name}`);
    }
    options.set(name, value);
  }
  const knownRules = [
    "pairing",
    "one-adr",
    "immutable-history",
    "supersession-only",
  ];
  const rules = options.has("--rules") ? options.get("--rules").split(",") : [];
  if (
    rules.some((rule) => !knownRules.includes(rule)) ||
    new Set(rules).size !== rules.length
  ) {
    throw new Error("unknown or duplicate rule");
  }
  if (rules.length === 0) {
    console.log(
      JSON.stringify({
        schema_version: 1,
        status: "skipped",
        reason: "no-adopted-rules",
        rules,
        checks: [],
        violations: [],
        review_required: [],
      }),
    );
    process.exit(0);
  }
  const adopted = (rule) => rules.includes(rule);
  for (const name of allowed) {
    if (name === "--rules") continue;
    if (!options.has(name)) throw new Error(`required option: ${name}`);
  }
  const delivery = options.get("--delivery");
  if (!["preserve", "squash"].includes(delivery)) {
    throw new Error("delivery must be preserve or squash");
  }
  const gitOutput = (...args) =>
    execFileSync("git", ["-C", options.get("--project"), ...args], {
      stdio: ["ignore", "pipe", "pipe"],
      env: { ...process.env, GIT_NO_REPLACE_OBJECTS: "1" },
    });
  const git = (...args) =>
    gitOutput(...args)
      .toString("utf8")
      .trim();
  const blob = (commit, path) =>
    new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(
      gitOutput("show", `${commit}:${path}`),
    );
  const hasArchitectureUpdate = (before, after, changes) => {
    const change = changes.find(
      ({ path, status }) =>
        path === "ARCHITECTURE.md" && ["A", "M"].includes(status),
    );
    if (!change) return false;
    const current = git("show", `${after}:ARCHITECTURE.md`).replace(/\s/gu, "");
    const previous =
      change.status === "A"
        ? ""
        : git("show", `${before}:ARCHITECTURE.md`).replace(/\s/gu, "");
    return current.length > 0 && current !== previous;
  };
  const resolve = (name) =>
    git(
      "rev-parse",
      "--verify",
      "--end-of-options",
      `${options.get(name)}^{commit}`,
    );
  if (git("rev-parse", "--is-shallow-repository") !== "false") {
    throw new Error(
      "complete history is required; shallow repositories are unsupported",
    );
  }
  const base = resolve("--base");
  const head = resolve("--head");
  const trunk = resolve("--trunk");
  git("merge-base", "--is-ancestor", base, head);
  const commonBases = git("merge-base", "--all", trunk, head).split("\n");
  if (commonBases.length !== 1 || commonBases[0] !== base) {
    throw new Error("base must be the unique merge base of trunk and head");
  }
  const commits = git(
    "rev-list",
    "--reverse",
    "--topo-order",
    `${base}..${head}`,
  )
    .split("\n")
    .filter(Boolean);
  const violations = [];
  const mergedAdrs = new Set(
    git(
      "log",
      "--full-history",
      "-m",
      "--format=",
      "--name-only",
      "-z",
      "--no-renames",
      "--diff-filter=A",
      trunk,
      "--",
      "docs/adr",
    )
      .split("\0")
      .filter(Boolean),
  );
  for (const commit of commits) {
    const parents = git("rev-list", "--parents", "-n", "1", commit)
      .split(" ")
      .slice(1);
    if (parents.length !== 1) {
      throw new Error(
        "architecture checks currently require linear candidate commits",
      );
    }
    const entries = git(
      "diff-tree",
      "--no-commit-id",
      "--name-status",
      "-r",
      "-z",
      "--no-renames",
      commit,
    ).split("\0");
    const changed = [];
    for (let index = 0; index + 1 < entries.length; index += 2) {
      changed.push({ status: entries[index], path: entries[index + 1] });
    }
    const adrChanges = changed.filter(({ path }) => isAdrPath(path));
    if (adrChanges.length === 0) continue;
    const added = adrChanges.filter(({ status }) => status === "A");
    for (const { path, status } of adrChanges) {
      const merged = mergedAdrs.has(path);
      if (
        (adopted("immutable-history") && merged) ||
        (adopted("supersession-only") && added.length > 0 && status !== "A")
      ) {
        const superseded =
          added.length === 1 &&
          status === "M" &&
          isSupersession(
            blob(parents[0], path),
            blob(commit, path),
            path,
            added[0].path,
          );
        if (!superseded)
          violations.push({
            code:
              adopted("immutable-history") && merged
                ? "merged-adr-immutable"
                : "other-adr-supersession-only",
            commit,
            path,
          });
      }
    }
    if (adopted("one-adr") && added.length > 1) {
      violations.push({
        code: "multiple-new-adrs",
        commit,
        paths: added.map(({ path }) => path),
      });
    }
    if (
      adopted("pairing") &&
      !hasArchitectureUpdate(parents[0], commit, changed)
    ) {
      violations.push({ code: "architecture-update-required", commit });
    }
  }
  if (delivery === "squash") {
    const entries = git(
      "diff",
      "--name-status",
      "--no-renames",
      "-z",
      base,
      head,
    ).split("\0");
    const changes = [];
    for (let index = 0; index + 1 < entries.length; index += 2) {
      changes.push({ status: entries[index], path: entries[index + 1] });
    }
    const adrChanges = changes.filter(({ path }) => isAdrPath(path));
    const additions = adrChanges
      .filter(({ status }) => status === "A")
      .map(({ path }) => path);
    for (const { path, status } of adrChanges) {
      const merged = mergedAdrs.has(path);
      if (
        (adopted("immutable-history") && merged) ||
        (adopted("supersession-only") && additions.length > 0 && status !== "A")
      ) {
        const superseded =
          additions.length === 1 &&
          status === "M" &&
          isSupersession(
            blob(base, path),
            blob(head, path),
            path,
            additions[0],
          );
        if (!superseded)
          violations.push({
            code:
              adopted("immutable-history") && merged
                ? "squash-merged-adr-immutable"
                : "squash-other-adr-supersession-only",
            head,
            path,
          });
      }
    }
    if (adopted("one-adr") && additions.length > 1)
      violations.push({
        code: "squash-multiple-new-adrs",
        head,
        paths: additions,
      });
    if (
      adopted("pairing") &&
      adrChanges.length > 0 &&
      !hasArchitectureUpdate(base, head, changes)
    )
      violations.push({ code: "squash-architecture-update-required", head });
  }
  console.log(
    JSON.stringify({
      schema_version: 1,
      base,
      head,
      trunk,
      delivery,
      checked_commits: commits,
      status: "checked",
      rules,
      checks: [
        "history-resolved",
        ...(adopted("pairing")
          ? ["same-commit-architecture-content-change"]
          : []),
        ...(adopted("one-adr") ? ["one-new-adr-per-commit"] : []),
        ...(adopted("immutable-history") ? ["merged-adr-immutability"] : []),
        ...(adopted("supersession-only")
          ? ["other-adr-supersession-only"]
          : []),
        ...(delivery === "squash"
          ? [
              ...(adopted("one-adr") ? ["squash-adr-count"] : []),
              ...(adopted("pairing")
                ? ["squash-architecture-content-change"]
                : []),
              ...(adopted("immutable-history")
                ? ["squash-merged-adr-immutability"]
                : []),
              ...(adopted("supersession-only")
                ? ["squash-other-adr-supersession-only"]
                : []),
            ]
          : []),
      ],
      violations,
      review_required: [
        "adopted-project-policy-and-overrides",
        ...(adopted("pairing") ? ["substantive-correspondence"] : []),
        "additional-semantic-review-only-if-adopted",
      ],
    }),
  );
  if (violations.length > 0) process.exitCode = 1;
} catch (error) {
  console.error(`architecture_policy.input_error ${error.message}`);
  process.exitCode = 2;
}
