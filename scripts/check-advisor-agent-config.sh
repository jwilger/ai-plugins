#!/usr/bin/env bash
set -euo pipefail

plugin_root="${1:-"$(cd "$(dirname "${BASH_SOURCE[0]}")/../plugins/development-system" && pwd)"}"
agent="$plugin_root/agents/advisor.toml"
claude_agent="$plugin_root/agents/advisor.md"
skill="$plugin_root/skills/advisor/SKILL.md"

fail() {
  echo "advisor-agent-config: $*" >&2
  exit 1
}

[ -f "$agent" ] || fail "missing-agent: $agent"
[ -f "$claude_agent" ] || fail "missing-claude-agent: $claude_agent"
[ -f "$skill" ] || fail "missing-skill: $skill"
[ ! -e "$plugin_root/components/advisor" ] || fail "nested-advisor-component-must-be-removed"

python3 - "$agent" <<'PY' || fail "invalid-codex-advisor-route"
import pathlib
import sys
import tomllib

agent = tomllib.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
assert "model" not in agent
assert agent.get("model_reasoning_effort") == "xhigh"
assert agent.get("sandbox_mode") == "read-only"
PY

grep -Fq 'model: opus' "$claude_agent" || fail "claude-advisor-must-use-moving-opus-alias"
grep -Fq 'effort: max' "$claude_agent" || fail "claude-advisor-must-use-highest-supported-effort"
grep -Fq 'highest-capability eligible model' "$skill" || fail "skill-must-select-by-capability"
skill_flat="$(tr -s '[:space:]' ' ' <"$skill")"
[[ "$skill_flat" == *'the explicitly selected highest-capability model, and `xhigh` reasoning effort'* ]] || fail "skill-must-name-codex-xhigh-effort"
[[ "$skill_flat" == *'On Claude Code, launch the exact named public `advisor` Agent with the explicit model selection and its configured highest-supported effort'* ]] || fail "skill-must-name-claude-highest-effort"
grep -Fq 'two or more dependent implementation steps' "$skill" || fail "skill-must-trigger-for-multi-step-plans"
grep -Fq 'executable BDD-style scenario' "$skill" || fail "skill-must-define-bdd-exemption"
grep -Fq 'Do not invoke Advisor again' "$skill" || fail "skill-must-prevent-recursion"
grep -Fq 'Treat accepted explicit Codex spawn controls as authoritative' "$skill" || fail "skill-must-trust-accepted-controls"
grep -Fq 'failed or empty child result' "$skill" || fail "skill-must-reject-failed-or-empty-child"
if grep -Fq 'cannot verify required Codex spawn_agent evidence fields' "$skill"; then
  fail "skill-must-not-require-echoed-spawn-evidence"
fi

if grep -Eq 'gpt-[0-9]+\.[0-9]+-(sol|pro)|agent_type: default' "$skill" "$agent"; then
  fail "static-model-or-fallback-configured"
fi

echo "advisor-agent-config: ok"
