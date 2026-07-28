import { spawn } from "node:child_process";

export type ReviewModelRoute = Readonly<{
  provider: string;
  model: string;
  thinking: string;
}>;
export type ReviewAssignment = Readonly<{
  assignment: string;
  modelRole: string;
}>;
export type ReviewChildLifecycle = Readonly<{
  state:
    | "completed"
    | "cancelled"
    | "timed-out"
    | "output-limited"
    | "provider-failed"
    | "malformed-result"
    | "spawn-failed";
  startedAt: string;
  endedAt: string;
  elapsedMs: number;
  processStarted: boolean;
  terminationRequested: boolean;
  exitCode: number | null;
  signal: string | null;
  stdoutBytes: number;
  stderrBytes: number;
}>;
export type ReviewResult = Readonly<{
  status: "completed";
  result: Record<string, unknown>;
  lifecycle: ReviewChildLifecycle;
  attestation: Readonly<{
    model_role: string;
    fresh_context: true;
    closed_after_result: true;
  }>;
}>;
export type ReviewFailure = Readonly<{
  status: "failed";
  code: string;
  reason:
    | "parent-abort"
    | "timeout"
    | "output-limit"
    | "provider-exit"
    | "malformed-result"
    | "spawn-error";
  lifecycle: ReviewChildLifecycle;
  retry: string;
}>;

export class ReviewChildError extends Error {
  readonly details: ReviewFailure;

  constructor(details: ReviewFailure) {
    super(details.code);
    this.name = "ReviewChildError";
    this.details = details;
  }
}

const defaultRoutes: Readonly<Record<string, ReviewModelRoute>> = Object.freeze(
  {
    "bounded-helper": {
      provider: "openai-codex",
      model: "gpt-5.6-luna",
      thinking: "medium",
    },
    "substantive-worker": {
      provider: "openai-codex",
      model: "gpt-5.6-terra",
      thinking: "high",
    },
    "strong-reviewer": {
      provider: "openai-codex",
      model: "gpt-5.6-sol",
      thinking: "high",
    },
    "strong-worker": {
      provider: "openai-codex",
      model: "gpt-5.6-sol",
      thinking: "high",
    },
  },
);

export function resolveReviewRoute(
  modelRole: string,
  overrides: Readonly<Record<string, string>> = {},
): ReviewModelRoute {
  const configured = overrides[modelRole];
  if (configured) {
    const separator = configured.indexOf("/");
    if (separator <= 0 || separator === configured.length - 1)
      throw new Error(
        `development_system.review_model_route_invalid role=${modelRole}`,
      );
    return {
      provider: configured.slice(0, separator),
      model: configured.slice(separator + 1),
      thinking: "high",
    };
  }
  const route = defaultRoutes[modelRole];
  if (!route)
    throw new Error(
      `development_system.review_model_role_unmapped role=${modelRole}`,
    );
  return route;
}

function retryFor(reason: ReviewFailure["reason"]): string {
  if (reason === "parent-abort")
    return "Confirm whether the parent turn was intentionally interrupted, then rerun the assignment from a fresh child.";
  if (reason === "timeout")
    return "Reduce the assignment scope or explicitly increase the bounded child timeout before retrying.";
  if (reason === "output-limit")
    return "Require a smaller structured result and rerun the assignment.";
  if (reason === "provider-exit" || reason === "spawn-error")
    return "Inspect provider availability and the selected route, then rerun; the review remains unresolved.";
  return "Correct the assignment result contract and rerun; the review remains unresolved.";
}

export function reviewFailureResult(error: unknown): ReviewFailure | null {
  return error instanceof ReviewChildError ? error.details : null;
}

