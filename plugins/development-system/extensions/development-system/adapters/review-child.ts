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
export type ReviewResult = Readonly<{
  result: Record<string, unknown>;
  attestation: Readonly<{
    model_role: string;
    fresh_context: true;
    closed_after_result: true;
  }>;
}>;

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
    let stderr = "";
    let settled = false;
    const stop = () => {
      if (child.killed) return;
      if (process.platform !== "win32" && child.pid) {
        try {
          process.kill(-child.pid, "SIGTERM");
        } catch {
          child.kill("SIGTERM");
        }
      } else child.kill("SIGTERM");
    };
    const timer = setTimeout(() => {
      stop();
      finish(new Error("development_system.review_child_timeout"));
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
      finish(new Error("development_system.review_child_cancelled"));
    };
    options.signal?.addEventListener("abort", aborted, { once: true });
    child.stdout.on("data", (chunk) => {
      stdout += chunk.toString("utf8");
      if (Buffer.byteLength(stdout) > 50 * 1024) {
        stop();
        finish(new Error("development_system.review_child_output_limit"));
      }
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk.toString("utf8");
    });
    child.on("error", (error) => finish(error));
    child.on("close", (code) => {
      if (settled) return;
      if (code !== 0) {
        finish(
          new Error(
            `development_system.review_child_provider_failed code=${code} detail=${stderr.slice(0, 512)}`,
          ),
        );
        return;
      }
      let result: unknown;
      try {
        result = JSON.parse(stdout.trim());
      } catch {
        finish(new Error("development_system.review_child_result_malformed"));
        return;
      }
      if (!result || typeof result !== "object" || Array.isArray(result)) {
        finish(new Error("development_system.review_child_result_malformed"));
        return;
      }
      finish();
      resolve({
        result: result as Record<string, unknown>,
        attestation: {
          model_role: options.assignment.modelRole,
          fresh_context: true,
          closed_after_result: true,
        },
      });
    });
  });
}
