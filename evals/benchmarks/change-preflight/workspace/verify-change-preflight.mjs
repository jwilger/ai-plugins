import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";

const recordPath = process.argv[2] ?? "change-preflight.json";
const workspaceRoot = path.resolve(process.argv[3] ?? ".");
const trustedScenario = process.argv[4] ?? inferScenario();
const expectedWorkspaceHash =
  "5cc527341af77e639fcd2e71081e11fb6b7dc98a18e1e95e742137c2d10ad71b";
const surfaces = [
  "behavior",
  "tests",
  "documentation",
  "configuration",
  "packaging",
  "releaseArtifacts",
  "migrations",
  "operationalStartup",
  "evaluations",
  "userWorkflows",
];
const effectTerms = {
  behavior: ["command", "default", "schema", "record", "behavior"],
  tests: ["test", "coverage", "regression", "migration"],
  documentation: ["document", "guidance", "setup", "recovery"],
  configuration: ["config", "default", "schema", "version"],
  packaging: ["completion", "binary", "package", "output"],
  releaseArtifacts: ["release", "changelog", "announce", "note"],
  migrations: ["upgrade", "migration", "rollback", "recovery", "schema"],
  operationalStartup: ["startup", "start", "deploy", "schema", "config"],
  evaluations: ["evaluation", "eval", "case", "behavior"],
  userWorkflows: ["user", "workflow", "setup", "export", "upgrade", "recovery"],
};

function fail(message) {
  console.error(`change-preflight invalid: ${message}`);
  process.exit(1);
}

function inferScenario() {
  const parent = path.basename(path.dirname(process.cwd()));
  return ["feature", "docs-config", "migration"].find((name) =>
    parent.startsWith(`plugin-eval-${name}-`),
  );
}

function workspaceHash(root) {
  const files = [];
  function visit(directory) {
    for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
      const target = path.join(directory, entry.name);
      const relative = path.relative(root, target);
      if (
        ["change-preflight.json", "verify-change-preflight.mjs"].includes(
          relative,
        )
      )
        continue;
      entry.isDirectory() ? visit(target) : files.push(relative);
    }
  }
  visit(root);
  const hash = crypto.createHash("sha256");
  for (const relative of files.sort()) {
    hash.update(relative).update("\0");
    hash.update(fs.readFileSync(path.join(root, relative))).update("\0");
  }
  return hash.digest("hex");
}

function hasMeaningfulText(value) {
  return typeof value === "string" && value.trim().length >= 12;
}

function containsAny(value, words) {
  const normalized = value.toLowerCase();
  return words.some((word) => normalized.includes(word));
}

function contradictsEffect(value) {
  return (
    /\b(no|not|without)\b.*\b(change|changes|effect|needed|required)\b/i.test(
      value,
    ) || /\b(remains?|stays?)\s+(the\s+same|unchanged)\b/i.test(value)
  );
}

function expressesAbsence(value) {
  return /\b(no|none|not|without|unchanged|doesn't|does not|don't|do not)\b/i.test(
    value,
  );
}

const scenarios = {
  feature: {
    classification: "feature",
    applicable: [
      "behavior",
      "tests",
      "documentation",
      "packaging",
      "releaseArtifacts",
      "evaluations",
      "userWorkflows",
    ],
    evidence: {
      behavior: ["feature/src/commands.md"],
      tests: ["feature/tests/cli.bats"],
      documentation: ["feature/docs/cli.md"],
      configuration: ["feature/request.md"],
      packaging: [
        "feature/completions/export.txt",
        "feature/release-binaries.json",
      ],
      releaseArtifacts: ["feature/CHANGELOG.md"],
      migrations: ["feature/request.md"],
      operationalStartup: ["feature/request.md"],
      evaluations: ["feature/evals/cases.json"],
      userWorkflows: ["feature/workflows/export-cli.md"],
    },
    reasonTerms: {
      configuration: ["configuration", "default"],
      migrations: ["schema", "data", "format"],
      operationalStartup: ["startup", "deployment"],
    },
  },
  "docs-config": {
    classification: "docs/config",
    applicable: [
      "documentation",
      "configuration",
      "operationalStartup",
      "userWorkflows",
    ],
    evidence: {
      behavior: ["docs-config/config/example.toml"],
      tests: ["docs-config/request.md"],
      documentation: ["docs-config/docs/setup.md"],
      configuration: ["docs-config/config/example.toml"],
      packaging: ["docs-config/request.md"],
      releaseArtifacts: ["docs-config/request.md"],
      migrations: ["docs-config/request.md"],
      operationalStartup: ["docs-config/scripts/start.sh"],
      evaluations: ["docs-config/request.md"],
      userWorkflows: ["docs-config/workflows/setup.md"],
    },
    reasonTerms: {
      behavior: ["application", "runtime", "default", "example"],
      tests: ["test"],
      packaging: ["package", "generated"],
      releaseArtifacts: ["release", "version", "catalog"],
      migrations: ["schema", "data"],
      evaluations: ["evaluation", "agent"],
    },
    allowedEvidence: {
      documentation: ["docs-config/docs/setup.md", "docs-config/request.md"],
      configuration: [
        "docs-config/config/example.toml",
        "docs-config/request.md",
      ],
      operationalStartup: [
        "docs-config/scripts/start.sh",
        "docs-config/config/example.toml",
      ],
      userWorkflows: [
        "docs-config/workflows/setup.md",
        "docs-config/docs/setup.md",
      ],
    },
  },
  migration: {
    classification: "migration",
    applicable: [
      "behavior",
      "tests",
      "documentation",
      "configuration",
      "releaseArtifacts",
      "migrations",
      "operationalStartup",
      "userWorkflows",
    ],
    evidence: {
      behavior: ["migration/src/task-schema.md"],
      tests: ["migration/tests/migration.bats"],
      documentation: ["migration/docs/recovery.md"],
      configuration: ["migration/config/schema-version.json"],
      packaging: ["migration/request.md"],
      releaseArtifacts: ["migration/CHANGELOG.md"],
      migrations: ["migration/workflows/migration-upgrade-rollback.md"],
      operationalStartup: ["migration/scripts/start.sh"],
      evaluations: ["migration/request.md"],
      userWorkflows: ["migration/workflows/upgrade-verify-recover.md"],
    },
    reasonTerms: {
      packaging: ["package", "generated"],
      evaluations: ["evaluation", "agent"],
    },
  },
};

