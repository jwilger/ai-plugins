export type ReviewProgressState =
  | "starting"
  | "running"
  | "tool-running"
  | "tool-completed"
  | "responding"
  | "settled";

export type ReviewProgressEvent = Readonly<{
  sequence: number;
  state: ReviewProgressState;
  message: string;
  elapsedMs: number;
  currentTool?: string;
  toolError?: boolean;
}>;

export type ReviewProgress = Readonly<{
  state: ReviewProgressState;
  elapsedMs: number;
  turns: number;
  toolCalls: number;
  toolErrors: number;
  activeToolCount: number;
  observedEvents: number;
  droppedEvents: number;
  recentEvents: readonly ReviewProgressEvent[];
  currentTool?: string;
}>;

type ActiveTool = Readonly<{
  id: string;
  label: string;
}>;

export type ReviewEventState = Readonly<{
  progress: ReviewProgress;
  activeTools: readonly ActiveTool[];
  activeToolCount: number;
  finalText: string | null;
  terminalStopReason: string | null;
  outputLimited: boolean;
}>;

export type ReviewEventUpdate = Readonly<{
  state: ReviewEventState;
  progressChanged: boolean;
}>;

const MAX_RECENT_EVENTS = 20;
const MAX_ACTIVE_TOOLS = 32;
const MAX_RESULT_BYTES = 50 * 1024;
const KNOWN_BUILTIN_TOOLS = new Set([
  "read",
  "bash",
  "edit",
  "write",
  "grep",
  "find",
  "ls",
]);

export function initialReviewEventState(): ReviewEventState {
  return {
    progress: {
      state: "starting",
      elapsedMs: 0,
      turns: 0,
      toolCalls: 0,
      toolErrors: 0,
      activeToolCount: 0,
      observedEvents: 0,
      droppedEvents: 0,
      recentEvents: [],
    },
    activeTools: [],
    activeToolCount: 0,
    finalText: null,
    terminalStopReason: null,
    outputLimited: false,
  };
}

function safeToolLabel(value: unknown): string {
  if (typeof value !== "string") return "unknown-tool";
  if (KNOWN_BUILTIN_TOOLS.has(value)) return value;
  if (value.startsWith("development_system_")) return "development-system";
  if (value.startsWith("final_review.")) return "final-review";
  if (value.startsWith("tiber.")) return "tiber";
  return "extension-tool";
}

function toolCallId(value: unknown, sequence: number): string {
  return typeof value === "string" && value.length > 0
    ? value.slice(0, 256)
    : `anonymous-${sequence}`;
}

function assistantMessage(value: unknown): Readonly<{
  text: string | null;
  stopReason: string | null;
}> | null {
  if (!value || typeof value !== "object") return null;
  const message = value as Record<string, unknown>;
  if (message.role !== "assistant") return null;
  const text = Array.isArray(message.content)
    ? message.content
        .filter((part): part is { type: "text"; text: string } =>
          Boolean(
            part &&
            typeof part === "object" &&
            (part as Record<string, unknown>).type === "text" &&
            typeof (part as Record<string, unknown>).text === "string",
          ),
        )
        .map((part) => part.text)
        .join("")
    : "";
  return {
    text: text || null,
    stopReason:
      typeof message.stopReason === "string" ? message.stopReason : null,
  };
}

function withProgressEvent(
  state: ReviewEventState,
  event: Omit<ReviewProgressEvent, "sequence">,
  counts: Partial<
    Pick<ReviewProgress, "turns" | "toolCalls" | "toolErrors">
  > = {},
): ReviewEventState {
  const observedEvents = state.progress.observedEvents + 1;
  const progressEvent: ReviewProgressEvent = {
    ...event,
    sequence: observedEvents,
  };
  const recentEvents = [...state.progress.recentEvents, progressEvent].slice(
    -MAX_RECENT_EVENTS,
  );
  return {
    ...state,
    progress: {
      state: event.state,
      elapsedMs: Math.max(state.progress.elapsedMs, event.elapsedMs),
      turns: counts.turns ?? state.progress.turns,
      toolCalls: counts.toolCalls ?? state.progress.toolCalls,
      toolErrors: counts.toolErrors ?? state.progress.toolErrors,
      activeToolCount: state.activeToolCount,
      observedEvents,
      droppedEvents: observedEvents - recentEvents.length,
      recentEvents,
      ...(event.currentTool ? { currentTool: event.currentTool } : {}),
    },
  };
}

