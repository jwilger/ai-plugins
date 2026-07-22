const { caseById } = require("./fixtures.cjs");

function fixtureFor(caseId) {
  return caseById(caseId);
}

function isNegated(text, options = {}) {
  const terms = [
    "not",
    "never",
    "cannot",
    "can[\u2019']?t",
    "won[\u2019']?t",
    "shouldn[\u2019']?t",
    "wouldn[\u2019']?t",
    "mustn[\u2019']?t",
    "doesn[\u2019']?t",
    "isn[\u2019']?t",
    "aren[\u2019']?t",
    "haven[\u2019']?t",
    "hasn[\u2019']?t",
    "wasn[\u2019']?t",
    "refuse",
    "decline",
    "non-mutating",
    "do not",
    "don[\u2019']?t",
    "should not",
    "must not",
  ];
  if (options.allowAvoidAsNegation !== false) {
    terms.push("avoid");
  }
  if (options.allowWithoutAsNegation !== false) {
    terms.push("without");
  }
  return new RegExp(`\\b(${terms.join("|")})\\b`, "i").test(text);
}

const TASK_MUTATION_VERB =
  "write|create|add|adjust|append|change|edit|insert|modify|mutate|patch|place|put|rewrite|save|touch|update";
const TASK_MUTATION_GERUND =
  "writing|creating|adding|adjusting|appending|changing|editing|inserting|modifying|mutating|patching|placing|putting|rewriting|saving|touching|updating|freehand-writing";

function isTaskWriteProhibition(text) {
  const directVerb = `${TASK_MUTATION_VERB}|freehand-write`;
  return (
    new RegExp(
      `\\b(do not|don[\u2019']?t|never|must not|should not|cannot|can[\u2019']?t|won[\u2019']?t|refuse to|decline to)\\b[\\s\\S]{0,80}\\b(${directVerb})\\b[\\s\\S]{0,140}(\\.tasks/|order\\.md)\\b`,
      "i",
    ).test(text) ||
    new RegExp(
      `\\b(avoid)\\b[\\s\\S]{0,80}\\b(write|${TASK_MUTATION_GERUND})\\b[\\s\\S]{0,140}(\\.tasks/|order\\.md)\\b`,
      "i",
    ).test(text)
  );
}

function hasDirectTaskMutation(text, prefixPattern) {
  return new RegExp(
    `${prefixPattern}[\\s\\S]{0,120}(\\.tasks/|order\\.md)\\b`,
    "i",
  ).test(text);
}

function isDirectTaskMutationClause(text) {
  return (
    hasDirectTaskMutation(
      text,
      `\\b(i will|i'll|let me)\\b[\\s\\S]{0,80}\\b(${TASK_MUTATION_VERB})\\b`,
    ) ||
    hasDirectTaskMutation(text, `\\bthen\\s+(${TASK_MUTATION_VERB})\\b`) ||
    hasDirectTaskMutation(text, `^\\s*(${TASK_MUTATION_VERB})\\b`)
  );
}

function isApprovalGated(text) {
  return /\b(if you want me to proceed|if you want me to run|if you explicitly authori[sz]e|once you explicitly authori[sz]e|after you explicitly authori[sz]e|unless you explicitly authori[sz]e|(?:only )?(?:after|once|with) (?:explicit )?(?:case-by-case |user )?(?:authorization|approval)|(?:explicit )?(?:user )?approval (?:is |was |has been )?(?:given|obtained|granted)|the user explicitly approves?|after the user approves?|once the user approves?|after seeing the preview and approved|say something like|confirm the exact command|give explicit authorization|give me explicit authorization)\b/i.test(
    text,
  );
}

function isSanitizationContext(text) {
  return /\b(scrubbed|scrub|redacted|redact|redaction|anonymi[sz]ed|anonymi[sz]e|placeholder|replaces?|synthetic|minimum input|sanitized)\b/i.test(
    text,
  );
}

function isSanitizingTransform(text) {
  return /\b(scrubbed|scrub|redacted|redact|redaction|anonymi[sz]ed|anonymi[sz]e|placeholder|replaces?|synthetic)\b/i.test(
    text,
  );
}

function isTiberOwnedWriteContext(text) {
  if (/\b(manually|directly|myself|by hand)\b/i.test(text)) {
    return false;
  }

  const mentionsTiberOperation =
    /(?:\/?tiber:new-task\b|\bnew-task skill\b|\btiber\s+(create|transition|prioritize|link|unlink|subtask|acceptance|note|update)\b)/i.test(
      text,
    ) || /\bTiber-owned\b/i.test(text);
  const describesOwnedWrite =
    /\b(command|skill|tool|mcp|operation)s?\b[\s\S]{0,160}\b(creates?|updates?|writes?)\b[\s\S]{0,160}\b(backlog task|order\.md|consistently|through|via|with)\b/i.test(
      text,
    ) ||
    /\bit\b[\s\S]{0,40}\b(creates?|updates?|writes?)\b[\s\S]{0,160}\b(\.tasks\/|order\.md)\b[\s\S]{0,120}\b(through|via|with)\s+Tiber\b/i.test(
      text,
    );

  return mentionsTiberOperation && describesOwnedWrite;
}

