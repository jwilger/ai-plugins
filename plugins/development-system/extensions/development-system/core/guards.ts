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
    }>
  | Readonly<{
      kind: "read-only-discovery";
      targetPath: string;
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
    if (";&|`$(){}\n\r".includes(character) && hazard === "none")
      hazard = "control";
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

function boundedGitRead(words: readonly string[]): boolean {
  if (words[0] !== "git") return false;
  const operation = words[1];
  const args = words.slice(2);
  if (operation === "status")
    return args.every(
      (argument) =>
        ["--short", "--branch", "--porcelain", "--porcelain=v1", "-b"].includes(
          argument,
        ) || argument.startsWith("--untracked-files="),
    );
  if (operation === "worktree")
    return (
      args[0] === "list" &&
      args.slice(1).every((argument) => argument === "--porcelain")
    );
  if (operation === "branch")
    return (
      args.length >= 1 &&
      ["--show-current", "--list"].includes(args[0]) &&
      args.slice(1).every((argument) => !argument.startsWith("-"))
    );
  if (operation === "rev-parse")
    return args.every(
      (argument) =>
        [
          "--path-format=absolute",
          "--git-dir",
          "--git-common-dir",
          "--show-toplevel",
          "--show-prefix",
          "--is-inside-work-tree",
          "--abbrev-ref",
          "HEAD",
        ].includes(argument) || /^[A-Za-z0-9][A-Za-z0-9._/-]*$/.test(argument),
    );
  if (operation === "log")
    return args.every(
      (argument) =>
        ["--oneline", "--decorate", "--no-decorate"].includes(argument) ||
        /^--max-count=\d+$/.test(argument) ||
        /^-n\d+$/.test(argument) ||
        /^[A-Za-z0-9][A-Za-z0-9._/-]*$/.test(argument),
    );
  return false;
}

function boundedCdDiscovery(command: string): ShellClassification | null {
  const parts = command.split("&&");
  if (parts.length !== 2) return null;
  const left = shellWords(parts[0]);
  const right = shellWords(parts[1]);
  if (
    !left ||
    !right ||
    left.hazard !== "none" ||
    right.hazard !== "none" ||
    left.words.length !== 2 ||
    left.words[0] !== "cd" ||
    !boundedGitRead(right.words)
  )
    return null;
  return { kind: "read-only-discovery", targetPath: left.words[1] };
}

const gitReadOperations = new Set([
  "blame",
  "cat-file",
  "check-ignore",
  "count-objects",
  "describe",
  "diff",
  "for-each-ref",
  "grep",
  "log",
  "ls-files",
  "ls-tree",
  "name-rev",
  "rev-list",
  "rev-parse",
  "shortlog",
  "show",
  "status",
]);

function gitReadInvocationSafe(
  operation: string,
  args: readonly string[],
): boolean {
  if (!gitReadOperations.has(operation)) return false;
  return !args.some(
    (argument) =>
      argument === "--output" ||
      argument.startsWith("--output=") ||
      argument === "--ext-diff" ||
      argument === "--textconv" ||
      argument === "--filters" ||
      argument === "--open-files-in-pager" ||
      argument.startsWith("--open-files-in-pager="),
  );
}

const gitMutationOperations = new Set([
  "add",
  "am",
  "apply",
  "checkout",
  "cherry-pick",
  "clean",
  "commit",
  "merge",
  "mv",
  "notes",
  "pull",
  "rebase",
  "reset",
  "restore",
  "revert",
  "rm",
  "stash",
  "switch",
  "tag",
  "update-ref",
  "symbolic-ref",
]);
const gitOptionsWithValues = new Set([
  "-C",
  "-c",
  "--git-dir",
  "--work-tree",
  "--namespace",
  "--config-env",
  "--exec-path",
]);
const gitFlagOptions = new Set([
  "--bare",
  "--no-pager",
  "--no-replace-objects",
  "-P",
  "--literal-pathspecs",
  "--no-literal-pathspecs",
  "--glob-pathspecs",
  "--noglob-pathspecs",
  "--icase-pathspecs",
]);

function normalizedShellCommands(command: string): string[][] | null {
  const commands: string[][] = [];
  let words: string[] = [];
  let word = "";
  let quote: "'" | '"' | null = null;
  let escaped = false;
  const flushWord = () => {
    if (!word) return;
    words.push(word);
    word = "";
  };
  const flushCommand = () => {
    flushWord();
    if (words.length > 0) commands.push(words);
    words = [];
  };
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
    if (";&|(){}\n\r`".includes(character)) {
      flushCommand();
      continue;
    }
    if (/\s/.test(character)) {
      flushWord();
      continue;
    }
    word += character;
  }
  if (quote || escaped) return null;
  flushCommand();
  return commands;
}

