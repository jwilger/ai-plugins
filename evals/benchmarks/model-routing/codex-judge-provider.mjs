import TraceEnforcedCodexProvider from "../gpt-5.6-model-family/trace-enforced-codex-provider.mjs";

function requiredAbsolutePath(environment, name) {
  const value = environment[name];
  if (typeof value !== "string" || !value.startsWith("/")) {
    throw new Error(`model-routing Codex judge requires absolute ${name}`);
  }
  return value;
}

function retryCount(raw) {
  let trace;
  try {
    trace = typeof raw === "string" ? JSON.parse(raw) : raw;
  } catch {
    return 0;
  }
  if (!Array.isArray(trace?.notifications)) return 0;
  return trace.notifications.filter(
    (notification) =>
      notification?.method === "error" &&
      notification?.params?.willRetry === true,
  ).length;
}

function normalizedUsage(tokenUsage) {
  const prompt = tokenUsage?.prompt;
  const cached = tokenUsage?.cached ?? 0;
  const completion = tokenUsage?.completion;
  if (
    ![prompt, cached, completion].every(
      (value) => Number.isInteger(value) && value >= 0,
    ) ||
    cached > prompt
  ) {
    throw new Error("model-routing Codex judge received invalid token usage");
  }
  return {
    input_tokens: prompt,
    cached_input_tokens: cached,
    output_tokens: completion,
  };
}

export function createCodexJudgeProviderFunction({
  ProviderClass = TraceEnforcedCodexProvider,
  environment = process.env,
  now = Date.now,
} = {}) {
  const codexHome = requiredAbsolutePath(
    environment,
    "MODEL_ROUTING_CODEX_HOME",
  );
  const workspace = requiredAbsolutePath(
    environment,
    "MODEL_ROUTING_WORKSPACE",
  );

  return async function run(request) {
    if (
      request?.isolation?.plugins !== false ||
      request?.isolation?.tools !== false ||
      request?.isolation?.workspace !== false
    ) {
      throw new Error("model-routing Codex judge requires full isolation");
    }
    const config = {
      model: request.model,
      working_dir: workspace,
      deep_tracing: false,
      skip_git_repo_check: true,
      cli_config: { features: { plugins: false } },
      cli_env: { CODEX_HOME: codexHome },
    };
    if (request.effort !== "none") {
      config.model_reasoning_effort = request.effort;
    }
    const provider = new ProviderClass({
      id: `model-routing-judge-${request.model}-${request.effort}`,
      config,
    });
    const startedAt = now();
    try {
      const response = await provider.callApi(request.prompt, {}, {});
      if (response?.error) {
        throw new Error(`model-routing Codex judge failed: ${response.error}`);
      }
      if (typeof response?.output !== "string") {
        throw new Error("model-routing Codex judge returned no text output");
      }
      return {
        status: "success",
        output: response.output,
        usage: normalizedUsage(response.tokenUsage),
        elapsed_ms: Math.max(0, now() - startedAt),
        attempts: 1 + retryCount(response.raw),
      };
    } finally {
      await provider.cleanup?.();
    }
  };
}

export default async function runWithEnvironment(request) {
  return createCodexJudgeProviderFunction()(request);
}
