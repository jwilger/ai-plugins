#!/usr/bin/env bats

setup() {
  SKILL="$BATS_TEST_DIRNAME/../skills/babysit-pr/SKILL.md"
}

@test "routes GitHub inline review comments through gh-address-comments" {
  grep -Fq 'github:gh-address-comments' "$SKILL"
  grep -Fq 'inline review thread' "$SKILL"
  grep -Fq 'Do not post a top-level PR comment' "$SKILL"
}