function unwrapCommand(words: readonly string[]): readonly string[] {
  let index = 0;
  const consumeAssignments = () => {
    while (/^[A-Za-z_][A-Za-z0-9_]*=/.test(words[index] ?? "")) index += 1;
  };
  consumeAssignments();
  for (;;) {
    const wrapper = path.basename(words[index] ?? "");
    if (wrapper === "env") {
      index += 1;
      for (;;) {
        const argument = words[index] ?? "";
        if (/^[A-Za-z_][A-Za-z0-9_]*=/.test(argument)) {
          index += 1;
          continue;
        }
        if (["-C", "--chdir", "-u", "--unset", "--argv0"].includes(argument)) {
          index += 2;
          continue;
        }
        if (argument.startsWith("-")) {
          index += 1;
          continue;
        }
        break;
      }
      consumeAssignments();
      continue;
    }
    if (["command", "exec", "nohup"].includes(wrapper)) {
      index += 1;
      while ((words[index] ?? "").startsWith("-")) index += 1;
      consumeAssignments();
      continue;
    }
    if (wrapper === "sudo") {
      index += 1;
      for (;;) {
        const argument = words[index] ?? "";
        if (
          [
            "-u",
            "--user",
            "-g",
            "--group",
            "-h",
            "--host",
            "-p",
            "--prompt",
            "-C",
            "--close-from",
            "-T",
            "--command-timeout",
            "-R",
            "--chroot",
            "-D",
            "--chdir",
          ].includes(argument)
        ) {
          index += 2;
          continue;
        }
        if (argument.startsWith("-")) {
          index += 1;
          continue;
        }
        break;
      }
      consumeAssignments();
      continue;
    }
    break;
  }
  return words.slice(index);
}

function normalizedGitInvocation(words: readonly string[]): Readonly<{
  operation: string;
  args: readonly string[];
  unsafeGlobalOption: boolean;
}> | null {
  const command = unwrapCommand(words);
  if (path.basename(command[0] ?? "") !== "git") return null;
  let index = 1;
  let unsafeGlobalOption = false;
  while (index < command.length) {
    const option = command[index];
    if (gitOptionsWithValues.has(option)) {
      const value = command[index + 1] ?? "";
      if (
        option === "--config-env" ||
        option === "--exec-path" ||
        (option === "-c" && !value.startsWith("color."))
      )
        unsafeGlobalOption = true;
      index += 2;
      continue;
    }
    if (
      gitFlagOptions.has(option) ||
      /^(?:--git-dir|--work-tree|--namespace|--config-env|--exec-path)=/.test(
        option,
      )
    ) {
      if (/^--(?:config-env|exec-path)=/.test(option))
        unsafeGlobalOption = true;
      index += 1;
      continue;
    }
    break;
  }
  const operation = command[index];
  return operation
    ? { operation, args: command.slice(index + 1), unsafeGlobalOption }
    : null;
}

function normalizedCommandWords(command: string, depth = 0): string[][] {
  const commands = normalizedShellCommands(command) ?? [];
  if (depth >= 3) return commands;
  return commands.flatMap((words) => {
    const unwrapped = unwrapCommand(words);
    const executable = path.basename(unwrapped[0] ?? "");
    const commandIndex = unwrapped.findIndex(
      (word, index) => index > 0 && /^-[^-]*c[^-]*$/.test(word),
    );
    if (
      ["bash", "dash", "ksh", "sh", "zsh"].includes(executable) &&
      commandIndex >= 0 &&
      unwrapped[commandIndex + 1]
    )
      return [
        words,
        ...normalizedCommandWords(unwrapped[commandIndex + 1], depth + 1),
      ];
    return [words];
  });
}