export function refreshReviewProgress(
  current: ReviewEventState,
  elapsedMs: number,
): ReviewEventState {
  if (elapsedMs <= current.progress.elapsedMs) return current;
  return {
    ...current,
    progress: {
      ...current.progress,
      elapsedMs,
    },
  };
}

export function consumeReviewEvent(
  current: ReviewEventState,
  input: unknown,
  elapsedMs: number,
): ReviewEventUpdate {
  if (!input || typeof input !== "object" || Array.isArray(input))
    return { state: current, progressChanged: false };
  const event = input as Record<string, unknown>;
  if (event.type === "agent_start") {
    return {
      state: withProgressEvent(current, {
        state: "running",
        message: "Fresh child started",
        elapsedMs,
      }),
      progressChanged: true,
    };
  }
  if (event.type === "turn_start") {
    return {
      state: withProgressEvent(
        current,
        {
          state: "running",
          message: "Child reasoning",
          elapsedMs,
        },
        { turns: current.progress.turns + 1 },
      ),
      progressChanged: true,
    };
  }
  if (event.type === "tool_execution_start") {
    const label = safeToolLabel(event.toolName);
    const id = toolCallId(event.toolCallId, current.progress.toolCalls + 1);
    const duplicate = current.activeTools.some((tool) => tool.id === id);
    const activeTools = duplicate
      ? current.activeTools
      : [...current.activeTools, { id, label }].slice(-MAX_ACTIVE_TOOLS);
    const activeToolCount = duplicate
      ? current.activeToolCount
      : Math.min(Number.MAX_SAFE_INTEGER, current.activeToolCount + 1);
    return {
      state: withProgressEvent(
        { ...current, activeTools, activeToolCount },
        {
          state: "tool-running",
          message: `Running ${label}`,
          elapsedMs,
          currentTool: label,
        },
        { toolCalls: current.progress.toolCalls + 1 },
      ),
      progressChanged: true,
    };
  }
  if (event.type === "tool_execution_end") {
    const completedLabel = safeToolLabel(event.toolName);
    const id =
      typeof event.toolCallId === "string"
        ? event.toolCallId.slice(0, 256)
        : null;
    const matchingIndex = id
      ? current.activeTools.findIndex((tool) => tool.id === id)
      : current.activeTools.findIndex((tool) => tool.label === completedLabel);
    const activeTools =
      matchingIndex < 0
        ? current.activeTools
        : current.activeTools.filter((_tool, index) => index !== matchingIndex);
    const completionMatched =
      matchingIndex >= 0 ||
      current.activeToolCount > current.activeTools.length;
    const activeToolCount = completionMatched
      ? Math.max(0, current.activeToolCount - 1)
      : current.activeToolCount;
    const remaining = activeTools.at(-1);
    const remainingLabel =
      activeToolCount > activeTools.length ? "active-tools" : remaining?.label;
    const toolError = event.isError === true;
    const completed = completionMatched
      ? `${toolError ? "Failed" : "Completed"} ${completedLabel}`
      : `Ignored unmatched completion for ${completedLabel}`;
    return {
      state: withProgressEvent(
        { ...current, activeTools, activeToolCount },
        {
          state: activeToolCount > 0 ? "tool-running" : "tool-completed",
          message: remainingLabel
            ? `${completed}; running ${remainingLabel}`
            : completed,
          elapsedMs,
          ...(remainingLabel
            ? { currentTool: remainingLabel }
            : { currentTool: completedLabel }),
          toolError,
        },
        {
          toolErrors:
            current.progress.toolErrors +
            (toolError && completionMatched ? 1 : 0),
        },
      ),
      progressChanged: true,
    };
  }
  if (event.type === "message_end") {
    const message = assistantMessage(event.message);
    if (!message) return { state: current, progressChanged: false };
    const bytes = message.text ? Buffer.byteLength(message.text, "utf8") : 0;
    return {
      state: withProgressEvent(
        {
          ...current,
          finalText:
            message.text && bytes <= MAX_RESULT_BYTES ? message.text : null,
          terminalStopReason: message.stopReason ?? current.terminalStopReason,
          outputLimited: current.outputLimited || bytes > MAX_RESULT_BYTES,
        },
        {
          state: "responding",
          message: "Child response ended",
          elapsedMs,
        },
      ),
      progressChanged: true,
    };
  }
  if (event.type === "agent_settled") {
    return {
      state: withProgressEvent(
        { ...current, activeTools: [], activeToolCount: 0 },
        {
          state: "settled",
          message: "Child settled",
          elapsedMs,
        },
      ),
      progressChanged: true,
    };
  }
  return { state: current, progressChanged: false };
}
