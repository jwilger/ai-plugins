const fs = require("fs");
const path = require("path");

const DEFAULT_BEHAVIOR_DIR = path.resolve(
  process.cwd(),
  "evals/fixtures/behavior",
);
const DEFAULT_MATRIX_FILE = path.resolve(process.cwd(), "evals/matrix.json");
const caseCache = new Map();

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
  const cacheKey = `${path.resolve(root)}::${path.resolve(behaviorDir)}`;
  if (caseCache.has(cacheKey)) {
    return caseCache.get(cacheKey);
  }

  const cases = walkJsonFiles(behaviorDir).flatMap((file) =>
    readCasesFromFile(file, root),
  );
  const seen = new Map();
  for (const testCase of cases) {
    const prior = seen.get(testCase.case_id);
    if (prior) {
      throw new Error(
        `Duplicate case_id "${testCase.case_id}" in ${testCase.fixture_file} and ${prior}`,
      );
    }
    seen.set(testCase.case_id, testCase.fixture_file);
  }

  caseCache.set(cacheKey, cases);
  return cases;
}

function selectedBehaviorCases(options = {}) {
  const cases = loadBehaviorCases(options);
  const caseFilter = options.caseFilter;
  const selected = caseFilter
    ? cases.filter((testCase) => testCase.case_id.includes(caseFilter))
    : cases;

  if (selected.length === 0) {
    throw new Error(
      `no behavior cases match case filter ${JSON.stringify(caseFilter)}`,
    );
  }

  return selected;
}

function selectedBehaviorPluginNames(options = {}) {
  const pluginNames = new Set();

  for (const testCase of selectedBehaviorCases(options)) {
    if (!Array.isArray(testCase.plugins) || testCase.plugins.length === 0) {
      throw new Error(
        `${testCase.fixture_file}: ${testCase.case_id} must declare a non-empty plugins array`,
      );
    }

    for (const pluginName of testCase.plugins) {
      if (
        typeof pluginName !== "string" ||
        pluginName.length === 0 ||
        pluginName.trim() !== pluginName
      ) {
        throw new Error(
          `${testCase.fixture_file}: ${testCase.case_id} declares an invalid plugin name`,
        );
      }
      pluginNames.add(pluginName);
    }
  }

  return [...pluginNames].sort();
}

function caseById(caseId, options = {}) {
  return loadBehaviorCases(options).find(
    (testCase) => testCase.case_id === caseId,
  );
}

function fileUrl(file, base = process.cwd()) {
  return `file://${path.resolve(base, file)}`;
}

function loadMatrix(options = {}) {
  const matrixFile =
    options.matrixFile ||
    path.join(options.root || process.cwd(), "evals/matrix.json");
  return JSON.parse(fs.readFileSync(matrixFile, "utf8"));
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
  DEFAULT_MATRIX_FILE,
  baselineLiftThreshold,
  caseById,
  coverageKinds,
  fileUrl,
  loadBehaviorCases,
  loadMatrix,
  selectedBehaviorCases,
  selectedBehaviorPluginNames,
  valueGateMode,
  walkJsonFiles,
};
