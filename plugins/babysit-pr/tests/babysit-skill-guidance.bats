#!/usr/bin/env bats

setup() {
  SKILL="$BATS_TEST_DIRNAME/../skills/babysit-pr/SKILL.md"
}

@test "routes GitHub inline review comments through gh-address-comments" {
  grep -Fq 'github:gh-address-comments' "$SKILL"
  grep -Fq 'inline review thread' "$SKILL"
  grep -Fq 'Do not post a top-level PR comment' "$SKILL"
}

@test "defines babysitting as continuous monitoring until merge" {
  grep -Fq 'Continue polling until the PR/MR is merged' "$SKILL"
  grep -Fq 'Pending checks, bot reviews, auto-merge, and merge queue states are waiting states, not blockers' "$SKILL"
  grep -Fq 'Do not stop merely because there is nothing to do yet' "$SKILL"
  grep -Fq 'concrete unfixable human gate' "$SKILL"
}
