import fs from "node:fs";
import path from "node:path";
import type { DeliveryMode } from "./configuration.ts";

export type PathClassification = Readonly<{
  kind: "inside" | "outside" | "protected-metadata" | "protected-secret";
  canonicalPath: string;
}>;

function canonicalProspectivePath(candidate: string): string {
  const missing: string[] = [];
  let ancestor = candidate;
  while (!fs.existsSync(ancestor)) {
    const parent = path.dirname(ancestor);
    if (parent === ancestor) break;
    missing.unshift(path.basename(ancestor));
    ancestor = parent;
  }
  return path.join(fs.realpathSync(ancestor), ...missing);
}

function isWithin(boundary: string, candidate: string): boolean {
  const relative = path.relative(boundary, candidate);
  return (
    relative === "" ||
    (!relative.startsWith(`..${path.sep}`) &&
      relative !== ".." &&
      !path.isAbsolute(relative))
  );
}

function secretName(relative: string): boolean {
  const basename = path.basename(relative).toLowerCase();
  return (
    basename === ".env" ||
    basename.startsWith(".env.") ||
    basename === "credentials" ||
    basename === "credentials.json" ||
    basename === "auth.json" ||
    basename === "id_rsa" ||
    basename === "id_ed25519"
  );
}

export function classifyPath(
  input: Readonly<{ rawPath: string; cwd: string; boundary: string }>,
): PathClassification {
  const raw = input.rawPath.startsWith("@")
    ? input.rawPath.slice(1)
    : input.rawPath;
  const canonicalBoundary = fs.realpathSync(input.boundary);
  const canonicalPath = canonicalProspectivePath(path.resolve(input.cwd, raw));
  if (!isWithin(canonicalBoundary, canonicalPath))
    return { kind: "outside", canonicalPath };
  const relative = path.relative(canonicalBoundary, canonicalPath);
  if (
    relative === ".git" ||
    relative.startsWith(`.git${path.sep}`) ||
    relative === ".development-system.toml"
  ) {
    return { kind: "protected-metadata", canonicalPath };
  }
  if (secretName(relative)) return { kind: "protected-secret", canonicalPath };
  return { kind: "inside", canonicalPath };
}

export type ShellClassification =
  | Readonly<{
      kind:
        | "read-only"
        | "delivery"
        | "destructive-delivery"
        | "mutation"
        | "ambiguous";
    }>
  | Readonly<{
      kind: "worktree-creation";
      targetPath: string;
      branch: string;
    }>;

function shellWords(command: string): Readonly<{
  words: string[];
  hazard: "none" | "redirection" | "control";
}> | null {
  const words: string[] = [];
  let word = "";
  let quote: "'" | '"' | null = null;
  let escaped = false;
  let hazard: "none" | "redirection" | "control" = "none";
  for (const character of command) {
    if (escaped) {
      word += character;
      escaped = false;
      continue;
    }
    if (character === "\\" && quote !== "'") {
      escaped = true;
      continue;
    }
    if (quote) {
      if (character === quote) quote = null;
      else word += character;
      continue;
    }
    if (character === "'" || character === '"') {
      quote = character;
      continue;
    }
    if ("><".includes(character)) {
      hazard = "redirection";
      continue;
    }
    if (";&|`$(){}\n\r".includes(character)) hazard = "control";
    if (/\s/.test(character)) {
      if (word) {
        words.push(word);
        word = "";
      }
      continue;
    }
    word += character;
  }
  if (quote || escaped) return null;
  if (word) words.push(word);
  return { words, hazard };
}

export function classifyShellCommand(command: string): ShellClassification {
  const parsed = shellWords(command);
  if (!parsed || parsed.hazard === "control" || parsed.words.length === 0)
    return { kind: "ambiguous" };
  if (parsed.hazard === "redirection") return { kind: "mutation" };
  const words = parsed.words;
  const [program, operation] = words;
  if (
    program === "git" &&
    operation === "worktree" &&
    words[2] === "add" &&
    words.length === 6
  ) {
    if (words[4] === "-b")
      return {
        kind: "worktree-creation",
        targetPath: words[3],
        branch: words[5],
      };
    if (words[3] === "-b")
      return {
        kind: "worktree-creation",
        targetPath: words[5],
        branch: words[4],
      };
  }
  if (program === "git" && operation === "push") {
    const destructive = words.some(
      (word) =>
        word === "-f" ||
        word === "--force" ||
        word.startsWith("--force-with-lease") ||
        word.startsWith("+"),
    );
    return { kind: destructive ? "destructive-delivery" : "delivery" };
  }
  if (
    program === "git" &&
    ["status", "log", "rev-parse", "branch"].includes(operation)
  )
    return { kind: "read-only" };
  if (["pwd", "ls"].includes(program)) return { kind: "read-only" };
  if (
    [
      "touch",
      "rm",
      "mv",
      "cp",
      "mkdir",
      "rmdir",
      "chmod",
      "chown",
      "install",
      "tee",
      "printf",
      "echo",
    ].includes(program)
  )
    return { kind: "mutation" };
  return { kind: "ambiguous" };
}

export function worktreeTargetAllowed(
  input: Readonly<{ rawPath: string; cwd: string; primary: string }>,
): boolean {
  const canonicalPrimary = fs.realpathSync(input.primary);
  const canonicalRoot = canonicalProspectivePath(
    path.resolve(canonicalPrimary, ".worktrees"),
  );
  const canonicalTarget = canonicalProspectivePath(
    path.resolve(input.cwd, input.rawPath),
  );
  return (
    canonicalRoot !== canonicalPrimary &&
    isWithin(canonicalPrimary, canonicalRoot) &&
    canonicalTarget !== canonicalRoot &&
    isWithin(canonicalRoot, canonicalTarget)
  );
}

export type DeliveryRejection = Readonly<{
  code: string;
  boundary: "delivery";
  missing: string;
  nextAction: string;
}>;

export function deliveryDecision(
  input: Readonly<{
    mode: DeliveryMode | null;
    branch: string;
    trunk: string;
    destructive: boolean;
  }>,
): DeliveryRejection | null {
  if (input.mode === null)
    return {
      code: "development_system.delivery_mode_missing",
      boundary: "delivery",
      missing: "configured delivery mode",
      nextAction: "Run setup before publishing.",
    };
  if (input.mode === "local-only")
    return {
      code: "development_system.local_only_publication_blocked",
      boundary: "delivery",
      missing: "current explicit publication authorization",
      nextAction: "Keep work local or configure an authorized delivery mode.",
    };
  if (input.mode === "pull-request" && input.branch === input.trunk)
    return {
      code: "development_system.pull_request_branch_required",
      boundary: "delivery",
      missing: "non-trunk pull-request branch",
      nextAction: "Create and push a feature branch.",
    };
  if (input.mode === "direct-to-trunk" && input.branch !== input.trunk)
    return {
      code: "development_system.direct_trunk_branch_required",
      boundary: "delivery",
      missing: `configured trunk branch ${input.trunk}`,
      nextAction: "Use the configured direct-to-trunk checkout.",
    };
  if (input.destructive)
    return {
      code: "development_system.destructive_approval_required",
      boundary: "delivery",
      missing: "case-specific trusted TUI approval",
      nextAction: "Confirm this exact operation in the local Pi TUI.",
    };
  return null;
}

export function guardMessage(
  rejection: Readonly<{
    code: string;
    boundary: string;
    missing: string;
    nextAction: string;
  }>,
): string {
  return `${rejection.code}; boundary=${rejection.boundary}; missing=${rejection.missing}; next=${rejection.nextAction}`;
}
