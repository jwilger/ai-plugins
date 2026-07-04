const fs = require("fs");
const path = require("path");

const DEFAULT_BEHAVIOR_DIR = path.resolve(
  process.cwd(),
  "evals/fixtures/behavior",
);

function walkJsonFiles(directory) {
  if (!fs.existsSync(directory)) {
    return [];
  }

  const entries = fs.readdirSync(directory, { withFileTypes: true });
  return entries
    .flatMap((entry) => {
      const entryPath = path.join(directory, entry.name);
      if (entry.isDirectory()) {
        return walkJsonFiles(entryPath);
      }
      return entry.isFile() && entry.name.endsWith(".json") ? [entryPath] : [];
    })
    .sort();
}

function readCasesFromFile(file, root) {
  const parsed = JSON.parse(fs.readFileSync(file, "utf8"));
  const cases = Array.isArray(parsed) ? parsed : parsed.cases;

  if (!Array.isArray(cases)) {
    throw new Error(`${file}: expected an array or object with cases array`);
  }

  return cases.map((testCase) => ({
    ...testCase,
    fixture_file: path.relative(root, file),
  }));
}

function loadBehaviorCases(options = {}) {
  const root = options.root || process.cwd();
  const behaviorDir =
    options.behaviorDir || path.join(root, "evals/fixtures/behavior");
  return walkJsonFiles(behaviorDir).flatMap((file) =>
    readCasesFromFile(file, root),
  );
}

function caseById(caseId, options = {}) {
  return loadBehaviorCases(options).find((testCase) => testCase.case_id === caseId);
}

function coverageKinds(testCase) {
  return [
    ...new Set([
      ...((testCase.coverage && testCase.coverage.kinds) || []),
      ...(testCase.coverageKinds || []),
    ]),
  ];
}

function valueGateMode(testCase) {
  return testCase.valueGate?.mode || testCase.valueGateMode || "standard";
}

function baselineLiftThreshold(testCase, matrix) {
  return Number(
    testCase.valueGate?.baselineLiftThreshold ??
      matrix?.valueGates?.defaultBaselineLiftThreshold ??
      0.1,
  );
}

module.exports = {
  DEFAULT_BEHAVIOR_DIR,
  baselineLiftThreshold,
  caseById,
  coverageKinds,
  loadBehaviorCases,
  valueGateMode,
  walkJsonFiles,
};
