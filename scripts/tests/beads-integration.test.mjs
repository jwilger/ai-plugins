import assert from "node:assert/strict";
import test from "node:test";

import {
  failClosedCiRecoveryHold,
  parseBeadsIssueList,
  parseBeadsVersion,
  selectActiveCiRecovery,
} from "../../plugins/development-system/extensions/development-system/adapters/beads.ts";
import { parseProjectPolicy } from "../../plugins/development-system/extensions/development-system/core/configuration.ts";
import {
  convertTiberBoard,
  migrateTiberPolicy,
  parseTiberTask,
} from "../../plugins/development-system/bin/migrate-tiber-to-beads.mjs";

const policyV2 = `schema_version = 2
[delivery]
mode = "direct-to-trunk"
trunk_branch = "main"
[features]
worktrees = true
beads = true
agentic_systems = false
eval_case_reporting = false
[worktrees]
root = ".worktrees"
[beads]
workflow = "development-change-direct"
`;

test("schema v2 makes Beads the task integration without retaining Tiber policy", () => {
  const policy = parseProjectPolicy(policyV2);
  assert.equal(policy.schemaVersion, 2);
  assert.equal(policy.features.beads, true);
  assert.equal(policy.beads.workflow, "development-change-direct");
  assert.equal(Object.hasOwn(policy.features, "tiber"), false);
  assert.equal(Object.hasOwn(policy, "tiber"), false);
});

test("legacy Tiber policy fails with one explicit migration action", () => {
  assert.throws(
    () =>
      parseProjectPolicy(`schema_version = 1
[delivery]
mode = "direct-to-trunk"
trunk_branch = "main"
[features]
worktrees = true
tiber = true
agentic_systems = false
eval_case_reporting = false
[worktrees]
root = ".worktrees"
[tiber]
max_queued = 5
`),
    /configuration_legacy_tiber.*migrate-tiber-to-beads/,
  );
});

test("legacy policy migration selects the delivery-specific Beads formula", () => {
  const migrated = migrateTiberPolicy(`schema_version = 1
[delivery]
mode = "pull-request"
trunk_branch = "main"
[features]
worktrees = true
tiber = true
agentic_systems = false
eval_case_reporting = false
[worktrees]
root = ".worktrees"
[tiber]
max_queued = 5
`);
  assert.match(migrated, /^schema_version = 2$/m);
  assert.match(migrated, /^beads = true$/m);
  assert.match(migrated, /\[beads\]\nworkflow = "development-change-pr"/);
  assert.doesNotMatch(migrated, /^tiber\s*=|^\[tiber]/m);
});

test("Beads adapter accepts the stable JSON envelope and validates versions", () => {
  assert.deepEqual(
    parseBeadsIssueList(
      JSON.stringify({
        schema_version: 1,
        data: [
          {
            id: "ai-abc",
            title: "Recover CI",
            status: "in_progress",
            labels: ["development-system:ci-recovery"],
          },
        ],
      }),
    ),
    [
      {
        id: "ai-abc",
        title: "Recover CI",
        status: "in_progress",
        labels: ["development-system:ci-recovery"],
      },
    ],
  );
  assert.deepEqual(parseBeadsVersion("bd version 1.1.2 (abc)"), [1, 1, 2]);
  assert.throws(
    () => parseBeadsVersion("bd version 1.1.1"),
    /beads_version_unsupported/,
  );
});

test("an open claimed CI recovery bead is the repository-wide hold", () => {
  const issue = selectActiveCiRecovery([
    {
      id: "ai-open",
      title: "Unclaimed",
      status: "open",
      labels: ["development-system:ci-recovery"],
    },
    {
      id: "ai-active",
      title: "Repair pushed CI",
      status: "in_progress",
      labels: ["development-system:ci-recovery"],
    },
  ]);
  assert.deepEqual(issue, {
    incidentId: "ai-active",
    state: "in_progress",
  });
  assert.equal(
    selectActiveCiRecovery([
      {
        id: "ai-done",
        title: "Recovered",
        status: "closed",
        labels: ["development-system:ci-recovery"],
      },
    ]),
    null,
  );
});

