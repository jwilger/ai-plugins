import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import {
  PI_REFERENCES,
  readPiReference,
} from "../../plugins/development-system/extensions/development-system/adapters/references.ts";

function piFixture() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "pi-reference-"));
  fs.mkdirSync(path.join(root, "bin"));
  fs.mkdirSync(path.join(root, "docs"));
  fs.writeFileSync(
    path.join(root, "package.json"),
    '{"name":"@earendil-works/pi-coding-agent"}\n',
  );
  const entrypoint = path.join(root, "bin", "pi.js");
  fs.writeFileSync(entrypoint, "entry\n");
  for (const relative of Object.values(PI_REFERENCES)) {
    fs.writeFileSync(path.join(root, relative), "one\ntwo\nthree\nfour\n");
  }
  return { root, entrypoint };
}

test("Pi reference reader exposes only named installed documents in bounded pages", async () => {
  const fixture = piFixture();
  const result = await readPiReference({
    document: "extensions",
    offset: 2,
    limit: 2,
    piEntrypoint: fixture.entrypoint,
  });
  assert.deepEqual(result.lines, ["two", "three"]);
  assert.equal(result.nextOffset, 4);
  assert.equal(result.path, path.join(fixture.root, "docs/extensions.md"));

  await assert.rejects(
    () =>
      readPiReference({
        document: "../../auth.json",
        piEntrypoint: fixture.entrypoint,
      }),
    /pi_reference_invalid/,
  );
  await assert.rejects(
    () =>
      readPiReference({
        document: "extensions",
        offset: 0,
        piEntrypoint: fixture.entrypoint,
      }),
    /pi_reference_range_invalid/,
  );
});
