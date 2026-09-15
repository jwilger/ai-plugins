#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";

function fail(message) {
  throw new Error(`model-routing verifier: ${message}`);
}

function parseArguments(argv) {
  const options = {};
  for (let index = 0; index < argv.length; index += 2) {
    const key = argv[index];
    const value = argv[index + 1];
    if (
      !value ||
      !["--specs", "--fixture", "--workspace", "--output"].includes(key)
    )
      fail("invalid arguments");
    options[key.slice(2)] =
      key === "--fixture" || key === "--output" ? value : path.resolve(value);
  }
  for (const name of ["specs", "fixture", "workspace", "output"]) {
    if (!options[name]) fail(`--${name} is required`);
  }
  return options;
}

function regularFiles(root, directory = root, result = {}) {
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const absolute = path.join(directory, entry.name);
    if (entry.isSymbolicLink()) fail("workspace contains a symlink");
    if (entry.isDirectory()) regularFiles(root, absolute, result);
    else if (entry.isFile())
      result[path.relative(root, absolute)] = fs.readFileSync(absolute, "utf8");
    else fail("workspace contains a special file");
  }
  return result;
}

function canonicalFileMap(files) {
  return Object.fromEntries(
    Object.entries(files).sort(([first], [second]) =>
      first.localeCompare(second),
    ),
  );
}

try {
  const options = parseArguments(process.argv.slice(2));
  const document = JSON.parse(fs.readFileSync(options.specs, "utf8"));
  const matches =
    document.fixtures?.filter(
      (fixture) => fixture.fixture_id === options.fixture,
    ) ?? [];
  if (matches.length !== 1) fail("fixture must resolve exactly once");
  const verification = matches[0].verification;
  if (
    verification?.kind !== "exact-workspace" ||
    !verification.expected_files
  ) {
    fail("fixture does not define exact-workspace verification");
  }
  const actual = canonicalFileMap(regularFiles(options.workspace));
  const expected = canonicalFileMap(verification.expected_files);
  const passed = JSON.stringify(actual) === JSON.stringify(expected);
  console.log(
    JSON.stringify({
      status: passed ? "pass" : "fail",
      method: "exact-workspace",
    }),
  );
  if (!passed) process.exitCode = 1;
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 2;
}
