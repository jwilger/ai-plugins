import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

const root = path.resolve(import.meta.dirname, "../..");

function fixture() {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), "pi-eval-"));
  const source = path.join(directory, "source");
  fs.mkdirSync(source, { mode: 0o700 });
  fs.writeFileSync(
    path.join(source, "auth.json"),
    JSON.stringify({
      "openai-codex": {
        type: "oauth",
        access: "secret-access",
        refresh: "secret-refresh",
        expires: 9999999999999,
      },
      "other-provider": { type: "api_key", key: "must-not-copy" },
    }),
    { mode: 0o600 },
  );
  return { directory, source };
}

test("Pi eval homes copy only OpenAI auth and derive packages from support inventory", () => {
  const { directory, source } = fixture();
  const sourceBefore = fs.readFileSync(path.join(source, "auth.json"), "utf8");
  for (const mode of ["no-plugins", "development-system", "full-marketplace"]) {
    const destination = path.join(directory, mode);
    execFileSync(
      process.execPath,
      [path.join(root, "scripts/evals/prepare-pi-home.mjs"), destination, mode],
      {
        env: { ...process.env, PI_EVAL_SOURCE_HOME: source },
      },
    );
    const auth = JSON.parse(
      fs.readFileSync(path.join(destination, "auth.json"), "utf8"),
    );
    assert.deepEqual(Object.keys(auth), ["openai-codex"]);
    assert.equal(
      fs.statSync(path.join(destination, "auth.json")).mode & 0o777,
      0o600,
    );
    const composition = JSON.parse(
      fs.readFileSync(path.join(destination, "composition.json"), "utf8"),
    );
    assert.deepEqual(
      composition.packages,
      mode === "no-plugins" ? [] : ["development-system"],
    );
  }
  assert.equal(
    fs.readFileSync(path.join(source, "auth.json"), "utf8"),
    sourceBefore,
  );
});

test("Pi Promptfoo provider captures output, trajectory, composition, and extension provenance", async () => {
  const { directory } = fixture();
  const fakePi = path.join(directory, "pi");
  fs.writeFileSync(
    fakePi,
    `#!/usr/bin/env node
const fs = require("fs");
if (process.env.DEVELOPMENT_SYSTEM_PI_EVAL_MARKER) fs.writeFileSync(process.env.DEVELOPMENT_SYSTEM_PI_EVAL_MARKER, JSON.stringify({ package: "development-system", extension: "/fixture/index.ts", version: "1.2.0" }));
console.log(JSON.stringify({ type: "tool_execution_end", toolName: "development_system_status", isError: false }));
console.log(JSON.stringify({ type: "message_end", message: { role: "assistant", content: [{ type: "text", text: "answer" }], usage: { input: 3, output: 2, totalTokens: 5, cost: { total: 0 } } } }));
`,
  );
  fs.chmodSync(fakePi, 0o755);
  process.env.PI_EVAL_BIN = fakePi;
  const { default: Provider } = await import(
    `../../scripts/evals/pi-provider.mjs?test=${Date.now()}`
  );
  const home = path.join(directory, "home");
  fs.mkdirSync(home);
  fs.writeFileSync(
    path.join(home, "composition.json"),
    JSON.stringify({
      mode: "development-system",
      packages: ["development-system"],
    }),
  );
  const provider = new Provider({
    id: "fixture",
    config: {
      agent_dir: home,
      working_dir: directory,
      package_mode: "development-system",
    },
  });
  const response = await provider.callApi("scenario");
  assert.equal(response.output, "answer");
  assert.equal(response.tokenUsage.total, 5);
  assert.deepEqual(response.metadata.packages, ["development-system"]);
  assert.equal(response.metadata.extensionProvenance.version, "1.2.0");
  assert.deepEqual(response.metadata.toolTrajectory, [
    { name: "development_system_status", isError: false },
  ]);
});
