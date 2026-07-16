import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

const maximumManifestBytes = 1024 * 1024;
const maximumStoreEntries = 8_192;
const nixStoreRootPattern =
  /^\/nix\/store\/[0-9abcdfghijklmnpqrsvwxyz]{32}-[^/\u0000-\u001f\u007f]+$/u;
const runRootMarkerContents = "ai-plugins downstream code-quality run root\n";

export class NixStoreClosureError extends Error {
  constructor(code) {
    super(code);
    this.code = code;
  }
}

function fail(code) {
  throw new NixStoreClosureError(code);
}

function storeRootForPath(candidate) {
  if (!path.isAbsolute(candidate) || path.resolve(candidate) !== candidate) {
    fail("nix-store-closure-incomplete");
  }
  const relative = path.relative("/nix/store", candidate);
  const [root, ...rest] = relative.split(path.sep);
  const storeRoot = path.join("/nix/store", root || "");
  if (
    rest.length === 0 ||
    !nixStoreRootPattern.test(storeRoot) ||
    !candidate.startsWith(`${storeRoot}${path.sep}`)
  ) {
    fail("nix-store-closure-incomplete");
  }
  return storeRoot;
}

function requireOwnedRunRoot(manifest) {
  const root = path.dirname(manifest);
  const marker = path.join(root, ".ai-plugins-code-quality-run-root");
  try {
    const rootMetadata = fs.lstatSync(root);
    const markerMetadata = fs.lstatSync(marker);
    if (
      fs.realpathSync(root) !== root ||
      !rootMetadata.isDirectory() ||
      rootMetadata.isSymbolicLink() ||
      rootMetadata.uid !== process.getuid() ||
      (rootMetadata.mode & 0o777) !== 0o700 ||
      fs.realpathSync(marker) !== marker ||
      !markerMetadata.isFile() ||
      markerMetadata.isSymbolicLink() ||
      markerMetadata.nlink !== 1 ||
      markerMetadata.uid !== process.getuid() ||
      (markerMetadata.mode & 0o777) !== 0o600 ||
      fs.readFileSync(marker, "utf8") !== runRootMarkerContents
    ) {
      fail("nix-store-closure-run-root-invalid");
    }
  } catch (error) {
    if (error instanceof NixStoreClosureError) throw error;
    fail("nix-store-closure-run-root-invalid");
  }
}

function readManifest(candidate) {
  if (!candidate || !path.isAbsolute(candidate)) {
    fail("nix-store-closure-missing");
  }
  requireOwnedRunRoot(candidate);
  let descriptor;
  try {
    const before = fs.lstatSync(candidate);
    if (
      path.resolve(candidate) !== candidate ||
      fs.realpathSync(candidate) !== candidate ||
      !before.isFile() ||
      before.isSymbolicLink() ||
      before.nlink !== 1 ||
      before.uid !== process.getuid() ||
      (before.mode & 0o777) !== 0o400 ||
      before.size < 45 ||
      before.size > maximumManifestBytes
    ) {
      fail("nix-store-closure-unsafe");
    }
    descriptor = fs.openSync(
      candidate,
      fs.constants.O_RDONLY | fs.constants.O_NOFOLLOW,
    );
    const opened = fs.fstatSync(descriptor);
    if (
      !opened.isFile() ||
      opened.nlink !== 1 ||
      opened.uid !== before.uid ||
      opened.dev !== before.dev ||
      opened.ino !== before.ino ||
      opened.size !== before.size ||
      (opened.mode & 0o777) !== 0o400
    ) {
      fail("nix-store-closure-unsafe");
    }
    return fs.readFileSync(descriptor);
  } catch (error) {
    if (error instanceof NixStoreClosureError) throw error;
    fail("nix-store-closure-unsafe");
  } finally {
    if (descriptor !== undefined) {
      try {
        fs.closeSync(descriptor);
      } catch {
        // Captured immutable bytes no longer depend on the descriptor.
      }
    }
  }
}

export function validatedNixStoreClosure({
  expectedSha256,
  manifest,
  requiredPaths,
}) {
  if (!/^[0-9a-f]{64}$/u.test(expectedSha256 ?? "")) {
    fail("nix-store-closure-sha256-invalid");
  }
  const bytes = readManifest(manifest);
  const observedSha256 = crypto
    .createHash("sha256")
    .update(bytes)
    .digest("hex");
  if (observedSha256 !== expectedSha256) {
    fail("nix-store-closure-sha256-mismatch");
  }

  let contents;
  try {
    contents = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
  } catch {
    fail("nix-store-closure-invalid");
  }
  if (!contents.endsWith("\n") || contents.includes("\0")) {
    fail("nix-store-closure-invalid");
  }
  const entries = contents.slice(0, -1).split("\n");
  if (
    entries.length === 0 ||
    entries.length > maximumStoreEntries ||
    entries.some((entry, index) => index > 0 && entry <= entries[index - 1])
  ) {
    fail("nix-store-closure-invalid");
  }

  for (const entry of entries) {
    try {
      const metadata = fs.lstatSync(entry);
      if (
        path.dirname(entry) !== "/nix/store" ||
        !nixStoreRootPattern.test(entry) ||
        fs.realpathSync(entry) !== entry ||
        !metadata.isDirectory() ||
        metadata.isSymbolicLink() ||
        (metadata.mode & 0o022) !== 0
      ) {
        fail("nix-store-closure-invalid");
      }
    } catch (error) {
      if (error instanceof NixStoreClosureError) throw error;
      fail("nix-store-closure-invalid");
    }
  }

  const entrySet = new Set(entries);
  for (const requiredPath of requiredPaths) {
    if (!entrySet.has(storeRootForPath(requiredPath))) {
      fail("nix-store-closure-incomplete");
    }
  }
  return entries;
}

export function nixStoreMountArgs(entries) {
  return [
    "--dir",
    "/nix",
    "--dir",
    "/nix/store",
    ...entries.flatMap((entry) => ["--ro-bind", entry, entry]),
  ];
}
