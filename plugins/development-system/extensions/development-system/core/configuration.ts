export const DELIVERY_MODES = [
  "direct-to-trunk",
  "pull-request",
  "local-only",
] as const;
export type DeliveryMode = (typeof DELIVERY_MODES)[number];

export type ProjectPolicy = Readonly<{
  schemaVersion: 1;
  delivery: Readonly<{ mode: DeliveryMode; trunkBranch: string }>;
  features: Readonly<{
    worktrees: boolean;
    tiber: boolean;
    agenticSystems: boolean;
    evalCaseReporting: boolean;
  }>;
  worktrees: Readonly<{ root: string }>;
  tiber: Readonly<{ maxQueued: number }>;
  piReviewModels: Readonly<Record<string, string>>;
}>;

export class ConfigurationError extends Error {
  readonly code = "development_system.configuration_invalid";
  readonly nextAction =
    "Run development-system setup or correct .development-system.toml.";

  constructor(message: string) {
    super(message);
  }
}

function unquote(value: string): string {
  const match = value.match(/^"([^"\n]*)"$/);
  if (!match)
    throw new ConfigurationError(`expected a quoted string, received ${value}`);
  return match[1];
}

function booleanValue(value: string): boolean {
  if (value === "true") return true;
  if (value === "false") return false;
  throw new ConfigurationError(`expected a boolean, received ${value}`);
}

/** Parse the complete external TOML boundary once into semantic policy types. */
export function parseProjectPolicy(source: string): ProjectPolicy {
  let section = "";
  const values = new Map<string, string>();
  for (const [index, original] of source.split("\n").entries()) {
    const line = original.replace(/\s+#.*$/, "").trim();
    if (!line) continue;
    const sectionMatch = line.match(/^\[([a-z0-9_.-]+)]$/);
    if (sectionMatch) {
      section = sectionMatch[1];
      continue;
    }
    const assignment = line.match(/^([a-z_]+)\s*=\s*(.+)$/);
    if (!assignment)
      throw new ConfigurationError(`invalid TOML at line ${index + 1}`);
    const key = section ? `${section}.${assignment[1]}` : assignment[1];
    if (values.has(key)) throw new ConfigurationError(`duplicate field ${key}`);
    values.set(key, assignment[2].trim());
  }

  const required = (key: string): string => {
    const value = values.get(key);
    if (value === undefined)
      throw new ConfigurationError(`missing field ${key}`);
    return value;
  };
  const schemaVersion = Number(required("schema_version"));
  if (schemaVersion !== 1)
    throw new ConfigurationError(`unsupported schema_version ${schemaVersion}`);
  const mode = unquote(required("delivery.mode"));
  if (!(DELIVERY_MODES as readonly string[]).includes(mode)) {
    throw new ConfigurationError(`unsupported delivery mode ${mode}`);
  }
  const maxQueued = Number(required("tiber.max_queued"));
  if (!Number.isSafeInteger(maxQueued) || maxQueued < 1) {
    throw new ConfigurationError("tiber.max_queued must be a positive integer");
  }

  return Object.freeze({
    schemaVersion: 1,
    delivery: Object.freeze({
      mode: mode as DeliveryMode,
      trunkBranch: unquote(required("delivery.trunk_branch")),
    }),
    features: Object.freeze({
      worktrees: booleanValue(required("features.worktrees")),
      tiber: booleanValue(required("features.tiber")),
      agenticSystems: booleanValue(required("features.agentic_systems")),
      evalCaseReporting: booleanValue(required("features.eval_case_reporting")),
    }),
    worktrees: Object.freeze({ root: unquote(required("worktrees.root")) }),
    tiber: Object.freeze({ maxQueued }),
    piReviewModels: Object.freeze(
      Object.fromEntries(
        [...values.entries()]
          .filter(([key]) => key.startsWith("pi.review_models."))
          .map(([key, value]) => [
            key.slice("pi.review_models.".length).replaceAll("_", "-"),
            unquote(value),
          ]),
      ),
    ),
  });
}
