#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";

function fail(message) {
  throw new Error(`model-routing fixture materializer: ${message}`);
}

function parseArguments(argv) {
  const options = {};
  for (let index = 0; index < argv.length; index += 2) {
    const key = argv[index];
    const value = argv[index + 1];
    if (!value || !["--specs", "--output"].includes(key))
      fail("invalid arguments");
    options[key.slice(2)] = path.resolve(value);
  }
  if (!options.specs || !options.output)
    fail("--specs and --output are required");
  return options;
}

function readSpecs(filePath) {
  let document;
  try {
    document = JSON.parse(fs.readFileSync(filePath, "utf8"));
  } catch {
    fail("specs could not be read as JSON");
  }
  if (document?.schema_version !== 1 || !Array.isArray(document.fixtures)) {
    fail("unsupported fixture schema");
  }
  return document.fixtures;
}

function validateRelativeFile(file) {
  if (
    typeof file !== "string" ||
    file.length === 0 ||
    path.isAbsolute(file) ||
    file.split(/[\\/]/).some((part) => !part || part === "." || part === "..")
  ) {
    fail("fixture contains an unsafe file path");
  }
}

function validateFixture(fixture, seen) {
  if (
    !/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(fixture?.fixture_id ?? "") ||
    seen.has(fixture.fixture_id) ||
    !fixture.files ||
    Array.isArray(fixture.files) ||
    typeof fixture.files !== "object" ||
    Object.keys(fixture.files).length === 0
  ) {
    fail("fixture identity or files are invalid");
  }
  seen.add(fixture.fixture_id);
  for (const [file, contents] of Object.entries(fixture.files)) {
    validateRelativeFile(file);
    if (typeof contents !== "string")
      fail("fixture file contents must be strings");
  }
}

function materialize(fixtures, output) {
  if (fs.existsSync(output)) fail("output already exists");
  fs.mkdirSync(output, { recursive: true, mode: 0o700 });
  const seen = new Set();
  for (const fixture of fixtures) {
    validateFixture(fixture, seen);
    const fixtureRoot = path.join(output, fixture.fixture_id);
    fs.mkdirSync(fixtureRoot, { mode: 0o700 });
    for (const [file, contents] of Object.entries(fixture.files).sort(
      ([a], [b]) => a.localeCompare(b),
    )) {
      const destination = path.join(fixtureRoot, file);
      fs.mkdirSync(path.dirname(destination), { recursive: true, mode: 0o700 });
      fs.writeFileSync(destination, contents, { flag: "wx", mode: 0o600 });
    }
  }
}

try {
  const options = parseArguments(process.argv.slice(2));
  materialize(readSpecs(options.specs), options.output);
  console.log(`model-routing fixtures materialized: ${options.output}`);
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 2;
}