let record;
try {
  record = JSON.parse(fs.readFileSync(recordPath, "utf8"));
} catch (error) {
  fail(error instanceof Error ? error.message : "unreadable JSON");
}

const target = path.join(workspaceRoot, "project/implementation-target.txt");
if (
  fs.readFileSync(target, "utf8") !==
  "Representative implementation target. Do not edit during preflight.\n"
)
  fail("implementation target changed before preflight completed");
if (workspaceHash(workspaceRoot) !== expectedWorkspaceHash)
  fail("representative repository changed before preflight completed");
const expected = scenarios[trustedScenario];
if (!expected) fail("trusted scenario is required");
if (record.scenario !== trustedScenario)
  fail("record does not match trusted scenario");
if (record.beforeEditing !== true)
  fail("record must be emitted before editing");
if (record.classification !== expected.classification)
  fail("classification does not match scenario evidence");
if (
  JSON.stringify(record.repositoryPolicyEvidence) !==
  JSON.stringify(["AGENTS.md"])
)
  fail("repositoryPolicyEvidence must cite repository facts");
const topFields = [
  "scenario",
  "beforeEditing",
  "classification",
  "repositoryPolicyEvidence",
  "surfaces",
];
const extras = Object.keys(record).filter((key) => !topFields.includes(key));
if (extras.length) fail(`unexpected top-level fields: ${extras.join(", ")}`);
if (
  !record.surfaces ||
  JSON.stringify(Object.keys(record.surfaces).sort()) !==
    JSON.stringify([...surfaces].sort())
)
  fail("all and only the ten required surfaces must be present");

for (const name of surfaces) {
  const decision = record.surfaces[name];
  const applicable = expected.applicable.includes(name);
  const wantedStatus = applicable ? "applicable" : "not-applicable";
  const actualStatus = decision?.status?.replaceAll(" ", "-");
  if (actualStatus !== wantedStatus)
    fail(`${name} must be ${wantedStatus} for ${record.scenario}`);
  const allowed = applicable
    ? ["status", "evidence", "effect"]
    : ["status", "evidence", "reason"];
  if (trustedScenario === "migration" && name === "migrations")
    allowed.push("decisions");
  const extraFields = Object.keys(decision).filter(
    (key) => !allowed.includes(key),
  );
  if (extraFields.length)
    fail(`${name} has unexpected fields: ${extraFields.join(", ")}`);
  if (
    !Array.isArray(decision.evidence) ||
    !expected.evidence[name].every((reference) =>
      decision.evidence.includes(reference),
    ) ||
    decision.evidence.some((reference) => {
      const allowed =
        expected.allowedEvidence?.[name] ?? expected.evidence[name];
      return typeof reference !== "string" || !allowed.includes(reference);
    })
  )
    fail(`${name} evidence is not grounded in the scenario repository facts`);
  for (const reference of decision.evidence)
    if (!fs.existsSync(path.join(workspaceRoot, reference)))
      fail(
        `${name} evidence path does not exist in the representative repository`,
      );
  if (applicable) {
    if (
      !hasMeaningfulText(decision.effect) ||
      !containsAny(decision.effect, effectTerms[name]) ||
      contradictsEffect(decision.effect) ||
      /^(affected|applicable|change required)\.?$/i.test(decision.effect.trim())
    )
      fail(`${name} applicable decision needs a concrete effect`);
  } else {
    if (
      !hasMeaningfulText(decision.reason) ||
      !expressesAbsence(decision.reason) ||
      !containsAny(decision.reason, expected.reasonTerms[name])
    )
      fail(`${name} not-applicable decision needs a scenario-specific reason`);
  }
}

if (trustedScenario === "migration") {
  const decisions = record.surfaces.migrations.decisions;
  const names = ["compatibility", "rollback", "recovery", "backfill"];
  const decisionTerms = {
    compatibility: ["compatible", "prior", "reader", "continue"],
    rollback: ["restore", "revert", "safe", "prior"],
    recovery: ["recover", "restore", "resume", "interrupt"],
    backfill: ["backfill", "existing", "on demand", "repository"],
  };
  const contradiction =
    /\b(impossible|unsupported|broken|incompatible|cannot|can't|never|no longer|stop working)\b/i;
  if (
    !decisions ||
    JSON.stringify(Object.keys(decisions).sort()) !==
      JSON.stringify([...names].sort()) ||
    names.some(
      (name) =>
        !hasMeaningfulText(decisions[name]) ||
        !containsAny(decisions[name], decisionTerms[name]) ||
        contradiction.test(decisions[name]),
    )
  )
    fail(
      "migrations must decide compatibility, rollback, recovery, and backfill",
    );
}
