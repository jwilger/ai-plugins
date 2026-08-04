#!/usr/bin/env node
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const minimumExactSecretBytes = 8;
const maximumExactSecretBytes = 16 * 1024;
const maximumFileBytes = 64 * 1024 * 1024;
const maximumTotalBytes = 256 * 1024 * 1024;
const maximumEntries = 10_000;
const maximumDirectoryEntries = 512;
const maximumDepth = 32;
const maximumSymlinkTargetBytes = 4 * 1024;
const environmentNamePattern = /^[A-Z][A-Z0-9_]{0,63}$/;
const identifierPattern = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;
const runtimeMarkerName = ".ai-plugins-code-quality-runtime-root";
const runtimeMarkerContents =
  "ai-plugins downstream code-quality runtime root\n";
const runtimeHelperNames = new Set([
  "apply_patch",
  "applypatch",
  "codex-execve-wrapper",
  "codex-linux-sandbox",
]);
const codexArg0Pattern = /^codex-arg0[A-Za-z0-9._-]{1,128}$/;
const genericSecretPatterns = [
  /\bgh[pousr]_[A-Za-z0-9]{20,}\b/,
  /\bgithub_pat_[A-Za-z0-9_]{20,}\b/,
  /\bsk-(?:proj-|svcacct-)?[A-Za-z0-9_-]{20,}\b/,
  /\bsk-ant-(?:api\d{2}-)?[A-Za-z0-9_-]{20,}\b/,
  /\b(?:authorization\s*[:=]\s*)?bearer\s+[A-Za-z0-9._~+/-]{16,}={0,2}/i,
  /-----BEGIN (?:RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----/,
  /["'](?:access[_-]?token|refresh[_-]?token|api[_-]?key|apikey|client[_-]?secret|authorization|token)["']\s*:\s*["'][^"'\r\n]{8,}["']/i,
  /(?:^|\s)(?:_authToken|authToken)\s*=\s*[^\s]{8,}/m,
];

class ScanFailure extends Error {
  constructor(code, status = 2) {
    super(code);
    this.code = code;
    this.status = status;
  }
}

function parseArguments(argv) {
  const environmentNames = [];
  const inputs = [];
  let exactOnly = false;
  let positionalOnly = false;
  let profile;
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (!positionalOnly && argument === "--") {
      positionalOnly = true;
    } else if (!positionalOnly && argument === "--exact-only") {
      if (exactOnly) throw new ScanFailure("invalid-arguments");
      exactOnly = true;
    } else if (!positionalOnly && argument === "--profile") {
      const value = argv[index + 1];
      if (profile || value !== "codex-runtime") {
        throw new ScanFailure("invalid-arguments");
      }
      profile = value;
      index += 1;
    } else if (!positionalOnly && argument === "--secret-env") {
      const name = argv[index + 1];
      if (!name || !environmentNamePattern.test(name)) {
        throw new ScanFailure("invalid-arguments");
      }
      environmentNames.push(name);
      index += 1;
    } else if (!positionalOnly && argument.startsWith("-")) {
      throw new ScanFailure("invalid-arguments");
    } else {
      inputs.push(argument);
    }
  }
  if (
    inputs.length === 0 ||
    inputs.length > 32 ||
    environmentNames.length > 32 ||
    new Set(environmentNames).size !== environmentNames.length
  ) {
    throw new ScanFailure("invalid-arguments");
  }
  if (profile && (!exactOnly || inputs.length !== 1)) {
    throw new ScanFailure("invalid-arguments");
  }
  return { environmentNames, exactOnly, inputs, profile };
}

function exactSecrets(environmentNames) {
  const secrets = [];
  for (const name of environmentNames) {
    const value = process.env[name];
    if (!value) continue;
    const bytes = Buffer.from(value);
    if (bytes.length < minimumExactSecretBytes) continue;
    if (bytes.length > maximumExactSecretBytes) {
      throw new ScanFailure("invalid-secret-value");
    }
    secrets.push(bytes);
  }
  return secrets;
}

function assertPrivateMode(stat) {
  if ((stat.mode & 0o077) !== 0) {
    throw new ScanFailure("input-not-private");
  }
}

function sameIdentity(first, second) {
  return first.dev === second.dev && first.ino === second.ino;
}

function sameSnapshot(first, second) {
  return (
    sameIdentity(first, second) &&
    first.mode === second.mode &&
    first.nlink === second.nlink &&
    first.size === second.size &&
    first.mtimeMs === second.mtimeMs &&
    first.ctimeMs === second.ctimeMs
  );
}

function isStrictDescendant(ancestor, descendant) {
  const relative = path.relative(ancestor, descendant);
  return (
    relative !== "" &&
    relative !== ".." &&
    !relative.startsWith(`..${path.sep}`) &&
    !path.isAbsolute(relative)
  );
}

function assertRuntimeProfileRoot(input, stat) {
  let temporaryRoot;
  try {
    temporaryRoot = fs.realpathSync(os.tmpdir());
  } catch {
    throw new ScanFailure("runtime-profile-invalid");
  }
  if (
    !stat.isDirectory() ||
    stat.isSymbolicLink() ||
    (stat.mode & 0o077) !== 0 ||
    !isStrictDescendant(temporaryRoot, input)
  ) {
    throw new ScanFailure("runtime-profile-invalid");
  }
}

function runtimeMutableExpectation(relative) {
  const segments = relative.split("/").filter(Boolean);
  const codexHomeIndex = segments.findIndex(
    (segment, index) =>
      segment === "codex-home" && segments[index + 1] === "tmp",
  );
  if (codexHomeIndex === -1) return undefined;
  if (
    codexHomeIndex !== 3 ||
    !identifierPattern.test(segments[0] || "") ||
    !/^sample-(?:[1-9]|10)$/.test(segments[1] || "") ||
    !identifierPattern.test(segments[2] || "")
  ) {
    return "invalid";
  }
  const mutable = segments.slice(4);
  if (mutable.length === 1) return "directory";
  if (mutable.length === 2) {
    return mutable[1] === "arg0" ? "directory" : "invalid";
  }
  if (mutable.length === 3) {
    return mutable[1] === "arg0" && codexArg0Pattern.test(mutable[2])
      ? "directory"
      : "invalid";
  }
  if (
    mutable.length === 4 &&
    mutable[1] === "arg0" &&
    codexArg0Pattern.test(mutable[2])
  ) {
    if (mutable[3] === ".lock") return "file";
    if (runtimeHelperNames.has(mutable[3])) return "symlink";
  }
  return "invalid";
}

function assertRuntimeProfileEntry(relative, stat) {
  const expectation = runtimeMutableExpectation(relative);
  if (expectation === "invalid") {
    throw new ScanFailure("runtime-profile-invalid");
  }
  if (
    (expectation === "directory" && !stat.isDirectory()) ||
    (expectation === "file" && !stat.isFile()) ||
    (expectation === "symlink" && !stat.isSymbolicLink()) ||
    (!expectation && stat.isSymbolicLink())
  ) {
    throw new ScanFailure("runtime-profile-invalid");
  }
}

function openPinned(candidate, directory, expectedStat) {
  let flags = fs.constants.O_RDONLY | fs.constants.O_NOFOLLOW;
  if (directory) flags |= fs.constants.O_DIRECTORY;
  const before = fs.lstatSync(candidate, { throwIfNoEntry: false });
  if (!before) throw new ScanFailure("input-unreadable");
  if (before.isSymbolicLink()) throw new ScanFailure("input-symlink");
  if (expectedStat && !sameIdentity(before, expectedStat)) {
    throw new ScanFailure("input-changed");
  }
  let descriptor;
  try {
    descriptor = fs.openSync(candidate, flags);
  } catch {
    throw new ScanFailure("input-unreadable");
  }
  const after = fs.fstatSync(descriptor);
  if (!sameIdentity(before, after)) {
    fs.closeSync(descriptor);
    throw new ScanFailure("input-changed");
  }
  return { descriptor, stat: after };
}

function containsExactSecret(bytes, secrets) {
  return secrets.some((secret) => bytes.indexOf(secret) !== -1);
}

function containsGenericSecret(bytes) {
  const text = bytes.toString("latin1");
  return genericSecretPatterns.some((pattern) => pattern.test(text));
}

function assertBytesContainNoSecret(bytes, secrets, exactOnly) {
  if (
    containsExactSecret(bytes, secrets) ||
    (!exactOnly && containsGenericSecret(bytes))
  ) {
    throw new ScanFailure("secret-detected", 1);
  }
}

function scanInputs(inputs, secrets, exactOnly, profile) {
  const state = { bytes: 0, entries: 0, runtimeMarkerSeen: false };

  function scanEntry(candidate, depth, expectedStat, relative = "") {
    if (depth > maximumDepth) throw new ScanFailure("input-too-deep");
    state.entries += 1;
    if (state.entries > maximumEntries) {
      throw new ScanFailure("input-too-many-entries");
    }

    const stat = fs.lstatSync(candidate, { throwIfNoEntry: false });
    if (!stat) throw new ScanFailure("input-unreadable");
    if (expectedStat && !sameIdentity(stat, expectedStat)) {
      throw new ScanFailure("input-changed");
    }
    if (profile === "codex-runtime") {
      assertRuntimeProfileEntry(relative, stat);
    } else if (stat.isSymbolicLink()) {
      throw new ScanFailure("input-symlink");
    }

    if (stat.isSymbolicLink()) {
      if (stat.nlink !== 1) throw new ScanFailure("input-hard-linked");
      let target;
      try {
        target = fs.readlinkSync(candidate, { encoding: "buffer" });
      } catch {
        throw new ScanFailure("input-unreadable");
      }
      if (target.length === 0 || target.length > maximumSymlinkTargetBytes) {
        throw new ScanFailure("runtime-profile-invalid");
      }
      state.bytes += target.length;
      if (state.bytes > maximumTotalBytes) {
        throw new ScanFailure("input-tree-too-large");
      }
      assertBytesContainNoSecret(target, secrets, exactOnly);
      const after = fs.lstatSync(candidate, { throwIfNoEntry: false });
      if (!after || !sameSnapshot(stat, after)) {
        throw new ScanFailure("input-changed");
      }
      return;
    }
    assertPrivateMode(stat);

    if (stat.isDirectory()) {
      const { descriptor, stat: pinnedStat } = openPinned(
        candidate,
        true,
        stat,
      );
      try {
        if (!pinnedStat.isDirectory()) {
          throw new ScanFailure("input-changed");
        }
        let directory;
        try {
          directory = fs.opendirSync(`/proc/self/fd/${descriptor}`);
        } catch {
          throw new ScanFailure("input-unreadable");
        }
        try {
          let directoryEntries = 0;
          let entry;
          while ((entry = directory.readSync())) {
            directoryEntries += 1;
            if (directoryEntries > maximumDirectoryEntries) {
              throw new ScanFailure("input-directory-too-large");
            }
            assertBytesContainNoSecret(
              Buffer.from(entry.name),
              secrets,
              exactOnly,
            );
            const childRelative = relative
              ? `${relative}/${entry.name}`
              : entry.name;
            scanEntry(
              `/proc/self/fd/${descriptor}/${entry.name}`,
              depth + 1,
              undefined,
              childRelative,
            );
          }
        } finally {
          directory.closeSync();
        }
        if (!sameSnapshot(pinnedStat, fs.fstatSync(descriptor))) {
          throw new ScanFailure("input-changed");
        }
      } finally {
        fs.closeSync(descriptor);
      }
      return;
    }

    if (!stat.isFile()) throw new ScanFailure("input-special-entry");
    if (stat.nlink !== 1) throw new ScanFailure("input-hard-linked");
    if (stat.size > maximumFileBytes) {
      throw new ScanFailure("input-file-too-large");
    }
    state.bytes += stat.size;
    if (state.bytes > maximumTotalBytes) {
      throw new ScanFailure("input-tree-too-large");
    }
    const { descriptor, stat: pinnedStat } = openPinned(candidate, false, stat);
    try {
      if (
        !pinnedStat.isFile() ||
        pinnedStat.nlink !== 1 ||
        pinnedStat.size !== stat.size
      ) {
        throw new ScanFailure("input-changed");
      }
      let bytes;
      try {
        bytes = fs.readFileSync(descriptor);
      } catch {
        throw new ScanFailure("input-unreadable");
      }
      if (bytes.length !== pinnedStat.size) {
        throw new ScanFailure("input-changed");
      }
      assertBytesContainNoSecret(bytes, secrets, exactOnly);
      if (profile === "codex-runtime" && relative === runtimeMarkerName) {
        if (!bytes.equals(Buffer.from(runtimeMarkerContents))) {
          throw new ScanFailure("runtime-profile-invalid");
        }
        state.runtimeMarkerSeen = true;
      }
      if (!sameSnapshot(pinnedStat, fs.fstatSync(descriptor))) {
        throw new ScanFailure("input-changed");
      }
    } finally {
      fs.closeSync(descriptor);
    }
  }

  for (const input of inputs) {
    if (!path.isAbsolute(input) || path.resolve(input) !== input) {
      throw new ScanFailure("input-path-invalid");
    }
    for (const segment of input.split(path.sep).filter(Boolean)) {
      assertBytesContainNoSecret(Buffer.from(segment), secrets, exactOnly);
    }
    const stat = fs.lstatSync(input, { throwIfNoEntry: false });
    if (!stat) {
      throw new ScanFailure("input-path-invalid");
    }
    if (stat.isSymbolicLink()) throw new ScanFailure("input-symlink");
    if (fs.realpathSync(input) !== input) {
      throw new ScanFailure("input-path-invalid");
    }
    if (profile === "codex-runtime") {
      assertRuntimeProfileRoot(input, stat);
    }
    scanEntry(input, 0, stat);
  }
  if (profile === "codex-runtime" && !state.runtimeMarkerSeen) {
    throw new ScanFailure("runtime-profile-invalid");
  }
}

function main() {
  try {
    const { environmentNames, exactOnly, inputs, profile } = parseArguments(
      process.argv.slice(2),
    );
    scanInputs(inputs, exactSecrets(environmentNames), exactOnly, profile);
    process.stdout.write("secret-scan:clean\n");
  } catch (error) {
    const failure =
      error instanceof ScanFailure
        ? error
        : new ScanFailure("internal-failure");
    process.stderr.write(`secret-scan:${failure.code}\n`);
    process.exitCode = failure.status;
  }
}

main();
