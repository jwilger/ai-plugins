import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

const compositionDomain =
  "ai-plugins-downstream-code-quality-verifier-composition-v1";
const maximumFileBytes = 4 * 1024 * 1024;
const maximumTotalBytes = 16 * 1024 * 1024;
const defaultRepositoryRoot = path.resolve(import.meta.dirname, "../../../..");

export const verifierCompositionFiles = Object.freeze([
  "evals/benchmarks/downstream-code-quality/assertions/expense-report.cjs",
  "evals/benchmarks/downstream-code-quality/benchmark-inputs.cjs",
  "evals/benchmarks/downstream-code-quality/benchmark.json",
  "evals/benchmarks/downstream-code-quality/cases.cjs",
  "evals/benchmarks/downstream-code-quality/fixtures/expense-report/.gitignore",
  "evals/benchmarks/downstream-code-quality/fixtures/expense-report/AGENTS.md",
  "evals/benchmarks/downstream-code-quality/fixtures/expense-report/Cargo.lock",
  "evals/benchmarks/downstream-code-quality/fixtures/expense-report/Cargo.toml",
  "evals/benchmarks/downstream-code-quality/fixtures/expense-report/src/main.rs",
  "evals/benchmarks/downstream-code-quality/fixtures/expense-report/tests/cli.rs",
  "evals/benchmarks/downstream-code-quality/manifest.cjs",
  "evals/benchmarks/downstream-code-quality/promptfooconfig.yaml",
  "evals/benchmarks/downstream-code-quality/runtime-manifest.cjs",
  "evals/benchmarks/downstream-code-quality/verifiers/expense-report.mjs",
  "evals/benchmarks/downstream-code-quality/verifiers/nix-store-closure.mjs",
  "evals/benchmarks/downstream-code-quality/verifiers/score-expense-report.mjs",
  "evals/benchmarks/downstream-code-quality/verifiers/verifier-composition.mjs",
  "scripts/evals/code-quality-runtime-contract.mjs",
  "scripts/evals/code-quality-runtime-evidence.mjs",
  "scripts/evals/code-quality-tree-hash.mjs",
]);

export class VerifierCompositionError extends Error {
  constructor(code = "verifier-composition-invalid") {
    super(code);
    this.code = code;
  }
}

function fail() {
  throw new VerifierCompositionError();
}

function framed(hash, bytes) {
  const length = Buffer.alloc(8);
  length.writeBigUInt64BE(BigInt(bytes.byteLength));
  hash.update(length).update(bytes);
}

function readPinnedFile(repositoryRoot, relative) {
  if (
    !relative ||
    relative.includes("\\") ||
    path.posix.normalize(relative) !== relative ||
    relative.startsWith("/") ||
    relative.split("/").includes("..")
  ) {
    fail();
  }
  const candidate = path.join(repositoryRoot, ...relative.split("/"));
  let descriptor;
  try {
    const before = fs.lstatSync(candidate);
    if (
      fs.realpathSync(candidate) !== candidate ||
      !before.isFile() ||
      before.isSymbolicLink() ||
      before.nlink !== 1 ||
      before.size > maximumFileBytes
    ) {
      fail();
    }
    descriptor = fs.openSync(
      candidate,
      fs.constants.O_RDONLY | fs.constants.O_NOFOLLOW,
    );
    const opened = fs.fstatSync(descriptor);
    if (
      !opened.isFile() ||
      opened.nlink !== 1 ||
      opened.dev !== before.dev ||
      opened.ino !== before.ino ||
      opened.size !== before.size ||
      opened.size > maximumFileBytes
    ) {
      fail();
    }
    const bytes = fs.readFileSync(descriptor);
    if (bytes.byteLength !== opened.size) fail();
    return bytes;
  } catch (error) {
    if (error instanceof VerifierCompositionError) throw error;
    fail();
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

export function verifierComposition(repositoryRoot = defaultRepositoryRoot) {
  let canonicalRoot;
  try {
    canonicalRoot = fs.realpathSync(repositoryRoot);
    if (
      !path.isAbsolute(repositoryRoot) ||
      path.resolve(repositoryRoot) !== repositoryRoot ||
      canonicalRoot !== repositoryRoot ||
      !fs.lstatSync(canonicalRoot).isDirectory()
    ) {
      fail();
    }
  } catch (error) {
    if (error instanceof VerifierCompositionError) throw error;
    fail();
  }
  if (
    verifierCompositionFiles.length === 0 ||
    verifierCompositionFiles.some(
      (relative, index) =>
        index > 0 && relative <= verifierCompositionFiles[index - 1],
    )
  ) {
    fail();
  }

  const hash = crypto.createHash("sha256");
  framed(hash, Buffer.from(compositionDomain));
  let totalBytes = 0;
  for (const relative of verifierCompositionFiles) {
    const bytes = readPinnedFile(canonicalRoot, relative);
    totalBytes += bytes.byteLength;
    if (totalBytes > maximumTotalBytes) fail();
    framed(hash, Buffer.from(relative));
    framed(hash, bytes);
  }
  return Object.freeze({
    files: verifierCompositionFiles,
    schemaVersion: 1,
    sha256: hash.digest("hex"),
  });
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  try {
    if (process.argv.length !== 3 || process.argv[2] !== "--sha256") fail();
    process.stdout.write(`${verifierComposition().sha256}\n`);
  } catch {
    process.stderr.write("verifier-composition-invalid\n");
    process.exitCode = 2;
  }
}
