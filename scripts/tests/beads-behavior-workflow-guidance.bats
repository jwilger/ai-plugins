#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  FORMULAS="$ROOT/plugins/development-system/formulas"
}

@test "behavior workflows require their inputs and preserve the red-to-green dependency graph" {
  run python3 - "$FORMULAS" <<'PY'
import pathlib
import sys
import tomllib

root = pathlib.Path(sys.argv[1])


def load(name):
    return tomllib.loads((root / f"{name}.formula.toml").read_text(encoding="utf-8"))


def assert_chain(formula, expected_ids):
    steps = formula["steps"]
    assert [step["id"] for step in steps] == expected_ids
    assert "needs" not in steps[0]
    for previous, current in zip(steps, steps[1:]):
        assert current.get("needs") == [previous["id"]], current["id"]


behavior = load("behavior-slice")
assert behavior["vars"]["behavior"]["required"] is True
assert behavior["vars"]["checkpoint_target"]["required"] is True
assert_chain(
    behavior,
    [
        "write-acceptance-test",
        "prove-acceptance-red",
        "implement-scenario-steps",
        "prove-acceptance-green",
        "run-complete-test-gate",
        "checkpoint",
    ],
)
acceptance_contract = behavior["steps"][0]["description"].lower()
assert "prefer gherkin when the project supports it" in acceptance_contract
assert "established acceptance-test convention" in acceptance_contract

bdd_step = load("bdd-step-cycle")
assert bdd_step["vars"]["scenario_step"]["required"] is True
assert_chain(
    bdd_step,
    [
        "wire-step",
        "observe-next-failure",
        "drive-implementation",
        "focused-green",
        "refactor",
    ],
)
PY
  [ "$status" -eq 0 ]
}

@test "delivery workflows preserve ordered completion and mode-specific checkpoint behavior" {
  run python3 - "$FORMULAS" <<'PY'
import pathlib
import sys
import tomllib

root = pathlib.Path(sys.argv[1])


def load(name):
    return tomllib.loads((root / f"{name}.formula.toml").read_text(encoding="utf-8"))


def assert_chain(formula, expected_ids):
    steps = formula["steps"]
    assert [step["id"] for step in steps] == expected_ids
    assert "needs" not in steps[0]
    for previous, current in zip(steps, steps[1:]):
        assert current.get("needs") == [previous["id"]], current["id"]


cases = {
    "development-change-direct": {
        "ids": [
            "preflight",
            "classify-change-slices",
            "complete-change-slices",
            "verify",
            "final-review",
            "confirm-trunk-delivery",
            "prove-pushed-ci",
            "finish",
        ],
        "checkpoint": ("push each completed checkpoint to main",),
    },
    "development-change-pr": {
        "ids": [
            "preflight",
            "classify-change-slices",
            "complete-change-slices",
            "verify",
            "final-review",
            "open-pull-request",
            "prove-pr-ci",
            "approval",
            "merge",
            "prove-post-merge-ci",
            "finish",
        ],
        "checkpoint": ("push each completed checkpoint to the feature branch",),
    },
    "development-change-local": {
        "ids": [
            "preflight",
            "classify-change-slices",
            "complete-change-slices",
            "verify",
            "final-review",
            "record-local-delivery",
            "finish",
        ],
        "checkpoint": ("create local commits", "without pushing"),
    },
}

for name, contract in cases.items():
    formula = load(name)
    assert formula["vars"]["work_item"]["required"] is True
    assert_chain(formula, contract["ids"])
    steps = {step["id"]: step for step in formula["steps"]}
    checkpoint = steps["complete-change-slices"]["description"].lower()
    for phrase in contract["checkpoint"]:
        assert phrase in checkpoint, (name, phrase)
    assert steps["final-review"]["needs"] == ["verify"]
PY
  [ "$status" -eq 0 ]
}

@test "ordinary ready-work guidance excludes coordination sentinels before ordering and claiming" {
  run python3 - "$ROOT/AGENTS.md" "$ROOT/plugins/development-system/skills/beads/SKILL.md" <<'PY'
import pathlib
import sys

command = "`bd ready --exclude-label gt:slot --json`"
for source in map(pathlib.Path, sys.argv[1:]):
    text = " ".join(source.read_text(encoding="utf-8").split()).lower()
    command_at = text.index(command)
    order_at = text.index("priority ascending", command_at)
    claim_at = text.index("claim", order_at)
    assert command_at < order_at < claim_at
    assert "ordinary ready" in text[max(0, command_at - 180):command_at]
    sentinel_contract = text[max(0, command_at - 220):command_at + 360]
    assert "persistent coordination" in sentinel_contract
    assert "open" in sentinel_contract
    assert "unclaimed" in sentinel_contract
    assert "unclosed" in sentinel_contract
PY
  [ "$status" -eq 0 ]
}
