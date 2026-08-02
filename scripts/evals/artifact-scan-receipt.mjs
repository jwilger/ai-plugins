#!/usr/bin/env node
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

const [command, receiptPath, ...artifacts] = process.argv.slice(2);

function fail() {
  console.error("provider eval artifact scan receipt is invalid");
  process.exit(2);
}

function artifactRecord(artifact) {
  const resolved = path.resolve(artifact);
  const stat = fs.lstatSync(resolved, { throwIfNoEntry: false });
  if (
    !stat ||
    !stat.isFile() ||
    stat.isSymbolicLink() ||
    stat.nlink !== 1 ||
    (stat.mode & 0o077) !== 0
  ) {
    fail();
  }
  const bytes = fs.readFileSync(resolved);
  return {
    name: path.basename(resolved),
    bytes: bytes.length,
    sha256: crypto.createHash("sha256").update(bytes).digest("hex"),
  };
}

if (
  !["write", "verify"].includes(command) ||
  !receiptPath ||
  artifacts.length === 0 ||
  new Set(artifacts.map((artifact) => path.basename(artifact))).size !==
    artifacts.length
) {
  fail();
}

const records = artifacts
  .map(artifactRecord)
  .sort((left, right) => left.name.localeCompare(right.name));

if (command === "write") {
  const resolvedReceipt = path.resolve(receiptPath);
  const temporary = `${resolvedReceipt}.${process.pid}.tmp`;
  try {
    fs.writeFileSync(
      temporary,
      `${JSON.stringify({ version: 1, artifacts: records })}\n`,
      { mode: 0o600 },
    );
    fs.renameSync(temporary, resolvedReceipt);
    fs.chmodSync(resolvedReceipt, 0o600);
  } finally {
    fs.rmSync(temporary, { force: true });
  }
  process.exit(0);
}

let receipt;
try {
  const stat = fs.lstatSync(receiptPath, { throwIfNoEntry: false });
  if (
    !stat ||
    !stat.isFile() ||
    stat.isSymbolicLink() ||
    stat.nlink !== 1 ||
    (stat.mode & 0o077) !== 0
  ) {
    fail();
  }
  receipt = JSON.parse(fs.readFileSync(receiptPath, "utf8"));
} catch {
  fail();
}
if (
  receipt?.version !== 1 ||
  JSON.stringify(receipt.artifacts) !== JSON.stringify(records)
) {
  fail();
}
