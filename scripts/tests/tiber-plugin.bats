#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
}

@test "tiber ships invokable new-task skill for backlog capture" {
  run node - "$ROOT" <<'NODE'
const fs = require('fs');
const path = require('path');

const root = process.argv[2];
const skillPath = path.join(root, 'plugins/tiber/skills/tiber/SKILL.md');
const newTaskSkillPath = path.join(root, 'plugins/tiber/skills/new-task/SKILL.md');
const shadowedCommandPath = path.join(root, 'plugins/tiber/commands/new-task.md');
const readmePath = path.join(root, 'plugins/tiber/README.md');
const casesPath = path.join(root, 'evals/fixtures/behavior/tiber/cases.json');

const skill = fs.readFileSync(skillPath, 'utf8');
const normalizedSkill = skill.replace(/\s+/g, ' ');
const newTaskSkill = fs.readFileSync(newTaskSkillPath, 'utf8');
const readme = fs.readFileSync(readmePath, 'utf8');
const cases = JSON.parse(fs.readFileSync(casesPath, 'utf8'));

const failures = [];
function listValuesAfter(key) {
  const lines = newTaskSkill.split('\n');
  const start = lines.findIndex((line) => line === `${key}:`);
  if (start === -1) {
    failures.push(`new-task skill missing ${key}:`);
    return [];
  }
  const values = [];
  for (const line of lines.slice(start + 1)) {
    if (/^[A-Za-z-]+:/.test(line)) break;
    const match = line.match(/^  - (.+)$/);
    if (match) values.push(match[1]);
  }
  return values;
}

function expectExactSet(label, actual, expected) {
  const missing = expected.filter((value) => !actual.includes(value));
  const extra = actual.filter((value) => !expected.includes(value));
  for (const value of missing) failures.push(`${label} missing ${value}`);
  for (const value of extra) failures.push(`${label} has unexpected ${value}`);
}

function expectPattern(label, pattern) {
  if (!pattern.test(newTaskSkill)) {
    failures.push(`new-task skill should include ${label}`);
  }
}

if (fs.existsSync(shadowedCommandPath)) {
  failures.push('new-task should not have a same-name legacy command shadowed by the skill');
}
if (!newTaskSkill.includes('disable-model-invocation: true')) {
  failures.push('new-task skill should be manually invokable');
}
if (!newTaskSkill.includes('name: new-task')) {
  failures.push('new-task should also ship as a first-class skill');
}
expectExactSet('allowed-tools', listValuesAfter('allowed-tools'), [
  'mcp__tiber__tiber_create',
  'mcp__tiber__tiber_update',
  'mcp__tiber__tiber_acceptance_add',
  'mcp__tiber__tiber_note_add',
  'mcp__tiber__tiber_transition',
  'mcp__tiber__tiber_validate_fix',
  'mcp__tiber__tiber_sync',
  'mcp__tiber__tiber_codex_sandbox_setup',
  'mcp__tiber__tiber_list',
  'mcp__tiber__tiber_show',
  'mcp__plugin_tiber_tiber__tiber_create',
  'mcp__plugin_tiber_tiber__tiber_update',
  'mcp__plugin_tiber_tiber__tiber_acceptance_add',
  'mcp__plugin_tiber_tiber__tiber_note_add',
  'mcp__plugin_tiber_tiber__tiber_transition',
  'mcp__plugin_tiber_tiber__tiber_validate_fix',
  'mcp__plugin_tiber_tiber__tiber_sync',
  'mcp__plugin_tiber_tiber__tiber_codex_sandbox_setup',
  'mcp__plugin_tiber_tiber__tiber_list',
  'mcp__plugin_tiber_tiber__tiber_show',
]);
expectPattern('installed Tiber MCP tools requirement', /Use the installed Tiber\s+MCP tools\./);
expectPattern('file and web tool prohibition', /Do not use file-editing tools or web\/network tools while running this skill\./);
expectPattern('requested task capture initialization boundary', /requested task capture needs Tiber state/);
expectPattern('create may initialize task state', /may initialize that state as part of creating the task/);
expectPattern('validation before board-health claim', /Run the structured Tiber MCP validation tool before claiming the board/);
expectPattern('structured create tool', /structured Tiber MCP create tool/);
expectPattern('structured update acceptance note tools', /structured Tiber MCP update, acceptance, or note/);
expectPattern('structured list or show tools', /structured Tiber MCP list or show tools/);
expectPattern('structured transition tool', /structured Tiber MCP transition tool/);
expectPattern('no CLI fallback', /There is no CLI fallback for this skill\./);
expectPattern('missing MCP tools stop path', /If the needed Tiber MCP tools are\s+unavailable, stop/);
expectPattern('no direct task-file edits', /Do not hand-edit `\.tasks`, `order\.md`, or task markdown files\./);
expectPattern('no shell launchers', /Do not\s+run shell commands, repository-relative launchers/);
expectPattern('untrusted task data handling', /untrusted task data/);
expectPattern('wildcard Bash prohibition', /wildcard Bash permission/);
expectPattern('partial sync created ref recovery', /tiber\.create_sync_failed created=<task-ref>/);
expectPattern('partial sync duplicate-create prohibition', /do not run\s+create again/);
expectPattern('structured sync recovery', /run the structured Tiber MCP\s+sync tool/);
expectPattern('structured sandbox setup recovery', /structured Tiber MCP sandbox setup\s+tool/);
expectPattern('default backlog status', /Leave the new task in `backlog`/);
for (const forbidden of [
  'Bash(<plugin-root>/bin/tiber init)',
  'Bash(<plugin-root>/bin/tiber validate --fix)',
  'Bash(<plugin-root>/bin/tiber list)',
  'Bash(${CLAUDE_SKILL_DIR}/../../bin/tiber init)',
  'Bash(${CLAUDE_SKILL_DIR}/../../bin/tiber validate --fix)',
  'Bash(${CLAUDE_SKILL_DIR}/../../bin/tiber list)',
  'Bash(tiber init)',
  'Bash(tiber validate --fix)',
  'Bash(tiber list)',
  'Bash(<plugin-root>/bin/tiber create *)',
  'Bash(<plugin-root>/bin/tiber update *)',
  'Bash(<plugin-root>/bin/tiber acceptance add *)',
  'Bash(<plugin-root>/bin/tiber note add *)',
  'Bash(<plugin-root>/bin/tiber transition * in-progress)',
  'Bash(<plugin-root>/bin/tiber show *)',
  'Bash(tiber create *)',
  'Bash(tiber update *)',
  'Bash(tiber acceptance add *)',
  'Bash(tiber note add *)',
  'Bash(tiber transition * in-progress)',
  'Bash(tiber show *)',
  'probing `PATH`',
  'tiber show <task-ref>',
]) {
  if (newTaskSkill.includes(forbidden)) {
    failures.push(`new-task skill should not grant wildcard shell permission ${forbidden}`);
  }
}
if (!skill.includes('Invoke the `tiber:new-task` skill')) {
  failures.push('tiber skill should advertise tiber:new-task');
}
if (!skill.includes('tiber.codex_sandbox_setup') || !skill.includes('tiber codex-sandbox --dry-run')) {
  failures.push('tiber skill should advertise Codex sandbox setup discovery');
}
if (!skill.includes('case-by-case approval for raw Git prefixes')) {
  failures.push('tiber skill should require case-by-case approval for raw Git prefixes');
}
if (!skill.includes('Persist approval only when the') || !skill.includes('exact Tiber-internal operation')) {
  failures.push('tiber skill should only allow persisted approvals for exact Tiber-internal operations');
}
if (!skill.includes('not merely to a raw') || !skill.includes('`git` prefix')) {
  failures.push('tiber skill should reject persisted raw git prefix approvals');
}
if (!/do not ask the user to rerun an equivalent/i.test(normalizedSkill)) {
  failures.push('tiber skill should reject manual CLI reruns as normal MCP recovery');
}
if (!/do not recommend running the whole Tiber MCP server outside the sandbox/i.test(normalizedSkill)) {
  failures.push('tiber skill should prefer narrow Git permissions over broad MCP-server escalation');
}
if (!readme.includes('tiber:new-task Document release checklist')) {
  failures.push('tiber README should document tiber:new-task usage');
}
if (!readme.includes('tiber codex-sandbox --dry-run')) {
  failures.push('tiber README should document Codex sandbox setup preview');
}
if (!cases.some((testCase) => testCase.case_id === 'tiber-new-task-command-backlog-capture')) {
  failures.push('missing tiber new-task behavior eval case');
}
if (!cases.some((testCase) => testCase.case_id === 'tiber-codex-sandbox-narrow-setup')) {
  failures.push('missing tiber Codex sandbox behavior eval case');
}
if (!cases.some((testCase) => testCase.case_id === 'tiber-agent-unresolvable-blocked-reason')) {
  failures.push('missing tiber agent-unresolvable blocked reason behavior eval case');
}

if (failures.length > 0) {
  console.error(failures.join('\n'));
  process.exit(1);
}
NODE

  if [ "$status" -ne 0 ]; then
    printf '%s\n' "$output"
  fi
  [ "$status" -eq 0 ]
}
