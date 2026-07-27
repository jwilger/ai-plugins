import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { chmod, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { resolveStatus } from "./adapters/status-interpreter.ts";
import type { HarnessMode } from "./core/status.ts";

const packageRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../..",
);

function concise(status: Awaited<ReturnType<typeof resolveStatus>>): string {
  const summary = status.configured
    ? `delivery=${status.deliveryMode} checkout=${status.checkout.kind} features=${status.enabledFeatures.join(",") || "none"}`
    : `configuration_missing checkout=${status.checkout.kind}`;
  const errors = status.errors.map((error) => error.code).join(",");
  return errors ? `${summary} errors=${errors}` : summary;
}

async function recordProvenanceMarker(): Promise<void> {
  const marker = process.env.DEVELOPMENT_SYSTEM_PI_EVAL_MARKER;
  if (!marker) return;
  if (!path.isAbsolute(marker))
    throw new Error("development_system.eval_marker_requires_absolute_path");
  await writeFile(
    marker,
    `${JSON.stringify({
      package: "development-system",
      extension: fileURLToPath(import.meta.url),
      version: "1.2.0",
    })}\n`,
    { mode: 0o600 },
  );
  await chmod(marker, 0o600);
}

/** Pi adapter composition root. Domain behavior lives in pure core modules. */
export default function developmentSystemExtension(pi: ExtensionAPI): void {
  let started = false;

  pi.registerCommand("development-system-status", {
    description: "Show deterministic development-system project status",
    handler: async (_arguments, context) => {
      const status = await resolveStatus(
        context.cwd,
        packageRoot,
        context.mode as HarnessMode,
      );
      context.ui.notify(
        JSON.stringify(status, null, 2),
        status.errors.length > 0 ? "warning" : "info",
      );
    },
  });

  pi.registerTool({
    name: "development_system_status",
    label: "Development System Status",
    description:
      "Return bounded project workflow, checkout, component, and enforcement status. This tool is read-only and never grants mutation authority.",
    parameters: { type: "object", properties: {}, additionalProperties: false },
    async execute(_toolCallId, _parameters, signal, _onUpdate, context) {
      if (signal?.aborted) throw new Error("development_system.cancelled");
      const status = await resolveStatus(
        context.cwd,
        packageRoot,
        context.mode as HarnessMode,
      );
      return {
        content: [{ type: "text", text: concise(status) }],
        details: status,
      };
    },
  });

  pi.on("session_start", async (_event, context) => {
    started = true;
    await recordProvenanceMarker();
    const status = await resolveStatus(
      context.cwd,
      packageRoot,
      context.mode as HarnessMode,
    );
    context.ui.setStatus(
      "development-system",
      status.configured
        ? `development-system: ${status.deliveryMode}`
        : "development-system: setup required",
    );
    for (const error of status.errors)
      context.ui.notify(`${error.code}: ${error.nextAction}`, "warning");
    for (const diagnostic of status.diagnostics)
      context.ui.notify(diagnostic, "warning");
  });

  pi.on("session_shutdown", async (_event, context) => {
    if (!started) return;
    started = false;
    context.ui.setStatus("development-system", undefined);
  });
}
