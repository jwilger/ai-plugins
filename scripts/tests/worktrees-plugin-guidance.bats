#!/usr/bin/env bats

setup() {
  SKILL="$BATS_TEST_DIRNAME/../../plugins/worktrees/skills/setup/SKILL.md"
  README="$BATS_TEST_DIRNAME/../../plugins/worktrees/README.md"
}

@test "setup skill defaults new worktrees to the ignored repo-local directory" {
  grep -Fq '`./.worktrees/`' "$SKILL"
  grep -Fq 'default checkout root' "$SKILL"
}

@test "setup skill requires adapting command wrappers to the project" {
  grep -Fq 'Detect the project command surface' "$SKILL"
  grep -Fq 'justfile' "$SKILL"
  grep -Fq 'Makefile' "$SKILL"
  grep -Fq 'package.json' "$SKILL"
  grep -Fq 'confirm the selected wrapper' "$SKILL"
}

@test "plugin readme documents shell-first behavior instead of assuming just" {
  grep -Fq './.worktrees/' "$README"
  grep -Fq 'do not require `just`' "$README"
}
