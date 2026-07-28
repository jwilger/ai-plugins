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

const invokedDirectly =
  process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href;
if (invokedDirectly) {
  const from = argument("--from");
  const to = argument("--to");
  if (!from || !to) {
    console.error(
      "usage: determine-development-system-bump.mjs --from <release-tag> --to <commit>",
    );
    process.exit(2);
  }
  const messages = commitsInRange(from, to);
  const result = determineBump(messages);
  process.stdout.write(
    `${JSON.stringify({ ...result, commits: messages.length, from, to })}\n`,
  );
}
