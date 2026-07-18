const path = require("node:path");
const { loadWorkspaceManifest, providerLabelFor } = require("./manifest.cjs");
const { loadRuntimeManifest } = require("./runtime-manifest.cjs");
const { promptFor } = require("./benchmark-inputs.cjs");

function assertionFor(testCase) {
  if (testCase.caseId === "rust-cli-feature") {
    return `file://${path.join(__dirname, "assertions/expense-report.cjs")}`;
  }
  throw new Error(`benchmark assertion is not implemented: ${testCase.caseId}`);
}

module.exports = function loadCodeQualityCases() {
  const workspaceState = loadWorkspaceManifest({
    requireBaselineHead: true,
    requireClean: true,
  });
  const runtimeState = loadRuntimeManifest({
    phase: "pre-turn",
    workspaceState,
  });
  const { manifest } = workspaceState;
  const rows = runtimeState.rows;

  return rows.map((row) => {
    const expectedProviderLabel = providerLabelFor(row.mode);
    return {
      description: `${row.caseId} sample ${row.sample} ${row.mode}`,
      options: { disableVarExpansion: true },
      providers: [expectedProviderLabel],
      vars: {
        baseline_oid: row.baselineOid,
        benchmark_expected_samples: manifest.sampleCount,
        case_id: row.caseId,
        condition_id: row.mode,
        expected_provider_label: expectedProviderLabel,
        fixture_digest: row.fixtureDigest,
        min_pass_rate: 0,
        sample_index: row.sample,
        scenario_prompt: promptFor(row),
        task_type: row.taskType,
        value_gate_mode: "measurement",
        workspace: row.workspace,
        available_skills: row.availableSkills,
        codex_home: row.codexHome,
        codex_tmp: row.codexTmp,
        composition_hash: row.compositionHash,
        contract_sha256: row.contractSha256,
        input_hash: row.inputHash,
        matrix_hash: row.matrixHash,
        run_id: row.runId,
        runtime_manifest_sha256: runtimeState.runtimeManifestSha256,
        workspace_manifest_sha256: row.workspaceManifestSha256,
      },
      assert: [
        {
          type: "javascript",
          value: assertionFor(row),
        },
      ],
    };
  });
};
