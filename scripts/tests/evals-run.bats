#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  RUNNER="$ROOT/scripts/evals/run.sh"
}

@test "eval runner prints help" {
  run "$RUNNER" --help

  [ "$status" -eq 0 ]
  [[ "$output" == *"Usage: scripts/evals/run.sh"* ]]
  [[ "$output" == *"Claude Code: provider=anthropic:claude-agent-sdk, model=sonnet, skills=all"* ]]
  [[ "$output" == *"Codex:       provider=openai:codex-sdk, model=gpt-5.5, model_reasoning_effort=medium"* ]]
  [[ "$output" == *"Full repository plugin marketplace is loaded for every scenario"* ]]
  [[ "$output" == *"@openai/codex-sdk@0.142.5"* ]]
  [[ "$output" == *"@anthropic-ai/claude-agent-sdk@0.3.201"* ]]
  [[ "$output" == *"Requires working Claude Code and Codex model authentication"* ]]
  [[ "$output" == *"PROMPTFOO_VERSION            (default: 0.121.17)"* ]]
  [[ "$output" == *"Prompt response caching and hosted sharing are disabled"* ]]
  [[ "$output" == *"results.junit.xml"* ]]
}

@test "eval runner dry-run uses provider-backed harness config and repo-owned artifacts" {
  run "$RUNNER" --dry-run

  [ "$status" -eq 0 ]
  [[ "$output" == *"promptfoo@0.121.17"* ]]
  [[ "$output" == *"--max-concurrency 2"* ]]
  [[ "$output" == *"--no-cache"* ]]
  [[ "$output" == *"--no-share"* ]]
  [[ "$output" == *"evals/out/generated/agentic-systems-engineering.behavior.yaml"* ]]
  [[ "$output" == *"evals/out/results.json"* ]]
  [[ "$output" == *"evals/out/report.html"* ]]
  [[ "$output" == *"evals/out/results.junit.xml"* ]]
}
