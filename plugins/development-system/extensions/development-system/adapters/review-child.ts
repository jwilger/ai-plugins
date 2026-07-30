import { spawn } from "node:child_process";
import { StringDecoder } from "node:string_decoder";
import {
  consumeReviewEvent,
  initialReviewEventState,
  refreshReviewProgress,
  type ReviewEventState,
  type ReviewProgress,
} from "../core/review-progress.ts";

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
  progress: ReviewProgress;
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
  progress: ReviewProgress;
  retry: string;
}>;

const MAX_PROTOCOL_LINE_BYTES = 4 * 1024 * 1024;
const MAX_PROTOCOL_STREAM_BYTES = 64 * 1024 * 1024;
const MAX_STDERR_BYTES = 128 * 1024;

type PendingFailure = Readonly<{
  code: string;
  reason: ReviewFailure["reason"];
  state: ReviewChildLifecycle["state"];
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
    terminationGraceMs?: number;
    progressIntervalMs?: number;
    progressThrottleMs?: number;
    protocolLineLimitBytes?: number;
    protocolStreamLimitBytes?: number;
    stderrLimitBytes?: number;
    onProgress?: (progress: ReviewProgress) => void;
  }>,
): Promise<ReviewResult> {
  const piBinary =
    options.piBinary ?? process.env.DEVELOPMENT_SYSTEM_PI_BIN ?? "pi";
  const timeoutMs = options.timeoutMs ?? 10 * 60_000;
  const terminationGraceMs = Math.max(10, options.terminationGraceMs ?? 1_000);
  const progressIntervalMs = Math.max(10, options.progressIntervalMs ?? 1_000);
  const progressThrottleMs =
    options.progressThrottleMs === 0
      ? 0
      : Math.max(10, options.progressThrottleMs ?? 250);
  const protocolLineLimitBytes = Math.max(
    1,
    options.protocolLineLimitBytes ?? MAX_PROTOCOL_LINE_BYTES,
  );
  const protocolStreamLimitBytes = Math.max(
    1,
    options.protocolStreamLimitBytes ?? MAX_PROTOCOL_STREAM_BYTES,
  );
  const stderrLimitBytes = Math.max(
    1,
    options.stderrLimitBytes ?? MAX_STDERR_BYTES,
  );
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
        "--mode",
        "json",
        "--print",
        prompt,
      ],
      {
        cwd: options.cwd,
        detached: process.platform !== "win32",
        stdio: ["ignore", "pipe", "pipe"],
      },
    );
    const stdoutDecoder = new StringDecoder("utf8");
    let stdoutBuffer = "";
    let stdoutBytes = 0;
    let stderrBytes = 0;
    let settled = false;
    let terminationRequested = false;
    let exitCode: number | null = null;
    let exitSignal: string | null = null;
    let eventState: ReviewEventState = initialReviewEventState();
    let pendingFailure: PendingFailure | null = null;
    const readPendingFailure = (): PendingFailure | null => pendingFailure;
    let forceKillTimer: NodeJS.Timeout | undefined;
    let timeoutTimer: NodeJS.Timeout | undefined;
    let progressTimer: NodeJS.Timeout | undefined;
    let progressFlushTimer: NodeJS.Timeout | undefined;
    let lastProgressEmittedAt = 0;
    let flushFinalProgress: () => void = () => {};

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
        progress: eventState.progress,
        retry: retryFor(reason),
      });
    const signalProcessGroup = (signal: NodeJS.Signals) => {
      terminationRequested = true;
      if (process.platform !== "win32" && child.pid) {
        try {
          process.kill(-child.pid, signal);
          return;
        } catch {
          // The child may have exited between observation and signaling.
        }
      }
      try {
        child.kill(signal);
      } catch {
        // Close/error remains the authoritative process boundary.
      }
    };
    const processGroupExists = () => {
      if (process.platform === "win32" || !child.pid) return false;
      try {
        process.kill(-child.pid, 0);
        return true;
      } catch {
        return false;
      }
    };
    const waitForProcessGroupExit = async () => {
      const deadline = Date.now() + terminationGraceMs;
      while (processGroupExists() && Date.now() < deadline) {
        await new Promise((resolveDelay) => setTimeout(resolveDelay, 20));
      }
      return !processGroupExists();
    };
    const closeRemainingProcessGroup = async (forceImmediately: boolean) => {
      if (!processGroupExists()) return true;
      signalProcessGroup(forceImmediately ? "SIGKILL" : "SIGTERM");
      if (await waitForProcessGroupExit()) return true;
      signalProcessGroup("SIGKILL");
      return waitForProcessGroupExit();
    };
    const finish = (error?: Error) => {
      if (settled) return;
      flushFinalProgress();
      settled = true;
      if (timeoutTimer) clearTimeout(timeoutTimer);
      if (forceKillTimer) clearTimeout(forceKillTimer);
      if (progressTimer) clearInterval(progressTimer);
      if (progressFlushTimer) clearTimeout(progressFlushTimer);
      options.signal?.removeEventListener("abort", aborted);
      if (error) reject(error);
    };
    const requestTermination = (
      code: string,
      reason: ReviewFailure["reason"],
      state: ReviewChildLifecycle["state"],
    ) => {
      if (settled || pendingFailure) return;
      pendingFailure = { code, reason, state };
      signalProcessGroup("SIGTERM");
      forceKillTimer = setTimeout(() => {
        if (!settled) signalProcessGroup("SIGKILL");
      }, terminationGraceMs);
    };
    const aborted = () => {
      requestTermination(
        "development_system.review_child_cancelled",
        "parent-abort",
        "cancelled",
      );
    };
    const publishProgress = () => {
      lastProgressEmittedAt = Date.now();
      try {
        options.onProgress?.(eventState.progress);
      } catch {
        // Parent rendering failures do not alter child execution or evidence.
      }
    };
    const emitProgress = (force = false) => {
      if (!options.onProgress) return;
      const elapsed = Date.now() - lastProgressEmittedAt;
      if (
        force ||
        progressThrottleMs === 0 ||
        lastProgressEmittedAt === 0 ||
        elapsed >= progressThrottleMs
      ) {
        if (progressFlushTimer) clearTimeout(progressFlushTimer);
        progressFlushTimer = undefined;
        publishProgress();
        return;
      }
      if (!progressFlushTimer) {
        progressFlushTimer = setTimeout(() => {
          progressFlushTimer = undefined;
          if (!settled) publishProgress();
        }, progressThrottleMs - elapsed);
      }
    };
    flushFinalProgress = () => emitProgress(true);

    timeoutTimer = setTimeout(() => {
      requestTermination(
        "development_system.review_child_timeout",
        "timeout",
        "timed-out",
      );
    }, timeoutMs);
    progressTimer = setInterval(() => {
      if (settled || pendingFailure) return;
      eventState = refreshReviewProgress(
        eventState,
        Math.max(0, Date.now() - started),
      );
      emitProgress();
    }, progressIntervalMs);
    if (options.signal?.aborted) aborted();
    else options.signal?.addEventListener("abort", aborted, { once: true });

    const processEventLine = (line: string) => {
      if (!line.trim() || settled || pendingFailure) return;
      let event: unknown;
      try {
        event = JSON.parse(line);
      } catch {
        requestTermination(
          "development_system.review_child_result_malformed",
          "malformed-result",
          "malformed-result",
        );
        return;
      }
      const update = consumeReviewEvent(
        eventState,
        event,
        Math.max(0, Date.now() - started),
      );
      eventState = update.state;
      if (update.progressChanged) emitProgress();
      if (eventState.outputLimited) {
        requestTermination(
          "development_system.review_child_output_limit",
          "output-limit",
          "output-limited",
        );
      }
    };
    child.stdout.on("data", (chunk: Buffer) => {
      if (settled || pendingFailure) return;
      const text = stdoutDecoder.write(chunk);
      stdoutBytes += chunk.byteLength;
      stdoutBuffer += text;
      if (
        stdoutBytes > protocolStreamLimitBytes ||
        Buffer.byteLength(stdoutBuffer, "utf8") > protocolLineLimitBytes
      ) {
        requestTermination(
          "development_system.review_child_output_limit",
          "output-limit",
          "output-limited",
        );
        return;
      }
      let newline = stdoutBuffer.indexOf("\n");
      while (newline >= 0 && !settled && !pendingFailure) {
        const line = stdoutBuffer.slice(0, newline).replace(/\r$/, "");
        stdoutBuffer = stdoutBuffer.slice(newline + 1);
        processEventLine(line);
        newline = stdoutBuffer.indexOf("\n");
      }
    });
    child.stderr.on("data", (chunk: Buffer) => {
      if (settled || pendingFailure) return;
      stderrBytes += chunk.byteLength;
      if (stderrBytes > stderrLimitBytes) {
        requestTermination(
          "development_system.review_child_output_limit",
          "output-limit",
          "output-limited",
        );
      }
    });
    child.on("exit", () => {
      if (settled || pendingFailure || !processGroupExists()) return;
      signalProcessGroup("SIGTERM");
      forceKillTimer = setTimeout(() => {
        if (!settled && processGroupExists()) signalProcessGroup("SIGKILL");
      }, terminationGraceMs);
    });
    child.on("error", () => {
      const requested = pendingFailure;
      finish(
        requested
          ? failure(requested.code, requested.reason, requested.state)
          : failure(
              "development_system.review_child_spawn_failed",
              "spawn-error",
              "spawn-failed",
            ),
      );
    });
    child.on("close", async (code, signal) => {
      exitCode = code;
      exitSignal = signal;
      if (settled) return;
      eventState = refreshReviewProgress(
        eventState,
        Math.max(0, Date.now() - started),
      );
      const processGroupClosed = await closeRemainingProcessGroup(
        pendingFailure !== null,
      );
      if (settled) return;
      if (!processGroupClosed) {
        finish(
          failure(
            "development_system.review_child_process_group_not_closed",
            "provider-exit",
            "provider-failed",
          ),
        );
        return;
      }
      if (pendingFailure) {
        const requested = pendingFailure;
        finish(failure(requested.code, requested.reason, requested.state));
        return;
      }
      stdoutBuffer += stdoutDecoder.end();
      if (stdoutBuffer.trim())
        processEventLine(stdoutBuffer.replace(/\r$/, ""));
      const requestedAfterFinalLine = readPendingFailure();
      if (requestedAfterFinalLine) {
        finish(
          failure(
            requestedAfterFinalLine.code,
            requestedAfterFinalLine.reason,
            requestedAfterFinalLine.state,
          ),
        );
        return;
      }
      if (eventState.progress.state !== "settled") {
        const settledUpdate = consumeReviewEvent(
          eventState,
          { type: "agent_settled" },
          Math.max(0, Date.now() - started),
        );
        eventState = settledUpdate.state;
        emitProgress();
      }
      if (
        code !== 0 ||
        eventState.terminalStopReason === "error" ||
        eventState.terminalStopReason === "aborted"
      ) {
        finish(
          failure(
            "development_system.review_child_provider_failed",
            "provider-exit",
            "provider-failed",
          ),
        );
        return;
      }
      if (eventState.outputLimited) {
        finish(
          failure(
            "development_system.review_child_output_limit",
            "output-limit",
            "output-limited",
          ),
        );
        return;
      }
      let result: unknown;
      try {
        result = JSON.parse(eventState.finalText ?? "");
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
        progress: eventState.progress,
        attestation: {
          model_role: options.assignment.modelRole,
          fresh_context: true,
          closed_after_result: true,
        },
      });
    });
  });
}
