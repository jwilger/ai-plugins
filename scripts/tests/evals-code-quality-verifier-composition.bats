#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  COMPOSITION="$ROOT/evals/benchmarks/downstream-code-quality/verifiers/verifier-composition.mjs"
  TEST_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/ai-plugins-verifier-composition.XXXXXX")"
}

teardown() {
  rm -rf "$TEST_ROOT"
}

@test "verifier composition binds the exact scoring-gate file set" {
  run node --input-type=module - "$COMPOSITION" <<'NODE'
import assert from "node:assert/strict";

const { verifierComposition, verifierCompositionFiles } = await import(
  process.argv[2]
);
const expected = [
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
];

assert.deepEqual(verifierCompositionFiles, expected);
const observed = verifierComposition();
assert.equal(observed.schemaVersion, 1);
assert.deepEqual(observed.files, expected);
assert.match(observed.sha256, /^[0-9a-f]{64}$/u);
NODE

  [ "$status" -eq 0 ]
  [ -z "$output" ]
}

@test "verifier composition rejects a symlinked member" {
  run node --input-type=module - "$COMPOSITION" "$ROOT" "$TEST_ROOT" <<'NODE'
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";

const { VerifierCompositionError, verifierComposition, verifierCompositionFiles } =
  await import(process.argv[2]);
const sourceRoot = process.argv[3];
const mirrorRoot = process.argv[4];
const symlinked =
  "evals/benchmarks/downstream-code-quality/verifiers/expense-report.mjs";

for (const relative of verifierCompositionFiles) {
  const destination = path.join(mirrorRoot, relative);
  fs.mkdirSync(path.dirname(destination), { recursive: true });
  if (relative === symlinked) {
    fs.symlinkSync(path.join(sourceRoot, relative), destination);
  } else {
    fs.copyFileSync(path.join(sourceRoot, relative), destination);
  }
}

assert.throws(
  () => verifierComposition(mirrorRoot),
  (error) =>
    error instanceof VerifierCompositionError &&
    error.code === "verifier-composition-invalid",
);
NODE

  [ "$status" -eq 0 ]
  [ -z "$output" ]
}