function normalizedGitInvocations(command: string) {
  return normalizedCommandWords(command)
    .map(normalizedGitInvocation)
    .filter(
      (
        invocation,
      ): invocation is Readonly<{
        operation: string;
        args: readonly string[];
        unsafeGlobalOption: boolean;
      }> => invocation !== null,
    );
}

const commandBoundary = "(?:^|[;&|({\\n]\\s*)";
const wrapperPrefix =
  "(?:(?:[A-Za-z_][A-Za-z0-9_]*=[^\\s;&|]+|env(?:\\s+(?:-[^\\s;&|]+|[A-Za-z_][A-Za-z0-9_]*=[^\\s;&|]+))*|command(?:\\s+-[^\\s;&|]+)?)\\s+)*";
const gitOption =
  "(?:(?:-C|-c|--git-dir|--work-tree|--namespace|--config-env|--exec-path)\\s+[^\\s;&|]+|(?:--git-dir|--work-tree|--namespace|--config-env|--exec-path)=[^\\s;&|]+|--bare|--no-pager|--paginate|--literal-pathspecs|--no-literal-pathspecs|--glob-pathspecs|--noglob-pathspecs|--icase-pathspecs)";
const gitPrefix = `${wrapperPrefix}git(?:\\s+${gitOption})*\\s+`;

export function classifyShellDelivery(
  command: string,
): Readonly<{ kind: "delivery" | "destructive-delivery" }> | null {
  const normalizedPushes = normalizedGitInvocations(command).filter(
    (invocation) => invocation.operation === "push",
  );
  if (normalizedPushes.length > 0) {
    const destructive = normalizedPushes.some((push) =>
      push.args.some(
        (argument) =>
          argument === "-f" ||
          argument === "--force" ||
          argument.startsWith("--force-with-lease") ||
          argument.startsWith("+"),
      ),
    );
    return { kind: destructive ? "destructive-delivery" : "delivery" };
  }
  const pushes = command.match(
    new RegExp(`${commandBoundary}${gitPrefix}push(?:\\s+[^;&|\\n]*)?`, "g"),
  );
  if (!pushes || pushes.length === 0) return null;
  const destructive = pushes.some((push) =>
    /(?:^|\s)(?:-f|--force)(?:\s|$)|(?:^|\s)--force-with-lease(?:=\S+)?(?:\s|$)|(?:^|\s)\+[^\s]+/.test(
      push,
    ),
  );
  return { kind: destructive ? "destructive-delivery" : "delivery" };
}

