#!/usr/bin/env bats

setup() {
  REPO_ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
}

@test "the repository declares concurrent tickets as the worktree mode" {
  grep -Fxq 'mode = "concurrent-tickets"' "$REPO_ROOT/.development-system.toml"
  grep -Fq 'Ordinary questions and read-only investigation require neither a ticket nor a' \
    "$REPO_ROOT/plugins/development-system/skills/worktrees/SKILL.md"
  grep -Fq 'worktree. A single mutable ticket may use its current checkout.' \
    "$REPO_ROOT/plugins/development-system/skills/worktrees/SKILL.md"
  ! rg -F 'development_system_worktree_' \
    "$REPO_ROOT/plugins/development-system/skills/worktrees/SKILL.md"
  grep -Eq '^pre-commit:' "$REPO_ROOT/lefthook.yml"
  ! grep -Eq '^pre-push:' "$REPO_ROOT/lefthook.yml"
  [ ! -e "$REPO_ROOT/scripts/agent-checkout-guard.sh" ]
}

@test "active formula mirrors do not require Pi logical-worktree cleanup" {
  local formula_name

  for formula_name in development-change-direct development-change-pr development-change-local; do
    cmp \
      "$REPO_ROOT/plugins/development-system/formulas/$formula_name.formula.toml" \
      "$REPO_ROOT/.beads/formulas/$formula_name.formula.toml"
    ! rg -F 'development_system_worktree_finish' \
      "$REPO_ROOT/plugins/development-system/formulas/$formula_name.formula.toml"
    ! rg -F 'semantic worktree finish' \
      "$REPO_ROOT/plugins/development-system/formulas/$formula_name.formula.toml"
  done
}
