#!/usr/bin/env node

import readline from "node:readline";

let threadId = "thread-fixture";
let nextTurn = 0;
let account = null;
const fixtureMode =
  process.env.TIBER_FIXTURE_MODE ??
  process.argv.find((argument) => argument.startsWith("--mode="))?.slice(7) ??
  "success";
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
    if (fixtureMode === "chatty") {
      const timer = setInterval(() => send({ method: "fixture/progress" }), 25);
      setTimeout(() => clearInterval(timer), 5_000);
      return;
    }
    send({
      id: message.id,
      result: {
        codexHome:
          fixtureMode === "wrong-home" ? "/unexpected" : process.env.CODEX_HOME,
        platformFamily: "unix",
        platformOs: "linux",
        userAgent:
          fixtureMode === "wrong-version"
            ? "fixture/0.148.0 compatibility/0.147.0"
            : "fixture/0.147.0",
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
  } else if (message.method === "account/read") {
    send({ id: message.id, result: { account, requiresOpenaiAuth: true } });
  } else if (message.method === "account/login/start") {
    if (message.params.type === "apiKey") {
      if (fixtureMode === "credential-rejection") {
        send({
          error: { code: -32602, message: `invalid ${message.params.apiKey}` },
          id: message.id,
        });
        return;
      }
      account = { type: "apiKey" };
      send({ id: message.id, result: { type: "apiKey" } });
    } else {
      send({
        id: message.id,
        result: {
          authUrl: "https://example.invalid/login",
          loginId: "login-fixture",
          type: "chatgpt",
        },
      });
      send({
        method: "account/login/completed",
        params:
          fixtureMode === "idless-login-failure"
            ? { error: "fixture login denied", success: false }
            : { loginId: "login-fixture", success: true },
      });
    }
  } else if (message.method === "account/logout") {
    account = null;
    send({ id: message.id, result: {} });
  } else if (message.method === "thread/start") {
    if (fixtureMode === "id-collision") {
      send({
        id: message.id,
        method: "item/commandExecution/requestApproval",
        params: { threadId, turnId: "turn-1" },
      });
      return;
    }
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
      method: "item/agentMessage/delta",
      params: {
        delta: "foreign text",
        itemId: "foreign-assistant",
        threadId: "foreign-thread",
        turnId: "foreign-turn",
      },
    });
    if (fixtureMode !== "success") {
      send({
        id: "foreign-dynamic",
        method: "item/tool/call",
        params: {
          arguments: { foreign: true },
          callId: "foreign-call",
          threadId: "foreign-thread",
          tool: "tiber_effect",
          turnId: "foreign-turn",
        },
      });
    }
    send({
      method: "item/agentMessage/delta",
      params: {
        delta: "hello from Tiber",
        itemId: `assistant-${nextTurn}`,
        threadId,
        turnId,
      },
    });
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
  } else if (fixtureMode === "id-collision" && message.result?.decision === "decline") {
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
  }
});
