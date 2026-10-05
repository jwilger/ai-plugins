// Pi adapter for Development System.
//
// This file owns no policy. It translates pi lifecycle events into calls to the
// same plugin-root launchers and CLI that the Codex plugin uses, so every
// deterministic check stays in the shared shell/Rust core.
import { readFileSync } from "node:fs";
import { dirname, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

import type { McpStdioServerConfig, PiContext, PiExtensionApi } from "./pi-api.ts";

const pluginRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const cli = join(pluginRoot, "bin/development-system");
const warningPrefix = "development_system.warning ";
const sessionStartTimeoutMs = 900_000;
const doctorTimeoutMs = 60_000;
const usage = "usage: /development-system doctor";

interface McpDeclaration {
  type?: string;
  command?: string;
  args?: string[];
  url?: string;
}

interface McpManifest {
  mcpServers: Record<string, McpDeclaration>;
}

/** Map the portable plugin-root mcp.json onto pi registrations. */
export function mcpRegistrations(root: string, manifest: McpManifest): Array<[string, McpStdioServerConfig]> {
  return Object.entries(manifest.mcpServers).map(([name, declared]) => {
    if ((declared.type ?? "stdio") !== "stdio" || typeof declared.command !== "string") {
      throw new Error(`development_system.pi_unsupported_mcp_transport server=${name}`);
    }
    let command = declared.command;
    if (command.startsWith("./")) {
      command = resolve(root, command);
      if (relative(root, command).split(sep).includes("..")) {
        throw new Error(`development_system.pi_mcp_command_escapes_plugin server=${name}`);
      }
    }
    // Codex declares plugin MCP tools directly; keep that parity in pi.
    return [name, { command, args: declared.args ?? [], exposure: "direct" }];
  });
}

/** Extract the warning payloads from the shared CLI's text output. */
export function warningsFrom(stdout: string): string[] {
  return stdout
    .split("\n")
    .filter((line) => line.startsWith(warningPrefix))
    .map((line) => line.slice(warningPrefix.length));
}

async function runCheck(
  pi: PiExtensionApi,
  ctx: PiContext,
  subcommand: "session-start" | "doctor",
  timeout: number,
): Promise<void> {
  const notify = (message: string, type: "info" | "warning" | "error") =>
    ctx.ui.notify(`development-system: ${message}`, type);
  try {
    const outcome = await pi.exec(cli, [subcommand, "--harness", "pi", "--project", ctx.cwd], {
      cwd: ctx.cwd,
      timeout,
    });
    if (outcome.code !== 0) {
      notify(outcome.stderr.trim() || `${subcommand} exited with status ${outcome.code}`, "error");
      return;
    }
    const warnings = warningsFrom(outcome.stdout);
    warnings.forEach((warning) => notify(warning, "warning"));
    if (warnings.length === 0 && subcommand === "doctor") notify("no conflicts found", "info");
  } catch (error) {
    notify(error instanceof Error ? error.message : String(error), "error");
  }
}

export default function developmentSystem(pi: PiExtensionApi): void {
  const manifest = JSON.parse(readFileSync(join(pluginRoot, "mcp.json"), "utf8")) as McpManifest;
  for (const [name, config] of mcpRegistrations(pluginRoot, manifest)) {
    pi.registerMcpServer(name, config);
  }

  // Binary repair can download a release bundle; never block session startup on it.
  pi.on("session_start", (_event, ctx) => {
    void runCheck(pi, ctx, "session-start", sessionStartTimeoutMs);
  });

  pi.registerCommand("development-system", {
    description: "Development System diagnostics (doctor)",
    handler: async (args, ctx) => {
      const subcommand = args.trim() || "doctor";
      if (subcommand !== "doctor") {
        ctx.ui.notify(`development-system: unknown subcommand '${subcommand}'; ${usage}`, "error");
        return;
      }
      await runCheck(pi, ctx, "doctor", doctorTimeoutMs);
    },
  });
}
