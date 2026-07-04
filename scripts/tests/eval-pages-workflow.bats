#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  WORKFLOW="$ROOT/.github/workflows/eval-pages.yml"
  README="$ROOT/README.md"
}

@test "eval pages workflow configures GitHub Actions Pages deployments" {
  grep -q "pages: write" "$WORKFLOW"
  grep -q "id-token: write" "$WORKFLOW"
  grep -q "actions/configure-pages@v5" "$WORKFLOW"
  grep -q "actions/upload-pages-artifact@v4" "$WORKFLOW"
  grep -q "actions/deploy-pages@v4" "$WORKFLOW"
}

@test "trusted eval workflows upload diagnostics even after provider failures" {
  grep -q "if: always()" "$WORKFLOW"
  grep -q "eval-diagnostics" "$WORKFLOW"
  grep -q "evals/out/status.json" "$WORKFLOW"
  grep -q "if: always() && steps.secrets.outputs.available == 'true'" "$ROOT/.github/workflows/live-evals.yml"
}

@test "eval pages publishes an explicit skipped dashboard when provider secrets are absent" {
  grep -q "id: secrets" "$WORKFLOW"
  grep -q "available=false" "$WORKFLOW"
  grep -q "Record missing provider credentials" "$WORKFLOW"
  grep -q "steps.secrets.outputs.available != 'true'" "$WORKFLOW"
  grep -q "steps.secrets.outputs.available == 'true'" "$WORKFLOW"
  grep -q "scripts/evals/write-status.mjs" "$WORKFLOW"
}

@test "readme gives the required GitHub Pages source setting" {
  grep -q "repository Pages settings" "$README"
  grep -q "Build and deployment / Source" "$README"
  grep -q "GitHub Actions" "$README"
}
