import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";

/** Pi package composition root. Domain behavior lives in sibling core modules. */
export default function developmentSystemExtension(pi: ExtensionAPI): void {
  pi.registerCommand("development-system-status", {
    description: "Show deterministic development-system project status",
    handler: async (_arguments, context) => {
      context.ui.notify(
        `Development System loaded in ${context.mode} mode. Run bin/development-system-pi status for machine-readable status.`,
        "info",
      );
    },
  });
}
