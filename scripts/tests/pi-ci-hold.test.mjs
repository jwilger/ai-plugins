import assert from "node:assert/strict";
import test from "node:test";
import { parseCiRecoveryHold } from "../../plugins/development-system/extensions/development-system/adapters/ci-hold.ts";

test("active Tiber incidents create a hold until exact terminal success", () => {
  assert.deepEqual(
    parseCiRecoveryHold(
      '{"incident_id":"ci-42","state":"waiting-ci","hold_released":false}',
    ),
    {
      incidentId: "ci-42",
      state: "waiting-ci",
    },
  );
  assert.equal(
    parseCiRecoveryHold(
      '{"incident_id":"ci-42","state":"resolved","hold_released":true,"release_proof":{"terminal_status":"success"}}',
    ),
    null,
  );
});

test("missing or malformed component state never invents an incident", () => {
  assert.equal(parseCiRecoveryHold("not-json"), null);
  assert.equal(parseCiRecoveryHold('{"state":"active"}'), null);
});
