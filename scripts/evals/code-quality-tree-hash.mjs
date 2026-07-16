import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

const defaultLimits = Object.freeze({
  directories: 128,
  fileBytes: 512 * 1024,
  files: 128,
  pathBytes: 512,
  totalBytes: 8 * 1024 * 1024,
});
const treeHashDomain = "ai-plugins-code-quality-tree-v1";

function fail(code) {
  throw new Error(code);
}

function frame(value) {
  const bytes = Buffer.isBuffer(value) ? value : Buffer.from(value, "utf8");
  const size = Buffer.alloc(8);
  size.writeBigUInt64BE(BigInt(bytes.byteLength));
  return [size, bytes];
}

export function sha256FramedEntries(domain, entries) {
  const hash = crypto.createHash("sha256");
  for (const part of frame(domain)) hash.update(part);
  const ordered = [...entries].sort(([left], [right]) =>
    Buffer.from(left).compare(Buffer.from(right)),
  );
  for (const [label, contents] of ordered) {
    for (const part of frame(label)) hash.update(part);
    for (const part of frame(contents)) hash.update(part);
  }
  return hash.digest("hex");
}

export function sha256TreeSnapshot(files) {
  if (!(files instanceof Map)) fail("tree-snapshot-invalid");
  return sha256FramedEntries(treeHashDomain, files);
}

function descriptorPath(descriptor, name) {
  return name === undefined
    ? `/proc/self/fd/${descriptor}`
    : `/proc/self/fd/${descriptor}/${name}`;
}

function openDirectory(candidate) {
  let descriptor;
  try {
    descriptor = fs.openSync(
      candidate,
      fs.constants.O_RDONLY |
        fs.constants.O_DIRECTORY |
        fs.constants.O_NOFOLLOW,
    );
    if (!fs.fstatSync(descriptor).isDirectory()) fail("tree-directory-invalid");
    return descriptor;
  } catch (error) {
    if (descriptor !== undefined) {
      try {
        fs.closeSync(descriptor);
      } catch {
        // The rejected descriptor is never reused.
      }
    }
    if (error instanceof Error && error.message.startsWith("tree-")) {
      throw error;
    }
    fail("tree-directory-invalid");
  }
}

function readRegularFile(candidate, maximumBytes) {
  let descriptor;
  try {
    descriptor = fs.openSync(
      candidate,
      fs.constants.O_RDONLY | fs.constants.O_NOFOLLOW | fs.constants.O_NONBLOCK,
    );
    const stat = fs.fstatSync(descriptor);
    if (!stat.isFile() || stat.nlink !== 1 || stat.size > maximumBytes) {
      fail("tree-file-invalid");
    }
    const contents = fs.readFileSync(descriptor);
    if (contents.byteLength > maximumBytes) fail("tree-file-invalid");
    return contents;
  } catch (error) {
    if (error instanceof Error && error.message.startsWith("tree-")) {
      throw error;
    }
    fail("tree-file-invalid");
  } finally {
    if (descriptor !== undefined) {
      try {
        fs.closeSync(descriptor);
      } catch {
        // Captured bytes no longer depend on the descriptor.
      }
    }
  }
}

function directoryEntries(descriptor, maximum) {
  let handle;
  const names = [];
  try {
    handle = fs.opendirSync(descriptorPath(descriptor));
    while (true) {
      const entry = handle.readSync();
      if (!entry) break;
      names.push(entry.name);
      if (names.length > maximum) fail("tree-entry-limit");
    }
  } catch (error) {
    if (error instanceof Error && error.message.startsWith("tree-")) {
      throw error;
    }
    fail("tree-directory-invalid");
  } finally {
    if (handle) {
      try {
        handle.closeSync();
      } catch {
        // Captured entry names no longer depend on the handle.
      }
    }
  }
  return names.sort((left, right) =>
    Buffer.from(left).compare(Buffer.from(right)),
  );
}