function containsObviousMutation(command: string): boolean {
  const filesystemMutationNames = new Set([
    "chmod",
    "chown",
    "cp",
    "install",
    "ln",
    "mkdir",
    "mkfifo",
    "mknod",
    "mv",
    "rm",
    "rmdir",
    "shred",
    "tee",
    "touch",
    "truncate",
    "unlink",
  ]);
  const normalizedFilesystemMutation = normalizedCommandWords(command).some(
    (words) => {
      if (
        path.basename(words[0] ?? "") === "env" &&
        words.some(
          (argument) => argument === "-S" || argument === "--split-string",
        )
      )
        return true;
      const unwrapped = unwrapCommand(words);
      const executable = path.basename(unwrapped[0] ?? "");
      const args = unwrapped.slice(1);
      return (
        filesystemMutationNames.has(executable) ||
        (executable === "xargs" &&
          args.some(
            (argument) =>
              filesystemMutationNames.has(path.basename(argument)) ||
              path.basename(argument) === "git",
          )) ||
        executable === "rsync" ||
        (executable === "sed" &&
          args.some(
            (argument) =>
              argument === "-i" ||
              argument.startsWith("-i") ||
              argument === "--in-place" ||
              argument.startsWith("--in-place="),
          )) ||
        (executable === "find" &&
          (args.includes("-delete") ||
            args.some((argument, index) =>
              ["-exec", "-execdir", "-ok", "-okdir"].includes(
                args[index - 1] ?? "",
              )
                ? filesystemMutationNames.has(path.basename(argument))
                : false,
            )))
      );
    },
  );
  if (normalizedFilesystemMutation) return true;
  const normalizedMutation = normalizedGitInvocations(command).some(
    (invocation) =>
      invocation.unsafeGlobalOption ||
      gitMutationOperations.has(invocation.operation) ||
      (invocation.operation === "branch" &&
        !["--show-current", "--list"].includes(invocation.args[0] ?? "")) ||
      (invocation.operation === "worktree" && invocation.args[0] !== "list") ||
      (invocation.operation === "config" &&
        !invocation.args.some((argument) =>
          ["--get", "--get-all", "--get-regexp", "--list", "-l"].includes(
            argument,
          ),
        )) ||
      (invocation.operation !== "push" &&
        invocation.operation !== "branch" &&
        invocation.operation !== "worktree" &&
        invocation.operation !== "config" &&
        !gitReadInvocationSafe(invocation.operation, invocation.args)),
  );
  if (normalizedMutation) return true;
  const gitMutation = new RegExp(
    `${commandBoundary}${gitPrefix}(?:add|am|apply|checkout|cherry-pick|clean|commit|merge|mv|notes|rebase|reset|restore|revert|rm|stash|switch|tag|update-ref|symbolic-ref)(?:\\s|$)`,
  );
  const gitBranchMutation = new RegExp(
    `${commandBoundary}${gitPrefix}branch\\s+(?!--show-current(?:\\s|$)|--list(?:\\s|$))`,
  );
  const gitWorktreeMutation = new RegExp(
    `${commandBoundary}${gitPrefix}worktree\\s+(?!list(?:\\s|$))`,
  );
  const filesystemMutation = new RegExp(
    `${commandBoundary}(?:touch|rm|mv|cp|mkdir|rmdir|chmod|chown|install|tee)(?:\\s|$)`,
  );
  return (
    gitMutation.test(command) ||
    gitBranchMutation.test(command) ||
    gitWorktreeMutation.test(command) ||
    filesystemMutation.test(command)
  );
}

export function classifyShellCommand(command: string): ShellClassification {
  const cdDiscovery = boundedCdDiscovery(command);
  if (cdDiscovery) return cdDiscovery;
  const parsed = shellWords(command);
  if (!parsed || parsed.words.length === 0) return { kind: "read-only" };
  if (parsed.hazard === "redirection") return { kind: "mutation" };
  const words = parsed.words;
  const [program, operation] = words;
  if (
    (program === "scripts/agent-checkout-guard.sh" ||
      program === "./scripts/agent-checkout-guard.sh") &&
    words.length === 1
  )
    return { kind: "read-only" };
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
  const delivery = classifyShellDelivery(command);
  if (containsObviousMutation(command)) return { kind: "mutation" };
  if (delivery) return delivery;
  if (
    program === "git" &&
    operation === "-C" &&
    words.length >= 4 &&
    boundedGitRead(["git", ...words.slice(3)])
  )
    return { kind: "read-only-discovery", targetPath: words[2] };
  if (boundedGitRead(words)) return { kind: "read-only" };
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
  // Coordination checkouts are useful for exploration, test runs, builds, and
  // repository inspection. Unknown commands are therefore allowed; direct
  // file tools, obvious filesystem mutation, and Git history/index/worktree
  // mutation remain separately guarded.
  return { kind: "read-only" };
}

export function worktreeTargetAllowed(
  input: Readonly<{
    rawPath: string;
    cwd: string;
    primary: string;
    configuredRoot?: string;
  }>,
): boolean {
  const canonicalPrimary = fs.realpathSync(input.primary);
  const configuredRoot = input.configuredRoot ?? ".worktrees";
  if (path.isAbsolute(configuredRoot)) return false;
  const canonicalRoot = canonicalProspectivePath(
    path.resolve(canonicalPrimary, configuredRoot),
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
