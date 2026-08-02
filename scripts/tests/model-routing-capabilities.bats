#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  ROUTER="$ROOT/evals/promptfoo/model-routing-capabilities.cjs"
  FIXTURE="$ROOT/scripts/tests/fixtures/codex-model-capabilities.json"
}

@test "strongest model resolution uses authoritative priority rather than model names or list order" {
  run env CODEX_MODEL_CAPABILITIES_FILE="$FIXTURE" node - "$ROUTER" <<'NODE'
const { strongestEligibleCodexModel } = require(process.argv[2]);
const resolved = strongestEligibleCodexModel('xhigh');
if (resolved.model !== 'codex-strong-current' || resolved.priority !== 1) {
  throw new Error(`unexpected strongest route: ${JSON.stringify(resolved)}`);
}
NODE

  [ "$status" -eq 0 ]
}

@test "a newly advertised higher-capability model becomes the expected strong route without code changes" {
  run node - "$ROUTER" "$FIXTURE" "$BATS_TEST_TMPDIR/future-models.json" <<'NODE'
const fs = require('fs');
const router = process.argv[2];
const source = process.argv[3];
const target = process.argv[4];
const document = JSON.parse(fs.readFileSync(source, 'utf8'));
document.models.unshift({
  slug: 'future-frontier-model',
  visibility: 'list',
  priority: 0,
  upgrade: null,
  supported_reasoning_levels: [{ effort: 'high' }, { effort: 'xhigh' }],
});
fs.writeFileSync(target, JSON.stringify(document));
process.env.CODEX_MODEL_CAPABILITIES_FILE = target;
const { strongestEligibleCodexModel } = require(router);
const resolved = strongestEligibleCodexModel('high');
if (resolved.model !== 'future-frontier-model') {
  throw new Error(`new strongest route was not selected: ${JSON.stringify(resolved)}`);
}
NODE

  [ "$status" -eq 0 ]
}

@test "installed dispatch assertions follow a newly advertised strongest model" {
  run node - "$ROOT" "$FIXTURE" "$BATS_TEST_TMPDIR/future-dispatch-models.json" <<'NODE'
const fs = require('fs');
const root = process.argv[2];
const source = process.argv[3];
const target = process.argv[4];
const document = JSON.parse(fs.readFileSync(source, 'utf8'));
document.models.unshift({
  slug: 'future-frontier-model',
  visibility: 'list',
  priority: 0,
  upgrade: null,
  supported_reasoning_levels: [{ effort: 'high' }, { effort: 'xhigh' }],
});
fs.writeFileSync(target, JSON.stringify(document));
process.env.CODEX_MODEL_CAPABILITIES_FILE = target;

const assertInstalledDispatch = require(`${root}/evals/promptfoo/assert-installed-dispatch.cjs`);
function context(model) {
  return {
    vars: { case_id: 'advisor-installed-delegated-dispatch' },
    provider: { id: () => 'openai:codex-sdk' },
    providerResponse: {
      raw: JSON.stringify({
        items: [
          {
            type: 'command_execution',
            command: 'cat /tmp/codex/plugins/cache/ai-plugins/development-system/3.1.0/skills/advisor/SKILL.md',
            status: 'completed',
            exit_code: 0,
          },
          {
            type: 'collab_tool_call',
            tool: 'spawn_agent',
            arguments: {
              task_name: 'advisor',
              model,
              reasoning_effort: 'xhigh',
              fork_turns: 'none',
              message: 'Review the plan.',
            },
            receiver_thread_ids: ['thread-1'],
            status: 'completed',
          },
          {
            type: 'collab_tool_call',
            tool: 'wait',
            receiver_thread_ids: ['thread-1'],
            agents_states: {
              'thread-1': { status: 'completed', output: 'Reviewed plan' },
            },
            status: 'completed',
          },
        ],
      }),
    },
  };
}

const stale = assertInstalledDispatch('', context('codex-strong-current'));
const current = assertInstalledDispatch('', context('future-frontier-model'));
if (stale.pass || !current.pass) {
  throw new Error(`dynamic dispatch mismatch: stale=${JSON.stringify(stale)} current=${JSON.stringify(current)}`);
}
NODE

  [ "$status" -eq 0 ]
}

@test "ambiguous strongest metadata fails closed instead of guessing from names or order" {
  run node - "$ROUTER" "$FIXTURE" "$BATS_TEST_TMPDIR/ambiguous-models.json" <<'NODE'
const fs = require('fs');
const router = process.argv[2];
const source = process.argv[3];
const target = process.argv[4];
const document = JSON.parse(fs.readFileSync(source, 'utf8'));
document.models.unshift({
  slug: 'lexically-later-model',
  visibility: 'list',
  priority: 1,
  upgrade: null,
  supported_reasoning_levels: [{ effort: 'high' }, { effort: 'xhigh' }],
});
fs.writeFileSync(target, JSON.stringify(document));
process.env.CODEX_MODEL_CAPABILITIES_FILE = target;
const { strongestEligibleCodexModel } = require(router);
try {
  strongestEligibleCodexModel('xhigh');
  throw new Error('ambiguous metadata unexpectedly resolved');
} catch (error) {
  if (!String(error.message).includes('ambiguous highest-capability priority')) {
    throw error;
  }
}
NODE

  [ "$status" -eq 0 ]
}