export async function runReviewChild(
  options: Readonly<{
    assignment: ReviewAssignment;
    cwd: string;
    route: ReviewModelRoute;
    signal?: AbortSignal;
    piBinary?: string;
    timeoutMs?: number;
  }>,
): Promise<ReviewResult> {
  const piBinary =
    options.piBinary ?? process.env.DEVELOPMENT_SYSTEM_PI_BIN ?? "pi";
  const timeoutMs = options.timeoutMs ?? 10 * 60_000;
  const prompt = `${options.assignment.assignment}\n\nReturn only one JSON object matching the coordinator assignment. Do not use parent conversation state.`;
  const started = Date.now();
  const startedAt = new Date(started).toISOString();
  return new Promise((resolve, reject) => {
    const child = spawn(
      piBinary,
      [
        "--provider",
        options.route.provider,
        "--model",
        options.route.model,
        "--thinking",
        options.route.thinking,
        "--no-session",
        "--print",
        prompt,
      ],
      {
        cwd: options.cwd,
        detached: process.platform !== "win32",
        stdio: ["ignore", "pipe", "pipe"],
      },
    );
    let stdout = "";
    let stdoutBytes = 0;
    let stderrBytes = 0;
    let settled = false;
    let terminationRequested = false;
    let exitCode: number | null = null;
    let exitSignal: string | null = null;
    const lifecycle = (
      state: ReviewChildLifecycle["state"],
    ): ReviewChildLifecycle => {
      const ended = Date.now();
      return {
        state,
        startedAt,
        endedAt: new Date(ended).toISOString(),
        elapsedMs: Math.max(0, ended - started),
        processStarted: typeof child.pid === "number",
        terminationRequested,
        exitCode,
        signal: exitSignal,
        stdoutBytes,
        stderrBytes,
      };
    };
    const stop = () => {
      terminationRequested = true;
      if (child.killed) return;
      if (process.platform !== "win32" && child.pid) {
        try {
          process.kill(-child.pid, "SIGTERM");
        } catch {
          child.kill("SIGTERM");
        }
      } else child.kill("SIGTERM");
    };
    const failure = (
      code: string,
      reason: ReviewFailure["reason"],
      state: ReviewChildLifecycle["state"],
    ) =>
      new ReviewChildError({
        status: "failed",
        code,
        reason,
        lifecycle: lifecycle(state),
        retry: retryFor(reason),
      });
    const timer = setTimeout(() => {
      stop();
      finish(
        failure(
          "development_system.review_child_timeout",
          "timeout",
          "timed-out",
        ),
      );
    }, timeoutMs);
    const finish = (error?: Error) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      options.signal?.removeEventListener("abort", aborted);
      if (error) reject(error);
    };
    const aborted = () => {
      stop();
      finish(
        failure(
          "development_system.review_child_cancelled",
          "parent-abort",
          "cancelled",
        ),
      );
    };
    if (options.signal?.aborted) aborted();
    else options.signal?.addEventListener("abort", aborted, { once: true });
    child.stdout.on("data", (chunk: Buffer) => {
      const text = chunk.toString("utf8");
      stdout += text;
      stdoutBytes += Buffer.byteLength(text);
      if (stdoutBytes > 50 * 1024) {
        stop();
        finish(
          failure(
            "development_system.review_child_output_limit",
            "output-limit",
            "output-limited",
          ),
        );
      }
    });
    child.stderr.on("data", (chunk: Buffer) => {
      stderrBytes += chunk.byteLength;
    });
    child.on("error", () =>
      finish(
        failure(
          "development_system.review_child_spawn_failed",
          "spawn-error",
          "spawn-failed",
        ),
      ),
    );
    child.on("close", (code, signal) => {
      exitCode = code;
      exitSignal = signal;
      if (settled) return;
      if (code !== 0) {
        finish(
          failure(
            "development_system.review_child_provider_failed",
            "provider-exit",
            "provider-failed",
          ),
        );
        return;
      }
      let result: unknown;
      try {
        result = JSON.parse(stdout.trim());
      } catch {
        finish(
          failure(
            "development_system.review_child_result_malformed",
            "malformed-result",
            "malformed-result",
          ),
        );
        return;
      }
      if (!result || typeof result !== "object" || Array.isArray(result)) {
        finish(
          failure(
            "development_system.review_child_result_malformed",
            "malformed-result",
            "malformed-result",
          ),
        );
        return;
      }
      finish();
      resolve({
        status: "completed",
        result: result as Record<string, unknown>,
        lifecycle: lifecycle("completed"),
        attestation: {
          model_role: options.assignment.modelRole,
          fresh_context: true,
          closed_after_result: true,
        },
      });
    });
  });
}
