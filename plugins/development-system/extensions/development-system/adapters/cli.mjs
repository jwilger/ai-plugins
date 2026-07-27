#!/usr/bin/env node
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";
import { resolveStatus } from "./status-interpreter.ts";

const packageRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../../..",
);
let project = ".";
let mode = "print";
const args = process.argv.slice(2);
if (args.shift() !== "status") {
  console.error("development_system.usage command=status");
  process.exit(2);
}
while (args.length > 0) {
  const option = args.shift();
  if ((option === "--project" || option === "--mode") && args.length > 0) {
    if (option === "--project") project = args.shift();
    else mode = args.shift();
  } else {
    console.error(`development_system.unknown_option option=${option}`);
    process.exit(2);
  }
}
if (!["tui", "rpc", "json", "print"].includes(mode)) {
  console.error(`development_system.unsupported_mode mode=${mode}`);
  process.exit(2);
}
try {
  const status = await resolveStatus(project, packageRoot, mode);
  process.stdout.write(`${JSON.stringify({ ok: true, status })}\n`);
} catch (error) {
  const value = error instanceof Error ? error : new Error(String(error));
  process.stdout.write(
    `${JSON.stringify({
      ok: false,
      error: {
        code: value.code ?? "development_system.status_failed",
        message: value.message,
        nextAction:
          value.nextAction ??
          "Run development-system doctor and correct the reported error.",
      },
    })}\n`,
  );
  process.exitCode = 2;
}
