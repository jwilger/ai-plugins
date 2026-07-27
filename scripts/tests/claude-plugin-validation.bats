#!/usr/bin/env bats

setup() {
  REPO_ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
}

@test "development-system passes the installed Claude plugin validator" {
  command -v claude >/dev/null 2>&1 || skip "Claude Code is not installed"

  run claude plugin validate "$REPO_ROOT/plugins/development-system"
  [ "$status" -eq 0 ]
}