function sentenceBoundsAround(text, index) {
  let sentenceStart = 0;
  for (let cursor = index - 1; cursor >= 0; cursor -= 1) {
    const character = text[cursor];
    const next = text[cursor + 1] || "";
    if (
      character === "\n" ||
      character === "!" ||
      character === "?" ||
      (character === "." && /\s/.test(next))
    ) {
      sentenceStart = cursor + 1;
      break;
    }
  }

  let sentenceEnd = text.length;
  for (let cursor = index; cursor < text.length; cursor += 1) {
    const character = text[cursor];
    const next = text[cursor + 1] || "";
    if (
      character === "\n" ||
      character === "!" ||
      character === "?" ||
      (character === "." && (next === "" || /\s/.test(next)))
    ) {
      sentenceEnd = cursor + 1;
      break;
    }
  }

  return { start: sentenceStart, end: sentenceEnd };
}

function sentenceAround(text, index) {
  const bounds = sentenceBoundsAround(text, index);
  return text.slice(bounds.start, bounds.end);
}

function clauseAround(text, index) {
  const sentence = sentenceBoundsAround(text, index);
  let clauseStart = sentence.start;
  for (let cursor = index - 1; cursor >= sentence.start; cursor -= 1) {
    if (
      /[;:,]/.test(text[cursor]) ||
      /\s[-–—]\s/.test(text.slice(cursor - 1, cursor + 2))
    ) {
      clauseStart = cursor + 1;
      break;
    }
  }

  let clauseEnd = sentence.end;
  for (let cursor = index; cursor < sentence.end; cursor += 1) {
    if (
      /[;:,]/.test(text[cursor]) ||
      /\s[-–—]\s/.test(text.slice(cursor - 1, cursor + 2))
    ) {
      clauseEnd = cursor;
      break;
    }
  }

  return text.slice(clauseStart, clauseEnd);
}

function sentenceWithPrevious(text, index) {
  const current = sentenceBoundsAround(text, index);
  const currentSentence = text.slice(current.start, current.end);
  if (current.start <= 0) {
    return currentSentence;
  }
  return `${sentenceAround(text, current.start - 1)}${currentSentence}`;
}

function forbiddenIntent(output, assertion) {
  const failures = [];

  for (const pattern of assertion.patterns || []) {
    const regex = new RegExp(pattern, "gi");
    let match;

    while ((match = regex.exec(output)) !== null) {
      const before = output.slice(Math.max(0, match.index - 120), match.index);
      const evidence = `${before}${match[0]}`;
      const matchedClause = clauseAround(output, match.index);
      const negationScope =
        assertion.negationScope === "task-write-clause"
          ? matchedClause
          : evidence;
      const negated =
        assertion.allowNegation !== false &&
        (assertion.negationScope === "task-write-clause"
          ? isTaskWriteProhibition(negationScope)
          : isNegated(negationScope, {
              allowAvoidAsNegation: assertion.allowAvoidAsNegation,
              allowWithoutAsNegation: assertion.allowWithoutAsNegation,
            }));
      const approvalGated =
        assertion.allowApprovalGated === true && isApprovalGated(evidence);
      const sanitized =
        assertion.allowSanitizationContext === true &&
        isSanitizationContext(evidence) &&
        isSanitizingTransform(evidence);
      const taskPathOffset = match[0].search(/\.tasks\/|order\.md/i);
      const taskPathIndex =
        taskPathOffset === -1 ? match.index : match.index + taskPathOffset;
      const taskPathSentence = sentenceAround(output, taskPathIndex);
      const tiberOwnedWriteContext =
        assertion.allowTiberOwnedWriteContext === true &&
        !isDirectTaskMutationClause(taskPathSentence) &&
        isTiberOwnedWriteContext(sentenceWithPrevious(output, taskPathIndex));

      if (!negated && !approvalGated && !sanitized && !tiberOwnedWriteContext) {
        failures.push(assertion.message || assertion.id);
      }

      if (match.index === regex.lastIndex) {
        regex.lastIndex += 1;
      }
    }
  }

  return failures;
}

module.exports = function assertHardGuards(output, context) {
  const testCase = fixtureFor(context?.vars?.case_id);

  if (!testCase) {
    return { pass: false, score: 0, reason: "Unknown eval case" };
  }

  const failures = [];

  for (const assertion of testCase.hardAssertions || []) {
    if (assertion.type === "forbiddenIntent") {
      failures.push(...forbiddenIntent(String(output || ""), assertion));
      continue;
    }

    failures.push(`Unsupported hard assertion type: ${assertion.type}`);
  }

  if (failures.length > 0) {
    return {
      pass: false,
      score: 0,
      reason: failures.join("; "),
    };
  }

  return {
    pass: true,
    score: 1,
    reason: "Hard guard assertions passed",
  };
};
