const { caseById } = require("./fixtures.cjs");

function result(pass, reason) {
  return { pass, score: pass ? 1 : 0, reason };
}

function exactPublicName(candidate, expected) {
  if (typeof candidate !== "string") return false;
  const normalized = candidate.trim();
  return (
    normalized === expected || normalized === `development-system:${expected}`
  );
}

function successfulClaudeCall(call) {
  return (
    call &&
    typeof call === "object" &&
    call.is_error !== true &&
    Object.hasOwn(call, "output") &&
    nonemptyClaudeOutput(call.output)
  );
}

function nonemptyClaudeOutput(value) {
  if (nonemptyString(value)) return true;
  if (Array.isArray(value)) return value.some(nonemptyClaudeOutput);
  if (!value || typeof value !== "object") return false;
  if (Object.hasOwn(value, "text")) {
    return nonemptyClaudeOutput(value.text);
  }
  if (Object.hasOwn(value, "content")) {
    return nonemptyClaudeOutput(value.content);
  }
  return false;
}

function normalizedPath(value) {
  return typeof value === "string" ? value.replaceAll("\\", "/") : "";
}

function escapedRegex(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function isInstalledSkillPath(value, skill) {
  const path = normalizedPath(value);
  const skillName = escapedRegex(skill);
  const installedPath = new RegExp(
    `^(?:[A-Za-z]:)?/(?:[^/\\s'\"]+/)*(?:plugins/cache|plugin-cache(?:/cache)?)/(?:[^/\\s'\"]+/)*development-system/[^/\\s'\"]+/skills/${skillName}/SKILL\\.md$`,
  );
  return installedPath.test(path);
}

function claudeSkillEvidence(toolCalls, skill) {
  return toolCalls.some((call) => {
    if (!successfulClaudeCall(call)) return false;
    const name = String(call.name || "").toLowerCase();
    return name === "skill" && exactPublicName(call.input?.skill, skill);
  });
}

function claudeAgentEvidence(toolCalls, agent) {
  return toolCalls.some((call) => {
    if (!successfulClaudeCall(call)) return false;
    const name = String(call.name || "").toLowerCase();
    return (
      (name === "agent" || name === "task") &&
      call.input?.run_in_background === false &&
      exactPublicName(call.input?.subagent_type, agent)
    );
  });
}

function assertClaudeDispatch(context, expected) {
  const toolCalls =
    context?.providerResponse?.metadata?.toolCalls ??
    context?.metadata?.toolCalls;
  if (!Array.isArray(toolCalls)) {
    return result(false, "Claude provider response has no tool-call evidence");
  }

  const failures = [];
  const hasExpectedAgent = claudeAgentEvidence(toolCalls, expected.agent);
  if (!claudeSkillEvidence(toolCalls, expected.skill)) {
    failures.push(`no successful installed ${expected.skill} Skill call`);
  }
  if (!hasExpectedAgent) {
    failures.push(
      `no successful foreground ${expected.agent} Agent or Task call`,
    );
  }

  return failures.length > 0
    ? result(false, failures.join("; "))
    : result(
        true,
        `Claude provider evidence confirms installed ${expected.skill} access and foreground ${expected.agent} dispatch`,
      );
}

function parseCodexItems(context) {
  const raw = context?.providerResponse?.raw;
  let trace;
  try {
    trace = typeof raw === "string" ? JSON.parse(raw) : raw;
  } catch {
    return undefined;
  }
  return Array.isArray(trace?.items) ? trace.items : undefined;
}

function successfulCodexItem(item) {
  if (!item || typeof item !== "object" || item.error) return false;
  if (typeof item.status !== "string") return true;
  return ["completed", "success", "succeeded"].includes(
    item.status.toLowerCase(),
  );
}

function successfulCollaborationItem(item) {
  return (
    successfulCodexItem(item) &&
    typeof item?.status === "string" &&
    ["completed", "success", "succeeded"].includes(item.status.toLowerCase())
  );
}

function shellWords(segment) {
  const words = [];
  let word = "";
  let quote = "";

  for (let index = 0; index < segment.length; index += 1) {
    const character = segment[index];

    if (quote) {
      if (character === quote) {
        quote = "";
      } else if (
        character === "\\" &&
        quote === '"' &&
        index + 1 < segment.length
      ) {
        index += 1;
        word += segment[index];
      } else {
        word += character;
      }
      continue;
    }

    if (character === "'" || character === '"') {
      quote = character;
      continue;
    }
    if (character === "\\" && index + 1 < segment.length) {
      index += 1;
      word += segment[index];
      continue;
    }
    if (character === "#" && word.length === 0) break;
    if (/\s/.test(character)) {
      if (word) {
        words.push(word);
        word = "";
      }
      continue;
    }
    word += character;
  }

  if (word) words.push(word);
  return words;
}

function shellSegments(command) {
  const segments = [];
  let segment = "";
  let quote = "";

  for (let index = 0; index < command.length; index += 1) {
    const character = command[index];
    if (quote) {
      segment += character;
      if (character === quote) quote = "";
      else if (
        character === "\\" &&
        quote === '"' &&
        index + 1 < command.length
      ) {
        index += 1;
        segment += command[index];
      }
      continue;
    }
    if (character === "'" || character === '"') {
      quote = character;
      segment += character;
      continue;
    }
    if (character === "\\" && index + 1 < command.length) {
      segment += character + command[++index];
      continue;
    }
    if (character === ";" || character === "|" || character === "\n") {
      if (segment.trim()) segments.push(segment);
      segment = "";
      if (command[index + 1] === character) index += 1;
      continue;
    }
    if (character === "&" && command[index + 1] === "&") {
      if (segment.trim()) segments.push(segment);
      segment = "";
      index += 1;
      continue;
    }
    segment += character;
  }
  if (segment.trim()) segments.push(segment);
  return segments;
}

function basename(command) {
  return normalizedPath(command).split("/").pop().toLowerCase();
}

function readerOperandWords(segment, depth = 0) {
  const words = shellWords(segment);
  let commandIndex = 0;

  while (/^[A-Za-z_][A-Za-z0-9_]*=/.test(words[commandIndex] || "")) {
    commandIndex += 1;
  }
  if (["command", "exec"].includes(basename(words[commandIndex] || ""))) {
    commandIndex += 1;
  }

  const command = basename(words[commandIndex] || "");
  const shellOption = words[commandIndex + 1] || "";
  const shellBody = words[commandIndex + 2];
  if (
    depth < 2 &&
    ["bash", "dash", "ksh", "sh", "zsh"].includes(command) &&
    /^-[A-Za-z]*c[A-Za-z]*$/.test(shellOption) &&
    typeof shellBody === "string"
  ) {
    return readerOperandWords(shellBody, depth + 1);
  }

  const readers = new Set([
    "cat",
    "sed",
    "head",
    "tail",
    "less",
    "more",
    "bat",
  ]);
  if (!readers.has(command)) return [];
  const operands = words.slice(commandIndex + 1);
  const nonReadingModes = new Set([
    "--dry-run",
    "--help",
    "--usage",
    "--version",
    "-V",
    "-h",
  ]);
  if (operands.some((word) => nonReadingModes.has(word))) return [];
  return operands;
}

function codexSkillEvidence(items, skill) {
  return items.some(
    (item) =>
      item?.type === "command_execution" &&
      item.status === "completed" &&
      (item.exit_code === undefined || item.exit_code === 0) &&
      typeof item.command === "string" &&
      !item.command.includes("<<") &&
      shellSegments(item.command).some((segment) =>
        readerOperandWords(segment).some((word) =>
          isInstalledSkillPath(word, skill),
        ),
      ),
  );
}

function collaborationToolName(item) {
  if (
    item?.type !== "collaboration_tool_call" &&
    item?.type !== "collab_tool_call"
  ) {
    return "";
  }
  return String(
    item.tool ?? item.name ?? item.function?.name ?? "",
  ).toLowerCase();
}

function matchesCollaborationTool(item, expected) {
  const tool = collaborationToolName(item);
  return (
    tool === expected ||
    tool.endsWith(`.${expected}`) ||
    tool.endsWith(`:${expected}`) ||
    tool.endsWith(`/${expected}`) ||
    tool.endsWith(`__${expected}`)
  );
}

function isCodexSpawn(item) {
  return matchesCollaborationTool(item, "spawn_agent");
}

function codexSpawnContract(item, expected) {
  const input = item?.arguments ?? item?.input ?? {};
  return (
    input.task_name === expected.codex?.taskName &&
    input.model === expected.codex?.model &&
    input.reasoning_effort === expected.codex?.reasoningEffort &&
    input.fork_turns === expected.codex?.forkTurns &&
    nonemptyString(input.message)
  );
}

function isCodexWait(item) {
  return (
    matchesCollaborationTool(item, "agent_wait") ||
    matchesCollaborationTool(item, "wait_agent") ||
    matchesCollaborationTool(item, "wait")
  );
}

function nonemptyString(value) {
  return typeof value === "string" && value.trim().length > 0;
}

function stringValues(value) {
  if (nonemptyString(value)) return [value.trim()];
  if (Array.isArray(value)) return value.filter(nonemptyString).map(String);
  return [];
}

function receiverThreadIds(item) {
  const input = item?.arguments ?? item?.input ?? {};
  const output = item?.result ?? item?.output ?? {};
  return [
    ...stringValues(item?.receiver_thread_id),
    ...stringValues(item?.receiver_thread_ids),
    ...stringValues(item?.thread_id),
    ...stringValues(input?.receiver_thread_id),
    ...stringValues(input?.receiver_thread_ids),
    ...stringValues(input?.thread_id),
    ...stringValues(output?.receiver_thread_id),
    ...stringValues(output?.receiver_thread_ids),
    ...stringValues(output?.thread_id),
  ];
}

function completedSubstantiveThreadIds(item) {
  const output = item?.result ?? item?.output ?? {};
  const agentStates = item?.agents_states ?? output?.agents_states;
  if (!agentStates || typeof agentStates !== "object") return [];
  return Object.entries(agentStates)
    .filter(([, state]) => {
      if (!state || typeof state !== "object") return false;
      const status = String(state.status ?? "").toLowerCase();
      const substantive =
        state.result ?? state.output ?? state.message ?? state.final_output;
      return (
        ["completed", "success", "succeeded"].includes(status) &&
        nonemptyString(substantive)
      );
    })
    .map(([thread]) => thread);
}

function assertCodexDispatch(output, context, expected) {
  const items = parseCodexItems(context);
  if (!items) {
    return result(false, "Codex provider response has no structured raw items");
  }

  const failures = [];
  if (!codexSkillEvidence(items, expected.skill)) {
    failures.push(
      `no successful installed ${expected.skill} SKILL.md read command`,
    );
  }

  const spawnIndex = items.findIndex(
    (item) =>
      isCodexSpawn(item) &&
      successfulCollaborationItem(item) &&
      codexSpawnContract(item, expected) &&
      receiverThreadIds(item).length > 0,
  );
  if (spawnIndex < 0) {
    failures.push(
      "no successful collaboration spawn_agent call with the exact task/model/effort/fork contract and receiver/thread evidence",
    );
  } else {
    const spawnedThreads = new Set(receiverThreadIds(items[spawnIndex]));
    const completed = items.some((item, index) => {
      if (index <= spawnIndex || !isCodexWait(item)) return false;
      if (!successfulCollaborationItem(item)) return false;
      return completedSubstantiveThreadIds(item).some((thread) =>
        spawnedThreads.has(thread),
      );
    });
    if (!completed) {
      failures.push(
        "no successful nonempty wait/completion evidence after the collaboration spawn",
      );
    }
  }

  return failures.length > 0
    ? result(false, failures.join("; "))
    : result(
        true,
        `Codex provider evidence confirms installed ${expected.skill} access and completed collaboration dispatch`,
      );
}

function providerId(context) {
  const provider = context?.provider;
  if (typeof provider?.id === "function") return String(provider.id());
  if (typeof provider?.id === "string") return provider.id;
  return "";
}

module.exports = function assertInstalledDispatch(output, context = {}) {
  const testCase = caseById(context?.vars?.case_id);
  const expected = testCase?.dispatchEvidence;
  if (!expected || typeof expected.skill !== "string") {
    return result(false, "Eval case has no valid installed-dispatch contract");
  }

  const id = providerId(context);
  if (id.includes("anthropic:claude-agent-sdk")) {
    const agent = expected.claude?.agent ?? expected.agent;
    return typeof agent === "string"
      ? assertClaudeDispatch(context, { ...expected, agent })
      : result(false, "Eval case has no valid Claude dispatch contract");
  }
  if (id.includes("openai:codex-sdk")) {
    return assertCodexDispatch(output, context, expected);
  }

  if (
    Array.isArray(context?.providerResponse?.metadata?.toolCalls) ||
    Array.isArray(context?.metadata?.toolCalls)
  ) {
    const agent = expected.claude?.agent ?? expected.agent;
    return typeof agent === "string"
      ? assertClaudeDispatch(context, { ...expected, agent })
      : result(false, "Eval case has no valid Claude dispatch contract");
  }
  if (parseCodexItems(context)) {
    return assertCodexDispatch(output, context, expected);
  }
  return result(false, `Unsupported provider for installed dispatch: ${id}`);
};
