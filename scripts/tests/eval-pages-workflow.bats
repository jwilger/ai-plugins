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
  grep -q "if: always() && steps.secrets.outputs.available == 'true'" "$ROOT/.github/workflows/live-evals.yml"
}

@test "readme gives the required GitHub Pages source setting" {
  grep -q "Settings > Pages" "$README"
  grep -q "Source" "$README"
  grep -q "GitHub Actions" "$README"
}
