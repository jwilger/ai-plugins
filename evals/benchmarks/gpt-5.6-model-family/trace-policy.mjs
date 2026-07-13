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
