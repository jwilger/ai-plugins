export const allowedNormalizedItemTypes = new Set([
  "user_message",
  "agent_message",
  "reasoning",
]);

export const allowedRawResponseItemTypes = new Set(["message", "reasoning"]);

const allowedNotificationItemTypes = new Set([
  "userMessage",
  "agentMessage",
  "reasoning",
]);

const benignNotificationMethods = new Set([
  "error",
  "thread/started",
  "thread/status/changed",
  "thread/tokenUsage/updated",
  "turn/started",
  "turn/completed",
  "item/started",
  "item/completed",
  "rawResponseItem/completed",
  "item/agentMessage/delta",
  "item/reasoning/summaryTextDelta",
  "item/reasoning/summaryPartAdded",
  "item/reasoning/textDelta",
  "turn/moderationMetadata",
  "model/safetyBuffering/updated",
  "warning",
  "deprecationNotice",
  "configWarning",
]);

export function toolNotificationDescription(notifications) {
  for (const notification of notifications) {
    const method = notification?.method;
    if (typeof method !== "string" || !method) {
      return "unknown notification";
    }
    if (!benignNotificationMethods.has(method)) {
      return method;
    }
    if (method === "item/started" || method === "item/completed") {
      const itemType = notification?.params?.item?.type;
      if (!allowedNotificationItemTypes.has(itemType)) {
        return `${method}:${itemType ?? "unknown"}`;
      }
    }
  }

  return undefined;
}

function requiredIdentifier(value) {
  return typeof value === "string" && value.length > 0 ? value : undefined;
}

function turnIdentifier(notification) {
  return requiredIdentifier(
    notification?.params?.turn?.id ?? notification?.params?.turnId,
  );
}

function threadIdentifier(notification) {
  return requiredIdentifier(notification?.params?.threadId);
}

export function turnLifecycleRejection(notifications) {
  if (!Array.isArray(notifications)) {
    return "missing notification trace";
  }

  const started = notifications
    .map((notification, index) => ({ notification, index }))
    .filter(({ notification }) => notification?.method === "turn/started");
  const completed = notifications
    .map((notification, index) => ({ notification, index }))
    .filter(({ notification }) => notification?.method === "turn/completed");

  if (started.length === 0) {
    return "missing turn/started notification";
  }
  if (started.length > 1) {
    return "duplicate turn/started notifications";
  }
  if (completed.length === 0) {
    return "missing turn/completed notification";
  }
  if (completed.length > 1) {
    return "duplicate turn/completed notifications";
  }

  const start = started[0];
  const completion = completed[0];
  const startedThreadId = threadIdentifier(start.notification);
  const completedThreadId = threadIdentifier(completion.notification);
  const startedTurnId = turnIdentifier(start.notification);
  const completedTurnId = turnIdentifier(completion.notification);

  if (!startedThreadId || !startedTurnId) {
    return "invalid turn/started notification";
  }
  if (!completedThreadId || !completedTurnId) {
    return "invalid turn/completed notification";
  }
  if (start.index >= completion.index) {
    return "turn/completed notification precedes turn/started";
  }
  if (startedThreadId !== completedThreadId) {
    return "thread identifiers do not match";
  }
  if (startedTurnId !== completedTurnId) {
    return "turn identifiers do not match";
  }

  const completionStatus = completion.notification?.params?.turn?.status;
  if (completionStatus !== "completed") {
    return `turn/completed status ${
      typeof completionStatus === "string" && completionStatus.length > 0
        ? completionStatus
        : "unknown"
    }`;
  }

  for (const [index, notification] of notifications.entries()) {
    if (notification?.method !== "error") {
      continue;
    }
    if (notification?.params?.willRetry !== true) {
      return "non-retryable error notification";
    }
    if (index <= start.index || index >= completion.index) {
      return "retryable error outside active turn";
    }
    const errorThreadId = threadIdentifier(notification);
    const errorTurnId = turnIdentifier(notification);
    if (errorThreadId !== startedThreadId || errorTurnId !== startedTurnId) {
      return "retryable error identifiers do not match active turn";
    }
  }

  return undefined;
}
