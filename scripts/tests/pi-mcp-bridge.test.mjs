import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import {
  McpClient,
  publicToolName,
  schemaIsAdmissible,
} from "../../plugins/development-system/extensions/development-system/adapters/mcp-client.ts";

function server(source) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "pi-mcp-"));
  const file = path.join(root, "server.mjs");
  fs.writeFileSync(file, source);
  return { root, file };
}

const normalServer = `
let buffer = "";
process.stdin.on("data", chunk => {
  buffer += chunk;
  while (buffer.includes("\\n")) {
    const index = buffer.indexOf("\\n");
    const line = buffer.slice(0, index); buffer = buffer.slice(index + 1);
    if (!line) continue;
    const request = JSON.parse(line);
    if (request.method === "initialize") respond(request.id, { protocolVersion: "2025-06-18", capabilities: { tools: {} }, serverInfo: { name: "fixture", version: "1" } });
    if (request.method === "tools/list") respond(request.id, { tools: [{ name: "final_review.plan", description: "List", inputSchema: { type: "object", properties: {} } }] });
    if (request.method === "tools/call") respond(request.id, { content: [{ type: "text", text: "ok" }], structuredContent: { ok: true } });
  }
});
function respond(id, result) { process.stdout.write(JSON.stringify({ jsonrpc: "2.0", id, result }) + "\\n"); }
`;

test("first-party MCP client dynamically discovers and calls admitted tools", async () => {
  const fixture = server(normalServer);
  const client = new McpClient({
    command: process.execPath,
    args: [fixture.file],
    cwd: fixture.root,
  });
  await client.start();
  const tools = await client.listTools();
  assert.equal(tools[0].name, "final_review.plan");
  assert.equal(schemaIsAdmissible(tools[0].inputSchema), true);
  assert.equal(
    publicToolName(tools[0].name),
    "development_system_review_final_review_plan",
  );
  const result = await client.callTool(tools[0].name, {});
  assert.equal(result.structuredContent.ok, true);
  client.stop();
});

test("schema admission fails closed for ambiguous provider contracts", () => {
  assert.equal(
    schemaIsAdmissible({ type: "object", oneOf: [{ type: "string" }] }),
    false,
  );
  assert.equal(
    schemaIsAdmissible({
      type: "object",
      properties: { value: { $ref: "#/x" } },
    }),
    false,
  );
});

test("MCP timeout terminates the supervised process", async () => {
  const fixture = server(
    "process.stdin.resume(); setInterval(() => {}, 1000);",
  );
  const client = new McpClient({
    command: process.execPath,
    args: [fixture.file],
    cwd: fixture.root,
    startupTimeoutMs: 30,
  });
  await assert.rejects(() => client.start(), /component_timeout/);
  client.stop();
});