export function snapshotRegularTree(
  root,
  configuredLimits = {},
  { ignoredEntryNames = [], ignoredRootEntries = [] } = {},
) {
  if (process.platform !== "linux") fail("tree-snapshot-requires-linux");
  const limits = { ...defaultLimits, ...configuredLimits };
  for (const value of Object.values(limits)) {
    if (!Number.isSafeInteger(value) || value < 1) fail("tree-limits-invalid");
  }
  const validIgnoredEntries = (entries) =>
    Array.isArray(entries) &&
    entries.every(
      (entry) =>
        typeof entry === "string" &&
        entry.length > 0 &&
        entry !== "." &&
        entry !== ".." &&
        !entry.includes(path.sep),
    ) &&
    new Set(entries).size === entries.length;
  if (
    !validIgnoredEntries(ignoredRootEntries) ||
    !validIgnoredEntries(ignoredEntryNames)
  ) {
    fail("tree-ignored-entries-invalid");
  }
  const ignoredAtRoot = new Set(ignoredRootEntries);
  const ignoredEverywhere = new Set(ignoredEntryNames);

  const files = new Map();
  const directories = new Set([""]);
  let directoryCount = 0;
  let totalBytes = 0;
  const rootDescriptor = openDirectory(root);

  function visit(directoryDescriptor, relativeDirectory) {
    directoryCount += 1;
    if (directoryCount > limits.directories) fail("tree-directory-limit");
    for (const name of directoryEntries(
      directoryDescriptor,
      limits.directories + limits.files,
    )) {
      if (
        ignoredEverywhere.has(name) ||
        (relativeDirectory === "" && ignoredAtRoot.has(name))
      ) {
        continue;
      }
      const relative = relativeDirectory
        ? path.join(relativeDirectory, name)
        : name;
      if (Buffer.byteLength(relative) > limits.pathBytes) {
        fail("tree-path-limit");
      }
      const candidate = descriptorPath(directoryDescriptor, name);
      let childDirectory;
      try {
        childDirectory = fs.openSync(
          candidate,
          fs.constants.O_RDONLY |
            fs.constants.O_DIRECTORY |
            fs.constants.O_NOFOLLOW,
        );
      } catch (error) {
        if (!["ENOTDIR", "ELOOP"].includes(error.code)) {
          fail("tree-entry-invalid");
        }
      }
      if (childDirectory !== undefined) {
        try {
          if (!fs.fstatSync(childDirectory).isDirectory()) {
            fail("tree-directory-invalid");
          }
          directories.add(relative);
          visit(childDirectory, relative);
        } finally {
          try {
            fs.closeSync(childDirectory);
          } catch {
            // The completed traversal no longer depends on the descriptor.
          }
        }
        continue;
      }
      if (files.size >= limits.files) fail("tree-file-limit");
      const contents = readRegularFile(candidate, limits.fileBytes);
      totalBytes += contents.byteLength;
      if (totalBytes > limits.totalBytes) fail("tree-total-size-limit");
      files.set(relative, contents);
    }
  }

  try {
    visit(rootDescriptor, "");
  } finally {
    try {
      fs.closeSync(rootDescriptor);
    } catch {
      // Captured bytes no longer depend on the root descriptor.
    }
  }
  return Object.freeze({
    digest: sha256TreeSnapshot(files),
    directoryCount,
    directories: Object.freeze([...directories]),
    fileCount: files.size,
    files,
    totalBytes,
  });
}

export function writeTreeSnapshot(snapshot, destination) {
  if (!snapshot?.files || !(snapshot.files instanceof Map)) {
    fail("tree-snapshot-invalid");
  }
  fs.mkdirSync(destination, { mode: 0o700 });
  for (const [relative, contents] of snapshot.files) {
    if (
      path.isAbsolute(relative) ||
      relative === ".." ||
      relative.startsWith(`..${path.sep}`)
    ) {
      fail("tree-path-invalid");
    }
    const target = path.join(destination, relative);
    fs.mkdirSync(path.dirname(target), { mode: 0o700, recursive: true });
    fs.writeFileSync(target, contents, { flag: "wx", mode: 0o600 });
  }
}
