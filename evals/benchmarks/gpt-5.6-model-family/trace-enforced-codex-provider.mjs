import { loadApiProvider } from "promptfoo";
import { isAbsolute } from "node:path";
import { fileURLToPath } from "node:url";
import {
  allowedNormalizedItemTypes,
  allowedRawResponseItemTypes,
  toolNotificationDescription,
} from "./trace-policy.mjs";

// package-lock pins @openai/codex-sdk -> @openai/codex 0.144.3, whose npm
// binary is linked here. Promptfoo otherwise resolves a bare `codex` on PATH.
const pinnedCodexPath = fileURLToPath(
  new URL("../../../node_modules/.bin/codex", import.meta.url),
);

const disabledFeatures = [
  "shell_tool",
  "unified_exec",
  "multi_agent",
  "enable_fanout",
  "apps",
  "enable_mcp_apps",
  "in_app_browser",
  "browser_use",
  "browser_use_full_cdp_access",
  "browser_use_external",
  "computer_use",
  "image_generation",
  "tool_suggest",
  "remote_plugin",
  "goals",
  "memories",
  "deferred_executor",
  "request_permissions_tool",
  "default_mode_request_user_input",
  "current_time_reminder",
  "skill_mcp_dependency_install",
  "hooks",
];

function isolatedConfig(config) {
  const cliConfig = config.cli_config ?? {};
  const cliEnv = config.cli_env ?? {};
  const features = cliConfig.features ?? {};
  const tools = cliConfig.tools ?? {};
  const forcedFeatures = Object.fromEntries(
    disabledFeatures.map((feature) => [feature, false]),
  );

  return {
    ...config,
    codex_path_override: pinnedCodexPath,
    sandbox_mode: "read-only",
    approval_policy: "never",
    inherit_process_env: false,
    reuse_server: false,
    network_access_enabled: false,
    ephemeral: true,
    experimental_raw_events: true,
    include_raw_events: true,
    cli_env: {
      ...cliEnv,
      HOME: cliEnv.CODEX_HOME,
    },
    cli_config: {
      ...cliConfig,
      web_search: "disabled",
      features: {
        ...features,
        ...forcedFeatures,
        plugins: features.plugins === true,
        multi_agent_v2: {
          ...features.multi_agent_v2,
          enabled: false,
        },
        token_budget: {
          ...features.token_budget,
          enabled: false,
        },
        code_mode: {
          ...features.code_mode,
          enabled: false,
        },
        code_mode_only: false,
      },
      tools: {
        ...tools,
        experimental_request_user_input: {
          ...tools.experimental_request_user_input,
          enabled: false,
        },
      },
    },
  };
}

function withoutPromptConfig(context) {
  if (!context?.prompt || !Object.hasOwn(context.prompt, "config")) {
    return context;
  }

  const { config: _ignoredConfig, ...prompt } = context.prompt;
  return { ...context, prompt };
}

/**
 * Promptfoo provider wrapper that starts every Codex app-server thread and turn
 * without an execution environment and rejects any tool use observed in the
 * app-server trace. Codex 0.144.3 still exposes code-mode tools for GPT-5.6;
 * this wrapper makes their use a provider error rather than claiming the model
 * has no tools.
 */
export default class TraceEnforcedCodexProvider {
  #innerProviderPromise;

  constructor(options = {}, providerLoader = loadApiProvider) {
    this.options = options;
    this.providerLoader = providerLoader;
  }

  id() {
    return this.options.id ?? "trace-enforced-codex-app-server";
  }

  #getInnerProvider() {
    if (!this.#innerProviderPromise) {
      this.#innerProviderPromise = this.#initializeInnerProvider();
    }

    return this.#innerProviderPromise;
  }

  async #initializeInnerProvider() {
    const { basePath, ...config } = this.options.config ?? {};
    const codexHome = config.cli_env?.CODEX_HOME;
    if (
      typeof codexHome !== "string" ||
      codexHome.trim() === "" ||
      codexHome.includes("{{") ||
      codexHome.includes("}}") ||
      !isAbsolute(codexHome)
    ) {
      throw new Error(
        "trace-enforced Codex provider requires CODEX_HOME to be a resolved absolute path",
      );
    }
    const inner = await this.providerLoader("openai:codex-app-server", {
      basePath,
      env: this.options.env,
      options: {
        id: this.id(),
        label: this.options.label,
        config: isolatedConfig(config),
        env: this.options.env,
      },
    });

    for (const methodName of [
      "buildThreadStartParams",
      "buildTurnStartParams",
    ]) {
      if (typeof inner[methodName] !== "function") {
        throw new Error(
          `Codex app-server provider does not expose ${methodName}()`,
        );
      }

      const buildParams = inner[methodName].bind(inner);
      inner[methodName] = (...args) => ({
        ...buildParams(...args),
        environments: [],
      });
    }

    return inner;
  }

  async callApi(prompt, context, callOptions) {
    const inner = await this.#getInnerProvider();
    const response = await inner.callApi(
      prompt,
      withoutPromptConfig(context),
      callOptions,
    );

    if (response?.error) {
      return response;
    }

    let raw;
    try {
      raw =
        typeof response?.raw === "string"
          ? JSON.parse(response.raw)
          : response?.raw;
    } catch {
      return {
        ...response,
        error: "trace-enforced Codex provider received an unverifiable trace",
      };
    }

    if (!raw || !Array.isArray(raw.items)) {
      return {
        ...response,
        error: "trace-enforced Codex provider received an unverifiable trace",
      };
    }

    const rejectedItem = raw?.items?.find(
      (item) => !allowedNormalizedItemTypes.has(item?.type),
    );

    if (rejectedItem) {
      return {
        ...response,
        error: `trace-enforced Codex provider rejected ${rejectedItem.type ?? "unknown"} trace item`,
      };
    }

    const rejectedNotification = Array.isArray(raw.notifications)
      ? toolNotificationDescription(raw.notifications)
      : undefined;
    if (rejectedNotification) {
      return {
        ...response,
        error: `trace-enforced Codex provider rejected ${rejectedNotification} notification`,
      };
    }

    const rawResponseNotifications = Array.isArray(raw.notifications)
      ? raw.notifications.filter(
          (notification) =>
            notification?.method === "rawResponseItem/completed",
        )
      : [];
    const rawResponseItems = rawResponseNotifications.map(
      (notification) => notification?.params?.item,
    );

    if (
      rawResponseItems.length === 0 ||
      rawResponseItems.some(
        (item) => !item || typeof item.type !== "string" || !item.type,
      )
    ) {
      return {
        ...response,
        error:
          "trace-enforced Codex provider received an unverifiable raw response trace",
      };
    }

    const rejectedRawItem = rawResponseItems.find(
      (item) => !allowedRawResponseItemTypes.has(item?.type),
    );

    if (rejectedRawItem) {
      return {
        ...response,
        error: `trace-enforced Codex provider rejected raw ${rejectedRawItem.type ?? "unknown"} item`,
      };
    }

    if (!Array.isArray(raw.serverRequests)) {
      return {
        ...response,
        error:
          "trace-enforced Codex provider received an unverifiable server request trace",
      };
    }
    if (raw.serverRequests.length > 0) {
      return {
        ...response,
        error: "trace-enforced Codex provider rejected a server request trace",
      };
    }

    return response;
  }

  async cleanup() {
    const inner = await this.#getInnerProvider();
    return inner.cleanup?.();
  }
}