test("CI recovery coordination failures and ambiguity fail closed", () => {
  assert.deepEqual(failClosedCiRecoveryHold(new Error("backend unavailable")), {
    incidentId: "coordination-unavailable",
    state: "unavailable",
  });
  let ambiguity;
  try {
    selectActiveCiRecovery([
      {
        id: "ai-first",
        title: "First recovery",
        status: "in_progress",
        labels: ["development-system:ci-recovery"],
      },
      {
        id: "ai-second",
        title: "Second recovery",
        status: "in_progress",
        labels: ["development-system:ci-recovery"],
      },
    ]);
  } catch (error) {
    ambiguity = error;
  }
  assert.deepEqual(failClosedCiRecoveryHold(ambiguity), {
    incidentId: "ambiguous-active-incidents",
    state: "ambiguous",
  });
});

const taskSource = `---
title: Repair release flow
blocked_by: []
blocks: [20260730-bbbb-follow-up]
tags: [bug, release]
pr_mr_url: https://example.invalid/pull/42
pr_mr_status: checks-pending
---

## Summary

Keep release state deterministic.

## Context / Why

The old tracker is being retired.

## Acceptance criteria

- [x] Existing state is preserved
- [ ] Beads import succeeds

## Subtasks

- [x] s1: Inspect source
- [ ] s2: Import source (after: s1)

## Notes / Log

- 2026-07-30: Started migration
`;

test("legacy task parsing preserves structured content needed by Beads", () => {
  const task = parseTiberTask(
    "in-progress/20260730-aaaa-repair-release-flow.md",
    taskSource,
  );
  assert.equal(task.id, "20260730-aaaa-repair-release-flow");
  assert.equal(task.status, "in-progress");
  assert.equal(task.title, "Repair release flow");
  assert.deepEqual(task.tags, ["bug", "release"]);
  assert.deepEqual(task.blocks, ["20260730-bbbb-follow-up"]);
  assert.match(task.summary, /deterministic/);
  assert.match(task.acceptanceCriteria, /Beads import succeeds/);
  assert.match(task.notes, /Started migration/);
});

test("Tiber conversion is deterministic, preserves history, and reconstructs dependencies", () => {
  const converted = convertTiberBoard({
    prefix: "ai",
    orderedOpenIds: [
      "20260730-aaaa-repair-release-flow",
      "20260730-bbbb-follow-up",
    ],
    tasks: [
      parseTiberTask(
        "in-progress/20260730-aaaa-repair-release-flow.md",
        taskSource,
      ),
      parseTiberTask(
        "backlog/20260730-bbbb-follow-up.md",
        taskSource
          .replace("Repair release flow", "Follow up after release repair")
          .replace("blocks: [20260730-bbbb-follow-up]", "blocks: []"),
      ),
      parseTiberTask(
        "abandoned/20260729-cccc-old-idea.md",
        taskSource
          .replace("Repair release flow", "Old idea")
          .replace("blocks: [20260730-bbbb-follow-up]", "blocks: []"),
      ),
    ],
  });

  assert.equal(converted.length, 3);
  assert.deepEqual(
    converted.map((issue) => issue.id),
    [
      "ai-tiber-20260730-aaaa-repair-release-flow",
      "ai-tiber-20260730-bbbb-follow-up",
      "ai-tiber-20260729-cccc-old-idea",
    ],
  );
  assert.equal(converted[0].status, "in_progress");
  assert.equal(converted[1].status, "open");
  assert.equal(converted[2].status, "closed");
  assert.ok(converted[2].labels.includes("legacy-tiber:abandoned"));
  assert.deepEqual(converted[1].dependencies, [
    {
      issue_id: "ai-tiber-20260730-bbbb-follow-up",
      depends_on_id: "ai-tiber-20260730-aaaa-repair-release-flow",
      type: "blocks",
    },
  ]);
  assert.equal(converted[0].metadata.legacy_tiber.order, 0);
  assert.equal(converted[1].metadata.legacy_tiber.order, 1);
  assert.equal(converted[0].external_ref, "20260730-aaaa-repair-release-flow");
  assert.equal(converted[0].source_system, "tiber");
});
