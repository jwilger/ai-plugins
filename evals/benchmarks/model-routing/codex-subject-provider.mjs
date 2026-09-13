import { loadApiProvider } from "promptfoo";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const pinnedCodexPath = fileURLToPath(
  new URL("../../../node_modules/.bin/codex", import.meta.url),
);

function requiredAbsolutePath(environment, name) {
  const value = environment[name];
  if (typeof value !== "string" || !path.isAbsolute(value)) {
    throw new Error(`model-routing Codex subject requires absolute ${name}`);
  }
  return value;
}

function requiredToolPath(environment) {
  const value = environment.MODEL_ROUTING_TOOL_PATH;
  if (
    typeof value !== "string" ||
    value.length === 0 ||
    value
      .split(":")
      .some(
        (entry) => !path.isAbsolute(entry) || !entry.startsWith("/nix/store/"),
      )
  ) {
    throw new Error(
      "model-routing Codex subject requires an absolute MODEL_ROUTING_TOOL_PATH",
    );
  }
  return value;
}

function pathsOverlap(first, second) {
  return (
    first === second ||
    first.startsWith(`${second}${path.sep}`) ||
    second.startsWith(`${first}${path.sep}`)
  );
}

function canonicalDirectory(candidate, label) {
  let metadata;
  try {
    metadata = fs.lstatSync(candidate);
  } catch {
    throw new Error(`model-routing Codex subject cannot read ${label}`);
  }
  if (
    !metadata.isDirectory() ||
    metadata.isSymbolicLink() ||
    fs.realpathSync(candidate) !== candidate
  ) {
    throw new Error(`model-routing Codex subject requires canonical ${label}`);
  }
  return candidate;
}

function assertSafeFixtureTree(directory) {
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const candidate = path.join(directory, entry.name);
    if (entry.isSymbolicLink()) {
      throw new Error("model-routing Codex subject fixture contains a symlink");
    }
    if (entry.isDirectory()) {
      assertSafeFixtureTree(candidate);
    } else if (!entry.isFile()) {
      throw new Error(
        "model-routing Codex subject fixture contains a special file",
      );
    }
  }
}

function defaultPrepareWorkspace(request, environment) {
  if (!/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(request.fixture ?? "")) {
    throw new Error(
      "model-routing Codex subject received an unsafe fixture id",
    );
  }
  const fixtureRoot = requiredAbsolutePath(
    environment,
    "MODEL_ROUTING_FIXTURE_ROOT",
  );
  const workRoot = requiredAbsolutePath(environment, "MODEL_ROUTING_WORK_ROOT");
  fs.mkdirSync(workRoot, { recursive: true, mode: 0o700 });
  canonicalDirectory(fixtureRoot, "MODEL_ROUTING_FIXTURE_ROOT");
  canonicalDirectory(workRoot, "MODEL_ROUTING_WORK_ROOT");
  const codexHome = requiredAbsolutePath(
    environment,
    "MODEL_ROUTING_CODEX_HOME",
  );
  if (
    pathsOverlap(fixtureRoot, workRoot) ||
    pathsOverlap(fixtureRoot, codexHome) ||
    pathsOverlap(workRoot, codexHome)
  ) {
    throw new Error(
      "model-routing Codex subject fixture, work, and auth roots must not overlap",
    );
  }
  const source = canonicalDirectory(
    path.join(fixtureRoot, request.fixture),
    "fixture directory",
  );
  assertSafeFixtureTree(source);
  const workspace = path.join(
    workRoot,
    crypto.createHash("sha256").update(request.job_id).digest("hex"),
  );
  if (!fs.existsSync(workspace)) {
    const temporary = `${workspace}.tmp-${process.pid}`;
    try {
      fs.cpSync(source, temporary, {
        recursive: true,
        errorOnExist: true,
        preserveTimestamps: true,
      });
      fs.renameSync(temporary, workspace);
    } finally {
      fs.rmSync(temporary, { recursive: true, force: true });
    }
  }
  return workspace;
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
    throw new Error("model-routing Codex subject received invalid token usage");
  }
  return {
    input_tokens: prompt,
    cached_input_tokens: cached,
    output_tokens: completion,
  };
}

function sandboxFor(surface) {
  if (surface === "workspace-read") return "read-only";
  if (surface === "workspace-write") return "workspace-write";
  throw new Error(`model-routing Codex subject rejects surface: ${surface}`);
}

export function createCodexSubjectProviderFunction({
  providerLoader = loadApiProvider,
  prepareWorkspace = defaultPrepareWorkspace,
  environment = process.env,
  now = Date.now,
} = {}) {
  const codexHome = requiredAbsolutePath(
    environment,
    "MODEL_ROUTING_CODEX_HOME",
  );
  const toolPath = requiredToolPath(environment);

  return async function run(request) {
    const workspace = prepareWorkspace(request, environment);
    const config = {
      codex_path_override: pinnedCodexPath,
      model: request.model,
      working_dir: workspace,
      sandbox_mode: sandboxFor(request.execution_surface),
      approval_policy: "never",
      network_access_enabled: false,
      inherit_process_env: false,
      reuse_server: false,
      ephemeral: true,
      experimental_raw_events: true,
      include_raw_events: true,
      skip_git_repo_check: true,
      cli_env: {
        CODEX_HOME: codexHome,
        HOME: codexHome,
        PATH: toolPath,
      },
      cli_config: {
        web_search: "disabled",
        features: {
          plugins: true,
          multi_agent: false,
          enable_fanout: false,
          apps: false,
          browser_use: false,
          computer_use: false,
          image_generation: false,
          remote_plugin: false,
          goals: false,
          memories: false,
          deferred_executor: false,
          hooks: false,
          multi_agent_v2: { enabled: false },
          token_budget: { enabled: false },
        },
      },
    };
    if (request.effort !== "none") {
      config.model_reasoning_effort = request.effort;
    }
    const inner = await providerLoader("openai:codex-app-server", {
      options: {
        id: `model-routing-subject-${request.model}-${request.effort}`,
        config,
      },
    });
    for (const methodName of [
      "buildThreadStartParams",
      "buildTurnStartParams",
    ]) {
      if (typeof inner[methodName] !== "function") continue;
      const buildParams = inner[methodName].bind(inner);
      inner[methodName] = (...args) => ({
        ...buildParams(...args),
        environments: [],
      });
    }
    const startedAt = now();
    try {
      const response = await inner.callApi(request.prompt, {}, {});
      if (response?.error) {
        throw new Error(
          `model-routing Codex subject failed: ${response.error}`,
        );
      }
      if (typeof response?.output !== "string") {
        throw new Error("model-routing Codex subject returned no text output");
      }
      return {
        status: "success",
        output: response.output,
        usage: normalizedUsage(response.tokenUsage),
        elapsed_ms: Math.max(0, now() - startedAt),
        attempts: 1 + retryCount(response.raw),
        workspace,
      };
    } finally {
      await inner.cleanup?.();
    }
  };
}

export default async function runWithEnvironment(request) {
  return createCodexSubjectProviderFunction()(request);
}
