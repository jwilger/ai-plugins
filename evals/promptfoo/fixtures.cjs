const fs = require("fs");
const path = require("path");

const DEFAULT_BEHAVIOR_DIR = path.resolve(
  process.cwd(),
  "evals/fixtures/behavior",
);
const DEFAULT_MATRIX_FILE = path.resolve(process.cwd(), "evals/matrix.json");
const caseCache = new Map();
const SUPPORTED_VALUE_GATE_MODES = Object.freeze([
  "measurement",
  "none",
  "safety-critical",
  "standard",
]);
const STABLE_CONTEXT_LEAK_MARKERS = Object.freeze([
  "AI_PLUGINS_PRIVATE_CONTEXT_CANARY",
  "PRIVATE_PLUGIN_GUIDANCE_CANARY",
]);
const HOST_CONTEXT_PATH_ENV = Object.freeze([
  "HOME",
  "CODEX_HOME",
  "CLAUDE_CONFIG_DIR",
  "CLAUDE_EVAL_AUTH_HOME",
  "CODEX_EVAL_HOME",
  "CODEX_EVAL_HOME_DEVELOPMENT_SYSTEM",
  "CODEX_EVAL_HOME_NO_PLUGINS",
  "CODEX_EVAL_SANITIZED_MARKETPLACE",
  "CLAUDE_EVAL_HOME_DEVELOPMENT_SYSTEM",
  "CLAUDE_EVAL_HOME_NO_PLUGINS",
  "CLAUDE_EVAL_CONFIG_DIR_DEVELOPMENT_SYSTEM",
  "CLAUDE_EVAL_CONFIG_DIR_NO_PLUGINS",
  "CLAUDE_EVAL_PLUGIN_CACHE_DIR_DEVELOPMENT_SYSTEM",
  "CLAUDE_EVAL_PLUGIN_CACHE_DIR_NO_PLUGINS",
  "CLAUDE_EVAL_RUNTIME_CONFIG_DIR_DEVELOPMENT_SYSTEM",
  "CLAUDE_EVAL_RUNTIME_CONFIG_DIR_NO_PLUGINS",
  "EVAL_PROVIDER_HOME",
  "EVAL_PROVIDER_WORKSPACE",
  "EVAL_PROVIDER_PLUGIN_SNAPSHOT",
  "EVAL_RUNTIME_ROOT",
]);

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

  return cases.map((testCase) => {
    validateProviderEvalPolicy(testCase, path.relative(root, file));
    return {
      ...testCase,
      fixture_file: path.relative(root, file),
    };
  });
}

function nonEmptyText(value) {
  return typeof value === "string" && value.trim().length > 0;
}

function normalizeHostPath(value) {
  if (!nonEmptyText(value) || !path.isAbsolute(value)) {
    return null;
  }

  const normalized = path.resolve(value);
  if (
    normalized === "/" ||
    normalized === "/runtime" ||
    normalized.startsWith("/runtime/") ||
    normalized === "/workspace" ||
    normalized.startsWith("/workspace/")
  ) {
    return null;
  }
  return normalized;
}

function contextLeakMarkers(options = {}) {
  return [
    ...new Set([
      ...stableContextLeakMarkers(),
      ...hostContextLeakMarkers(options),
    ]),
  ].sort(
    (left, right) => right.length - left.length || left.localeCompare(right),
  );
}

function stableContextLeakMarkers() {
  return [...STABLE_CONTEXT_LEAK_MARKERS];
}

function hostContextLeakMarkers(options = {}) {
  const env = options.env || process.env;
  const repositoryRoot = normalizeHostPath(
    options.root || path.resolve(__dirname, "../.."),
  );
  const hostPaths = [
    repositoryRoot,
    ...HOST_CONTEXT_PATH_ENV.map((name) => normalizeHostPath(env[name])),
  ].filter(Boolean);

  return [...new Set(hostPaths)].sort(
    (left, right) => right.length - left.length || left.localeCompare(right),
  );
}

function validateProviderEvalPolicy(testCase, fixtureFile) {
  const policy = testCase.providerEval;
  const label = `${fixtureFile}: ${testCase.case_id || "unknown-case"}`;
  if (!policy) {
    throw new Error(
      `${label} must declare providerEval with an unresolved stochastic question and deterministic-insufficiency rationale`,
    );
  }
  if (policy.deterministicVerificationFullyDecides === true) {
    throw new Error(
      `${label} is fully decided by deterministic verification and must not be a provider eval`,
    );
  }
  const mode = testCase.valueGate?.mode ?? testCase.valueGateMode ?? "standard";
  if (!SUPPORTED_VALUE_GATE_MODES.includes(mode)) {
    throw new Error(
      `${label} declares unsupported valueGate mode ${JSON.stringify(mode)}; expected one of ${SUPPORTED_VALUE_GATE_MODES.join(", ")}`,
    );
  }
  if (
    ["standard", "safety-critical"].includes(mode) &&
    !nonEmptyText(testCase.valueGate?.incrementalValueHypothesis)
  ) {
    throw new Error(
      `${label} baseline-lift gate requires an explicit incremental-value hypothesis`,
    );
  }

  if (mode === "standard") {
    const threshold = testCase.valueGate?.baselineLiftThreshold;
    if (
      threshold === undefined ||
      !Number.isFinite(Number(threshold)) ||
      Number(threshold) <= 0
    ) {
      throw new Error(
        `${label} standard baseline-lift threshold must be a positive number`,
      );
    }
  }

  if (testCase.baselineOutcome === "unexpected-pass" && mode === "none") {
    const audit = testCase.baselineAudit;
    if (
      !nonEmptyText(audit?.isolationAudit) ||
      !nonEmptyText(audit?.promptAndRubricAudit) ||
      !nonEmptyText(audit?.utilityDisposition)
    ) {
      throw new Error(
        `${label} cannot waive an unexpectedly successful baseline without isolation, prompt/rubric, and case-utility audit evidence`,
      );
    }
  }

  if (
    mode === "none" &&
    (policy.evidencePurpose !== "absolute-reliability" ||
      !nonEmptyText(policy.baselineAuditDisposition))
  ) {
    throw new Error(
      `${label} valueGate none requires an absolute-reliability evidence purpose and a defensible baseline-audit disposition`,
    );
  }

  if (
    policy.deterministicVerificationFullyDecides !== false ||
    !nonEmptyText(policy.unresolvedStochasticQuestion) ||
    !nonEmptyText(policy.deterministicInsufficiency)
  ) {
    throw new Error(
      `${label} must name an unresolved stochastic question and explain why deterministic verification is insufficient`,
    );
  }
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
  let caseFilterPattern;

  if (caseFilter) {
    if (typeof caseFilter !== "string") {
      throw new Error(
        `behavior case filter must be a string, received ${JSON.stringify(caseFilter)}`,
      );
    }

    try {
      caseFilterPattern = new RegExp(caseFilter);
    } catch (error) {
      const reason = error instanceof Error ? error.message : String(error);
      throw new Error(
        `invalid behavior case filter regex ${JSON.stringify(caseFilter)}: ${reason}`,
      );
    }
  }

  const selected = caseFilterPattern
    ? cases.filter((testCase) => caseFilterPattern.test(testCase.case_id))
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
  return testCase.valueGate?.mode ?? testCase.valueGateMode ?? "standard";
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
  contextLeakMarkers,
  hostContextLeakMarkers,
  fileUrl,
  loadBehaviorCases,
  loadMatrix,
  selectedBehaviorCases,
  selectedBehaviorPluginNames,
  stableContextLeakMarkers,
  valueGateMode,
  walkJsonFiles,
};
