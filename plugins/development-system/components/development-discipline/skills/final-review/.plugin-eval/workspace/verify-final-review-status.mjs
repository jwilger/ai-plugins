import { readFileSync } from "node:fs";
import { basename, dirname } from "node:path";
import { validateFinalReviewStatus } from "./final-review-status-contract.mjs";

const tempRoot = basename(dirname(process.cwd()));
const match = /^plugin-eval-(.+)-[^-]+$/.exec(tempRoot);
if (!match) {
  console.error(
    `cannot derive benchmark scenario from workspace path: ${tempRoot}`,
  );
  process.exit(1);
}
const expectedScenario = match[1];

let status;
try {
  status = JSON.parse(readFileSync("final-review-status.json", "utf8"));
} catch (error) {
  console.error(
    `final-review-status.json must be valid JSON: ${error.message}`,
  );
  process.exit(1);
}

const errors = validateFinalReviewStatus(status, expectedScenario);
if (errors.length > 0) {
  for (const error of errors) console.error(error);
  process.exitCode = 1;
}
