const fs = require("fs");
const os = require("os");
const path = require("path");

function capabilityFileCandidates() {
  const configured = process.env.CODEX_MODEL_CAPABILITIES_FILE;
  if (configured) return [configured];

  const homes = [
    process.env.CODEX_EVAL_AUTH_HOME,
    process.env.CODEX_HOME,
    path.join(os.homedir(), ".codex"),
  ].filter((value, index, values) => value && values.indexOf(value) === index);
  return homes.map((home) => path.join(home, "models_cache.json"));
}

function readCapabilityDocument() {
  const candidates = capabilityFileCandidates();
  const file = candidates.find((candidate) => fs.existsSync(candidate));
  if (!file) {
    throw new Error(
      `no Codex model capability cache found at ${candidates.join(", ")}`,
    );
  }

  let document;
  try {
    document = JSON.parse(fs.readFileSync(file, "utf8"));
  } catch (error) {
    const detail = error instanceof Error ? error.message : String(error);
    throw new Error(`invalid Codex model capability cache ${file}: ${detail}`);
  }
  if (!Array.isArray(document.models)) {
    throw new Error(`Codex model capability cache ${file} has no models array`);
  }
  return { document, file };
}

function supportsEffort(model, reasoningEffort) {
  return model.supported_reasoning_levels?.some(
    (level) => level?.effort === reasoningEffort,
  );
}

function strongestEligibleCodexModel(reasoningEffort) {
  const { document, file } = readCapabilityDocument();
  const eligible = document.models.filter(
    (model) =>
      typeof model?.slug === "string" &&
      model.slug.length > 0 &&
      model.visibility === "list" &&
      model.upgrade == null &&
      Number.isFinite(model.priority) &&
      supportsEffort(model, reasoningEffort),
  );
  if (eligible.length === 0) {
    throw new Error(
      `Codex model capability cache ${file} advertises no eligible visible model supporting ${reasoningEffort} effort`,
    );
  }

  const bestPriority = Math.min(...eligible.map((model) => model.priority));
  const strongest = eligible.filter((model) => model.priority === bestPriority);
  if (strongest.length !== 1) {
    throw new Error(
      `Codex model capability cache ${file} has ambiguous highest-capability priority ${bestPriority}: ${strongest.map((model) => model.slug).join(", ")}`,
    );
  }

  return {
    model: strongest[0].slug,
    priority: bestPriority,
    source: file,
  };
}

module.exports = { strongestEligibleCodexModel };
