const {
  baselineLiftThreshold,
  coverageKinds,
  fileUrl,
  loadMatrix,
  resolveSkillReferences,
  selectedBehaviorCases,
  valueGateMode,
} = require("./fixtures.cjs");
const fs = require("fs");
const path = require("path");

function matrix() {
  try {
    return loadMatrix();
  } catch {
    return {};
  }
}

function runtimeOptions() {
  const optionsFile =
    process.env.EVAL_RUNTIME_OPTIONS_FILE ||
    path.join(process.cwd(), "evals/out/generated/runtime-options.json");
  if (!fs.existsSync(optionsFile)) {
    return {};
  }
  return JSON.parse(fs.readFileSync(optionsFile, "utf8"));
}

function skillInvocationMode(runtime) {
  const mode =
    process.env.EVAL_SKILL_INVOCATION_MODE ||
    runtime.skillInvocationMode ||
    "natural";
  if (!["natural", "forced"].includes(mode)) {
    throw new Error(
      `EVAL_SKILL_INVOCATION_MODE must be natural or forced; got ${JSON.stringify(mode)}`,
    );
  }
  return mode;
}

function scenarioPrompt(testCase, skillReferences, invocationMode) {
  if (invocationMode === "natural") {
    return testCase.prompt;
  }

  return `Apply ${skillReferences.join(", ")} and read its instructions before answering.\n\n${testCase.prompt}`;
}

module.exports = function generateTests() {
  const runtime = runtimeOptions();
  const samples = Number.parseInt(
    process.env.EVAL_SAMPLES || runtime.samples || "1",
    10,
  );
  const filter = process.env.EVAL_CASE_FILTER || runtime.caseFilter;
  const invocationMode = skillInvocationMode(runtime);
  const evalMatrix = matrix();
  const cases = selectedBehaviorCases({ caseFilter: filter });

  return cases.flatMap((testCase) =>
    Array.from({ length: samples }, (_, index) => {
      const skillReferences =
        invocationMode === "forced" ? resolveSkillReferences(testCase) : [];
      return {
        description:
          samples > 1
            ? `${testCase.case_id} sample ${index + 1}`
            : testCase.case_id,
        options: { disableVarExpansion: true },
        vars: {
          case_id: testCase.case_id,
          behavior: testCase.behavior,
          scenario_prompt: scenarioPrompt(
            testCase,
            skillReferences,
            invocationMode,
          ),
          sample_index: index + 1,
          sample_count: samples,
          configured_min_pass_rate: testCase.minPassRate,
          min_pass_rate: samples === 1 ? 1 : testCase.minPassRate,
          fixture_file: testCase.fixture_file,
          plugins: testCase.plugins || [],
          skills: testCase.skills || [],
          skill_invocation_mode: invocationMode,
          skill_references: skillReferences,
          coverage_kinds: coverageKinds(testCase),
          value_gate_mode: valueGateMode(testCase),
          baseline_lift_threshold: baselineLiftThreshold(testCase, evalMatrix),
          hard_guard_status:
            (testCase.hardAssertions || []).length > 0 ? "configured" : "none",
          tags: (testCase.tags || []).join(","),
        },
        assert: [
          {
            type: "javascript",
            value: fileUrl("assert-hard-guards.cjs", __dirname),
          },
          {
            type: "llm-rubric",
            value: testCase.semanticRubric,
          },
        ],
      };
    }),
  );
};
