#!/usr/bin/env bash
set -euo pipefail

plugin_root="${1:-"$(cd "$(dirname "${BASH_SOURCE[0]}")/../plugins/advisor" && pwd)"}"
agent="$plugin_root/agents/advisor.toml"
skill="$plugin_root/skills/advisor/SKILL.md"

fail() {
  echo "advisor-agent-config: $*" >&2
  exit 1
}

[ -f "$agent" ] || fail "missing-agent: $agent"
[ -f "$skill" ] || fail "missing-skill: $skill"

[ "$(grep -Fxc 'model = "gpt-5.6-sol"' "$agent")" -eq 1 ] ||
  fail "model-must-be-gpt-5.6-sol"
[ "$(grep -Fxc 'model_reasoning_effort = "high"' "$agent")" -eq 1 ] ||
  fail "reasoning-effort-must-be-high"
[ "$(grep -Fxc 'sandbox_mode = "read-only"' "$agent")" -eq 1 ] ||
  fail "sandbox-must-be-read-only"

if grep -Eqi '(^|_)(fallback|override)(_|[[:space:]]*=)' "$agent"; then
  fail "fallback-or-override-configured"
fi

if grep -Fq 'agent_type: default' "$skill"; then
  fail "skill-configures-default-agent-fallback"
fi

expected_unavailable_lines='   - If the custom agent is unavailable, stop and report the unavailable advisor agent. Do not silently substitute a different agent or model.'
actual_unavailable_lines="$(grep -Ei 'unavailable|substitut' "$skill")"

if [ "$actual_unavailable_lines" != "$expected_unavailable_lines" ]; then
  fail "skill-configures-agent-fallback"
fi

grep -Fq '`gpt-5.6-sol` with `model_reasoning_effort: high`' "$skill" ||
  fail "skill-must-document-pinned-model-and-effort"

expected_effort_lines='   - Use the custom `advisor` agent when available. Its agent file is the single routing source and pins read-only `gpt-5.6-sol` with `model_reasoning_effort: high`.
6. footer: `effort=high; playbook=<yes|no>; context=<repo/docs/web/none checked>`. `high` is the effort pinned by the custom agent file. For context, report only sources actually inspected; if you did not inspect repo files, docs, or web sources, use `none checked`.'
actual_effort_lines="$(grep -Ei 'effort|reasoning' "$skill")"

if [ "$actual_effort_lines" != "$expected_effort_lines" ]; then
  fail "skill-reports-unpinned-reasoning-effort"
fi

grep -Fq 'footer: `effort=high;' "$skill" ||
  fail "skill-must-report-pinned-reasoning-effort"

grep -Fq 'stop and report the unavailable advisor agent' "$skill" ||
  fail "skill-must-require-visible-unavailable-agent-failure"

expected_skill_sha256="3e5f67c0732dfabcda77da821258169ffe0acc56fa3c6c40d8425e12343a5c93"
actual_skill_sha256="$(sha256sum "$skill" | awk '{print $1}')"
[ "$actual_skill_sha256" = "$expected_skill_sha256" ] ||
  fail "skill-contract-drift"

echo "advisor-agent-config: ok"
