#!/usr/bin/env node

import readline from "node:readline";

let threadId = "thread-fixture";
let nextTurn = 0;
const fixtureMode = process.env.TIBER_FIXTURE_MODE ?? "success";
const input = readline.createInterface({ input: process.stdin });

if (fixtureMode === "ignored-term") {
  process.on("SIGTERM", () => {});
}

function send(message) {
  process.stdout.write(`${JSON.stringify(message)}\n`);
}

function completeTurn(turnId) {
  send({
    method: "turn/completed",
    params: {
      threadId,
      turn: { id: turnId, items: [], status: "completed" },
    },
  });
}

input.on("line", (line) => {
  if (fixtureMode === "silent" || fixtureMode === "ignored-term") return;
  if (fixtureMode === "early-close") process.exit(3);
  if (fixtureMode === "malformed") {
    process.stdout.write("not-json\n");
    return;
  }
  const message = JSON.parse(line);
  if (message.method === "initialize") {
    send({
      id: message.id,
      result: {
        codexHome:
          fixtureMode === "wrong-home" ? "/unexpected" : process.env.CODEX_HOME,
        platformFamily: "unix",
        platformOs: "linux",
        userAgent: "fixture/0.147.0",
      },
    });
  } else if (message.method === "permissionProfile/list") {
    send({
      id: message.id,
      result: {
        data: [
          {
            allowed: true,
            description: "Read-only, offline inference for the Tiber harness",
            id: "tiber-inference",
          },
        ],
      },
    });
  } else if (message.method === "thread/start") {
    send({
      id: message.id,
      result: {
        activePermissionProfile: { extends: null, id: "tiber-inference" },
        approvalPolicy: "never",
        approvalsReviewer: "user",
        sandbox: { networkAccess: false, type: "readOnly" },
        thread: { id: threadId },
      },
    });
  } else if (message.method === "command/exec") {
    const isControl = message.params.command.includes("process.exit(0)");
    if (fixtureMode === "control-failure" && isControl) {
      send({ id: message.id, result: { exitCode: 1 } });
      return;
    }
    if (fixtureMode === "command-timeout" && !isControl) return;
    if (fixtureMode === "command-malformed" && !isControl) {
      send({ id: message.id, result: {} });
      return;
    }
    if (fixtureMode === "command-error" && !isControl) {
      send({
        error: { code: -32601, message: "command/exec unavailable" },
        id: message.id,
      });
      return;
    }
    send({
      id: message.id,
      result: {
        exitCode: isControl ? 0 : 1,
        stderr: isControl ? "" : "write denied by fixture sandbox",
        stdout: "",
      },
    });
  } else if (message.method === "turn/start") {
    nextTurn += 1;
    const turnId = `turn-${nextTurn}`;
    send({ id: message.id, result: { turn: { id: turnId } } });
    send({
      method: "item/started",
      params: {
        item: { id: `user-${nextTurn}`, type: "userMessage" },
        threadId,
        turnId,
      },
    });
    if (nextTurn === 1) {
      send({
        id: "dynamic-fixture",
        method: "item/tool/call",
        params: {
          arguments: { action: "sentinel" },
          callId: "call-fixture",
          namespace: null,
          threadId,
          tool: "tiber_authority_probe",
          turnId,
        },
      });
      if (fixtureMode === "close-after-request") {
        setImmediate(() => process.exit(4));
      }
    } else {
      completeTurn(turnId);
    }
  } else if (message.id === "dynamic-fixture") {
    if (message.result?.success !== false) process.exitCode = 1;
    completeTurn("turn-1");
  }
});
