#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import path from "node:path";
import process from "node:process";
import { pathToFileURL } from "node:url";

const root = path.resolve(import.meta.dirname, "..");
const breakingHeader = /^[a-z][a-z0-9-]*(?:\([^\r\n)]+\))?!:/i;
const featureHeader = /^feat(?:\([^\r\n)]+\))?:/i;
const breakingFooter = /^breaking(?: |-)change:\s*\S/im;

export function determineBump(messages) {
  if (!Array.isArray(messages) || messages.length === 0)
    throw new Error("development_system.release_commits_empty");
  const breaking = messages.some((message) => {
    const normalized = String(message).replaceAll("\r\n", "\n").trim();
    const [header = ""] = normalized.split("\n", 1);
    return breakingHeader.test(header) || breakingFooter.test(normalized);
  });
  const features = messages.some((message) => {
    const [header = ""] = String(message)
      .replaceAll("\r\n", "\n")
      .trim()
      .split("\n", 1);
    return featureHeader.test(header);
  });
  return {
    bump: breaking ? "major" : features ? "minor" : "patch",
    breaking,
    features,
  };
}

function argument(name) {
  const index = process.argv.indexOf(name);
  return index >= 0 ? process.argv[index + 1] : undefined;
}

function git(args) {
  const result = spawnSync("git", args, { cwd: root, encoding: "utf8" });
  if (result.status !== 0)
    throw new Error(
      `development_system.release_git_failed args=${JSON.stringify(args)} status=${result.status}\n${result.stderr}`,
    );
  return result.stdout.trim();
}

function commitsInRange(from, to) {
  const result = spawnSync(
    "git",
    ["log", "--format=%B%x00", `${from}..${to}`],
    { cwd: root, encoding: "utf8" },
  );
  if (result.status !== 0)
    throw new Error(
      `development_system.release_history_failed from=${from} to=${to} status=${result.status}\n${result.stderr}`,
    );
  return result.stdout
    .split("\0")
    .map((message) => message.trim())
    .filter(Boolean);
}

function automaticBase(currentVersion, to) {
  if (!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(currentVersion))
    throw new Error("development_system.release_version_invalid");
  const currentTag = `development-system-v${currentVersion}`;
  const matchingTag = git(["tag", "--list", currentTag, "--merged", to]);
  if (matchingTag === currentTag)
    return { from: currentTag, pendingVersionCommit: false };

  const versionCommit = git([
    "log",
    "-1",
    "--format=%H%x00%s",
    to,
    "--",
    "plugins/development-system/package.json",
  ]);
  const separator = versionCommit.indexOf("\0");
  const oid = separator >= 0 ? versionCommit.slice(0, separator) : "";
  const subject = separator >= 0 ? versionCommit.slice(separator + 1) : "";
  if (
    /^[0-9a-f]{40}$/.test(oid) &&
    subject === `chore(release): development-system v${currentVersion}`
  )
    return { from: oid, pendingVersionCommit: true };

  return {
    from: git([
      "describe",
      "--tags",
      "--match",
      "development-system-v[0-9]*",
      "--abbrev=0",
      to,
    ]),
    pendingVersionCommit: false,
  };
}

const invokedDirectly =
  process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href;
if (invokedDirectly) {
  const explicitFrom = argument("--from");
  const currentVersion = argument("--current-version");
  const to = argument("--to");
  if ((!explicitFrom && !currentVersion) || !to) {
    console.error(
      "usage: determine-development-system-bump.mjs (--from <release-tag>|--current-version <semver>) --to <commit>",
    );
    process.exit(2);
  }
  const base = explicitFrom
    ? { from: explicitFrom, pendingVersionCommit: false }
    : automaticBase(currentVersion, to);
  const messages = commitsInRange(base.from, to);
  const result =
    messages.length === 0
      ? { bump: "current", breaking: false, features: false }
      : determineBump(messages);
  process.stdout.write(
    `${JSON.stringify({ ...result, commits: messages.length, from: base.from, to, pendingVersionCommit: base.pendingVersionCommit })}\n`,
  );
}
