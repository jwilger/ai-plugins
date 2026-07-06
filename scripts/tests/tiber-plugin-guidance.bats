#!/usr/bin/env bats

setup() {
  SKILL="$BATS_TEST_DIRNAME/../../plugins/tiber/skills/tiber/SKILL.md"
}

@test "tiber skill initializes MCP use before falling back to CLI" {
  grep -Fq 'Check for an installed `tiber` MCP server before using CLI commands.' "$SKILL"
  grep -Fq 'If the MCP tools are available, initialize the server with `tiber.init` only when setup is required, then use MCP tools for task reads and writes.' "$SKILL"
  grep -Fq 'If MCP tools are unavailable or fail to expose the needed operation, fall back to the bundled `tiber` CLI.' "$SKILL"
}
