const fs = require("node:fs");
const path = require("node:path");

const root = path.resolve(__dirname, "../../..");
const models = ["sol", "terra", "luna"];
const standardProviderLabels = models.map(
  (model) => `codex-gpt-5.6-${model}-standard`,
);
const advisorProviderLabels = models.map(
  (model) => `codex-gpt-5.6-${model}-advisor-like`,
);
const standardCaseIds = new Set([
  "agentic-tool-contracts-and-loops",
  "development-discipline-review-feedback-skepticism",
]);

function readJson(file) {
  return JSON.parse(fs.readFileSync(path.join(root, file), "utf8"));
}

function selectedStandardFixtures() {
  return readJson("evals/fixtures/behavior/full-marketplace/cases.json").filter(
    (testCase) => standardCaseIds.has(testCase.case_id),
  );
}

function standardPluginNames() {
  const plugins = [
    ...new Set(
      selectedStandardFixtures().flatMap((testCase) => testCase.plugins || []),
    ),
  ].sort();

  if (plugins.length === 0) {
    throw new Error("standard benchmark cases select no marketplace plugins");
  }
  if (plugins.includes("advisor")) {
    throw new Error(
      "standard benchmark cases cannot load delegation-only Advisor guidance",
    );
  }

  return plugins;
}

function standardCases() {
  return selectedStandardFixtures().map((testCase) => ({
    id: testCase.case_id,
    behavior: testCase.behavior,
    prompt: testCase.prompt,
    rubric: testCase.semanticRubric,
    category: "standard",
    providers: standardProviderLabels,
  }));
}

function advisorLikeCases() {
  const selected = new Set(["tradeoff-recommendation", "ticket-plan-outline"]);
  const benchmark = readJson(
    "plugins/advisor/skills/advisor/.plugin-eval/benchmark.json",
  );

  return benchmark.scenarios
    .filter((scenario) => selected.has(scenario.id))
    .map((scenario) => ({
      id: `advisor-like-${scenario.id}`,
      behavior: scenario.purpose,
      prompt: `Answer directly without delegating to another agent. ${scenario.userInput.replace(
        /^Use the local Codex skill "advisor" if it helps\.\s*/,
        "",
      )}`,
      rubric: `Pass only if the response satisfies every requirement: ${scenario.successChecklist.join(
        " ",
      )}`,
      category: "advisor-like",
      providers: advisorProviderLabels,
    }));
}

function loadBenchmarkCases() {
  const rawSamples = process.env.GPT56_BENCHMARK_SAMPLES ?? "1";
  if (!/^(?:[1-9]|10)$/.test(rawSamples)) {
    throw new RangeError(
      `GPT56_BENCHMARK_SAMPLES must be a canonical integer from 1 through 10; got ${JSON.stringify(rawSamples)}`,
    );
  }
  const samples = Number(rawSamples);
  const cases = [...standardCases(), ...advisorLikeCases()];

  return cases.flatMap((testCase) =>
    Array.from({ length: samples }, (_, index) => ({
      description:
        samples > 1 ? `${testCase.id} sample ${index + 1}` : testCase.id,
      providers: testCase.providers,
      vars: {
        case_id: testCase.id,
        behavior: testCase.behavior,
        benchmark_category: testCase.category,
        benchmark_expected_provider_labels: testCase.providers,
        benchmark_expected_samples: samples,
        min_pass_rate: 0,
        scenario_prompt: testCase.prompt,
        sample_index: index + 1,
        value_gate_mode: "measurement",
      },
      assert: [
        {
          type: "llm-rubric",
          value: testCase.rubric,
        },
      ],
    })),
  );
}

loadBenchmarkCases.standardPluginNames = standardPluginNames;

module.exports = loadBenchmarkCases;
