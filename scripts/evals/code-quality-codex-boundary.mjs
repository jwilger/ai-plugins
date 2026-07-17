#!/usr/bin/env node

import { spawn, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  accessSync,
  chmodSync,
  copyFileSync,
  cpSync,
  constants,
  lstatSync,
  mkdtempSync,
  mkdirSync,
  opendirSync,
  readFileSync,
  readdirSync,
  realpathSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { fileURLToPath } from "node:url";
import { basename, dirname, isAbsolute, join, resolve, sep } from "node:path";
import process from "node:process";
import {
  canonicalJson,
  parseExecutionSurface,
} from "./code-quality-runtime-contract.mjs";

let activeChildProcess;
let activeRuntimeSupport;
let handlingInternalFailure = false;
let cancellationSignal;
let forceKillTimer;

function signalActiveChild(signal) {
  if (!activeChildProcess?.pid) return;
  try {
    process.kill(-activeChildProcess.pid, signal);
  } catch {
    try {
      activeChildProcess.kill(signal);
    } catch {
      // The child already exited.
    }
  }
}

for (const signal of ["SIGHUP", "SIGINT", "SIGTERM"]) {
  process.on(signal, () => {
    if (cancellationSignal) {
      signalActiveChild("SIGKILL");
      return;
    }
    cancellationSignal = signal;
    signalActiveChild(signal);
    if (activeChildProcess?.pid) {
      forceKillTimer = setTimeout(() => signalActiveChild("SIGKILL"), 5_000);
      forceKillTimer.unref();
    }
  });
}

function cleanupRuntimeSupport() {
  if (!activeRuntimeSupport) return true;
  try {
    rmSync(activeRuntimeSupport, { force: true, recursive: true });
    activeRuntimeSupport = undefined;
    return true;
  } catch {
    return false;
  }
}

function handleInternalFailure() {
  if (handlingInternalFailure) process.exit(70);
  handlingInternalFailure = true;
  if (activeChildProcess?.pid) {
    signalActiveChild("SIGKILL");
  }
  cleanupRuntimeSupport();
  console.error("CODE_QUALITY_BOUNDARY_ERROR:runtime:internal-error");
  process.exit(70);
}

process.on("uncaughtException", handleInternalFailure);
process.on("unhandledRejection", handleInternalFailure);

function fail(category, detail, exitCode) {
  if (!cleanupRuntimeSupport()) {
    console.error("CODE_QUALITY_BOUNDARY_ERROR:runtime:cleanup-failed");
    process.exit(70);
  }
  console.error(`CODE_QUALITY_BOUNDARY_ERROR:${category}:${detail}`);
  process.exit(exitCode);
}

function canonicalDirectory(candidate, detail) {
  try {
    const metadata = lstatSync(candidate);
    if (
      !isAbsolute(candidate) ||
      resolve(candidate) !== candidate ||
      realpathSync(candidate) !== candidate ||
      !metadata.isDirectory()
    ) {
      fail("configuration", detail, 64);
    }
    return statSync(candidate);
  } catch {
    fail("configuration", detail, 64);
  }
}

function canonicalRegularFile(candidate, detail) {
  try {
    const metadata = lstatSync(candidate);
    if (realpathSync(candidate) !== candidate || !metadata.isFile()) {
      fail("configuration", detail, 64);
    }
    accessSync(candidate, constants.R_OK);
    return candidate;
  } catch {
    fail("configuration", detail, 64);
  }
}

function optionalCanonicalDirectory(candidate, detail) {
  let metadata;
  try {
    metadata = lstatSync(candidate);
  } catch (error) {
    if (error?.code === "ENOENT") return null;
    fail("configuration", detail, 64);
  }
  if (!metadata.isDirectory()) {
    fail("configuration", detail, 64);
  }
  try {
    if (realpathSync(candidate) !== candidate)
      fail("configuration", detail, 64);
    accessSync(candidate, constants.R_OK | constants.X_OK);
    return candidate;
  } catch {
    fail("configuration", detail, 64);
  }
}

function pathsOverlap(first, second) {
  return (
    first === second ||
    first === sep ||
    second === sep ||
    first.startsWith(`${second}${sep}`) ||
    second.startsWith(`${first}${sep}`)
  );
}

function validatedToolDirectories(toolPath) {
  const directories = toolPath.split(":");
  if (directories.length === 0 || directories.some((entry) => !entry)) {
    fail("configuration", "unsafe-tool-path", 64);
  }
  for (const directory of directories) {
    try {
      if (
        !directory.startsWith("/nix/store/") ||
        !isAbsolute(directory) ||
        resolve(directory) !== directory ||
        realpathSync(directory) !== directory ||
        !lstatSync(directory).isDirectory()
      ) {
        fail("configuration", "unsafe-tool-path", 64);
      }
    } catch {
      fail("configuration", "unsafe-tool-path", 64);
    }
  }
  return directories;
}

function nixStoreRootForToolDirectory(directory) {
  const root = dirname(directory);
  if (
    basename(directory) !== "bin" ||
    dirname(root) !== "/nix/store" ||
    !/^[0-9abcdfghijklmnpqrsvwxyz]{32}-.+$/u.test(basename(root))
  ) {
    fail("configuration", "unsafe-tool-path", 64);
  }
  return root;
}

function validatedNixStoreClosure(candidate, expectedSha256, toolDirectories) {
  if (!/^[0-9a-f]{64}$/u.test(expectedSha256 ?? "")) {
    fail("configuration", "invalid-nix-store-closure-sha256", 64);
  }
  let bytes;
  let metadata;
  try {
    metadata = lstatSync(candidate);
    if (
      !isAbsolute(candidate) ||
      resolve(candidate) !== candidate ||
      realpathSync(candidate) !== candidate ||
      !metadata.isFile() ||
      metadata.nlink !== 1 ||
      metadata.size < 45 ||
      metadata.size > 1_048_576 ||
      (metadata.mode & 0o022) !== 0
    ) {
      fail("configuration", "nix-store-closure-unsafe", 64);
    }
    bytes = readFileSync(candidate);
  } catch {
    fail("configuration", "nix-store-closure-unsafe", 64);
  }
  const actualSha256 = createHash("sha256").update(bytes).digest("hex");
  if (actualSha256 !== expectedSha256) {
    fail("integrity", "nix-store-closure-sha256-mismatch", 65);
  }
  const contents = bytes.toString("utf8");
  if (!contents.endsWith("\n") || contents.includes("\0")) {
    fail("configuration", "nix-store-closure-invalid", 64);
  }
  const entries = contents.slice(0, -1).split("\n");
  if (
    entries.length === 0 ||
    entries.length > 8_192 ||
    entries.some((entry, index) => index > 0 && entry <= entries[index - 1])
  ) {
    fail("configuration", "nix-store-closure-invalid", 64);
  }
  for (const entry of entries) {
    try {
      const entryMetadata = lstatSync(entry);
      if (
        dirname(entry) !== "/nix/store" ||
        !/^[0-9abcdfghijklmnpqrsvwxyz]{32}-[^/\u0000-\u001f\u007f]+$/u.test(
          basename(entry),
        ) ||
        realpathSync(entry) !== entry ||
        !entryMetadata.isDirectory() ||
        (entryMetadata.mode & 0o022) !== 0
      ) {
        fail("configuration", "nix-store-closure-invalid", 64);
      }
    } catch {
      fail("configuration", "nix-store-closure-invalid", 64);
    }
  }
  const entrySet = new Set(entries);
  for (const directory of toolDirectories) {
    if (!entrySet.has(nixStoreRootForToolDirectory(directory))) {
      fail("configuration", "nix-store-closure-incomplete", 64);
    }
  }
  return entries;
}

function executableFromSafePath(directories, executable) {
  for (const directory of directories) {
    try {
      const candidate = join(directory, executable);
      accessSync(candidate, constants.R_OK | constants.X_OK);
      return candidate;
    } catch {
      continue;
    }
  }
  fail("configuration", `missing-safe-tool-${executable}`, 64);
}

function requiredHostFile(candidate, detail) {
  try {
    const canonical = realpathSync(candidate);
    if (!lstatSync(canonical).isFile()) {
      fail("configuration", detail, 64);
    }
    accessSync(canonical, constants.R_OK);
    return canonical;
  } catch {
    fail("configuration", detail, 64);
  }
}

function pinnedRuntimeExecutable(candidate, expectedSha256, expectedName) {
  if (!/^[0-9a-f]{64}$/.test(expectedSha256 ?? "")) {
    fail("configuration", `invalid-${expectedName}-sha256`, 64);
  }
  let canonical;
  try {
    if (
      !isAbsolute(candidate) ||
      resolve(candidate) !== candidate ||
      basename(candidate) !== expectedName ||
      realpathSync(dirname(candidate)) !== dirname(candidate)
    ) {
      fail("configuration", `${expectedName}-binary-not-canonical`, 64);
    }
    canonical = realpathSync(candidate);
    if (!lstatSync(canonical).isFile()) {
      fail("configuration", `${expectedName}-binary-not-canonical`, 64);
    }
    accessSync(canonical, constants.R_OK | constants.X_OK);
  } catch {
    fail("configuration", `${expectedName}-binary-not-canonical`, 64);
  }
  const actualSha256 = createHash("sha256")
    .update(readFileSync(canonical))
    .digest("hex");
  if (actualSha256 !== expectedSha256) {
    fail("integrity", `${expectedName}-sha256-mismatch`, 65);
  }
  return canonical;
}

function assertPinnedFile(candidate, expectedSha256, label) {
  if (!/^[0-9a-f]{64}$/.test(expectedSha256 ?? "")) {
    fail("configuration", `invalid-${label}-sha256`, 64);
  }
  let canonical;
  try {
    canonical = realpathSync(candidate);
    if (canonical !== candidate || !lstatSync(canonical).isFile()) {
      fail("configuration", `${label}-not-canonical`, 64);
    }
    accessSync(canonical, constants.R_OK);
  } catch {
    fail("configuration", `${label}-not-canonical`, 64);
  }
  const actualSha256 = createHash("sha256")
    .update(readFileSync(canonical))
    .digest("hex");
  if (actualSha256 !== expectedSha256) {
    fail("integrity", `${label}-sha256-mismatch`, 65);
  }
  return canonical;
}

function snapshotPinnedFile(source, destination, expectedSha256, label) {
  copyFileSync(source, destination, constants.COPYFILE_FICLONE);
  chmodSync(destination, 0o500);
  const snapshotSha256 = createHash("sha256")
    .update(readFileSync(destination))
    .digest("hex");
  if (snapshotSha256 !== expectedSha256) {
    fail("integrity", `${label}-snapshot-sha256-mismatch`, 65);
  }
  return destination;
}

function boundedPositiveInteger(raw, label, maximum) {
  if (!/^[1-9][0-9]*$/.test(raw ?? "")) {
    fail("configuration", `invalid-${label}`, 64);
  }
  const value = Number(raw);
  if (!Number.isSafeInteger(value) || value > maximum) {
    fail("configuration", `invalid-${label}`, 64);
  }
  return value;
}

function scanWritableRoots(roots, maxBytes, maxEntries, tolerateRaces = false) {
  const pendingDirectories = roots.map(
    ({ label, path, rejectHardlinks = false, rejectSymlinks }) => ({
      label,
      path,
      rejectHardlinks,
      rejectSymlinks,
    }),
  );
  let entryCount = 0;
  let regularFileBytes = 0n;
  const byteLimit = BigInt(maxBytes);

  while (pendingDirectories.length > 0) {
    const {
      label,
      path: directoryPath,
      rejectHardlinks,
      rejectSymlinks,
    } = pendingDirectories.pop();
    let directory;
    try {
      directory = opendirSync(directoryPath);
    } catch (error) {
      if (tolerateRaces && error.code === "ENOENT") continue;
      return `${label}-scan-failed`;
    }
    try {
      for (;;) {
        const entry = directory.readSync();
        if (!entry) break;
        entryCount += 1;
        if (entryCount > maxEntries) {
          return `${label}-entry-limit-exceeded`;
        }

        const entryPath = join(directoryPath, entry.name);
        let metadata;
        try {
          metadata = lstatSync(entryPath, { bigint: true });
        } catch (error) {
          if (tolerateRaces && error.code === "ENOENT") continue;
          return `${label}-scan-failed`;
        }
        if (metadata.isSymbolicLink()) {
          if (rejectSymlinks) return `${label}-symlink-detected`;
          continue;
        }
        if (metadata.isDirectory()) {
          pendingDirectories.push({
            label,
            path: entryPath,
            rejectHardlinks,
            rejectSymlinks,
          });
          continue;
        }
        if (!metadata.isFile()) {
          return `${label}-special-file-detected`;
        }
        if (rejectHardlinks && metadata.nlink > 1n) {
          return `${label}-hardlink-detected`;
        }
        regularFileBytes += metadata.size;
        if (regularFileBytes > byteLimit) {
          return `${label}-byte-limit-exceeded`;
        }
      }
    } catch {
      return `${label}-scan-failed`;
    } finally {
      try {
        directory.closeSync();
      } catch {
        // A completed read may already have closed the handle.
      }
    }
  }
  return null;
}

function copyTreeSnapshot(
  source,
  destination,
  label,
  maxBytes,
  maxEntries,
) {
  try {
    cpSync(source, destination, {
      errorOnExist: true,
      force: false,
      preserveTimestamps: true,
      recursive: true,
      verbatimSymlinks: true,
    });
  } catch {
    fail("runtime", `${label}-snapshot-failed`, 70);
  }
  const safetyError = scanWritableRoots(
    [
      {
        label,
        path: destination,
        rejectHardlinks: true,
        rejectSymlinks: true,
      },
    ],
    maxBytes,
    maxEntries,
  );
  if (safetyError) fail("safety", safetyError, 77);
  return destination;
}

function assertSafeConfigOverrides(overrides, workspace, safeToolPath) {
  const reasoningEfforts = [];
  const pluginFlags = [];
  const exactValues = new Map([
    ["approval_policy", '"never"'],
    ["features.apps", "false"],
    ["features.auth_elicitation", "false"],
    ["features.browser_use", "false"],
    ["features.browser_use_external", "false"],
    ["features.browser_use_full_cdp_access", "false"],
    ["features.code_mode_host", "false"],
    ["features.computer_use", "false"],
    ["features.enable_fanout", "false"],
    ["features.enable_request_compression", "true"],
    ["features.fast_mode", "false"],
    ["features.goals", "false"],
    ["features.guardian_approval", "true"],
    ["features.hooks", "false"],
    ["features.image_generation", "false"],
    ["features.in_app_browser", "false"],
    ["features.mentions_v2", "false"],
    ["features.multi_agent", "false"],
    ["features.personality", "false"],
    ["features.plugin_sharing", "false"],
    ["features.remote_compaction_v2", "false"],
    ["features.remote_plugin", "false"],
    ["features.secret_auth_storage", "false"],
    ["features.shell_snapshot", "false"],
    ["features.shell_tool", "true"],
    ["features.skill_mcp_dependency_install", "false"],
    ["features.tool_call_mcp_elicitation", "false"],
    ["features.tool_suggest", "false"],
    ["features.unified_exec", "true"],
    ["features.web_search_request", "false"],
    ["features.workspace_dependencies", "false"],
    ["history.persistence", '"none"'],
    ["sandbox_workspace_write.exclude_slash_tmp", "true"],
    ["sandbox_workspace_write.exclude_tmpdir_env_var", "true"],
    ["sandbox_workspace_write.network_access", "false"],
    ["sandbox_workspace_write.writable_roots", "[]"],
    ["shell_environment_policy.experimental_use_profile", "false"],
    ["shell_environment_policy.ignore_default_excludes", "false"],
    ["shell_environment_policy.inherit", '"none"'],
    ["shell_environment_policy.set.CARGO_HOME", `"${workspace}/.cargo-home"`],
    ["shell_environment_policy.set.CARGO_TARGET_DIR", `"${workspace}/target"`],
    ["shell_environment_policy.set.GIT_CONFIG_GLOBAL", '"/dev/null"'],
    ["shell_environment_policy.set.GIT_CONFIG_NOSYSTEM", '"1"'],
    ["shell_environment_policy.set.HOME", `"${workspace}/.home"`],
    ["shell_environment_policy.set.LANG", '"C.UTF-8"'],
    ["shell_environment_policy.set.LC_ALL", '"C.UTF-8"'],
    ["shell_environment_policy.set.PATH", JSON.stringify(safeToolPath)],
    ["shell_environment_policy.set.TMPDIR", `"${workspace}/.tmp"`],
    ["web_search", '"disabled"'],
  ]);

  for (const override of overrides) {
    const equals = override.indexOf("=");
    if (equals <= 0) {
      fail("invocation", "malformed-config-override", 64);
    }
    const key = override.slice(0, equals).trim();
    const value = override.slice(equals + 1).trim();
    if (!/^[A-Za-z][A-Za-z0-9_.]*$/.test(key)) {
      fail("invocation", "invalid-config-override-key", 64);
    }
    if (key === "sandbox_workspace_write.writable_roots" && value !== "[]") {
      fail("invocation", "unsafe-writable-roots", 64);
    }
    if (key === "sandbox_workspace_write.network_access" && value !== "false") {
      fail("invocation", "unsafe-network-override", 64);
    }
    if (key === "features.plugins" && /^(true|false)$/.test(value)) {
      pluginFlags.push(value);
      continue;
    }
    if (
      key === "model_reasoning_effort" &&
      /^"(low|medium|high|xhigh)"$/.test(value)
    ) {
      reasoningEfforts.push(value);
      continue;
    }
    if (exactValues.get(key) === value) continue;
    fail("invocation", `unsupported-config-override-${key}`, 64);
  }
  return { pluginFlags, reasoningEfforts };
}

const argv = process.argv.slice(2);
if (argv[0] !== "exec") {
  fail("invocation", "exec-required", 64);
}

function parseSdkInvocation(options) {
  const forwardedOptions = [];
  const configOverrides = [];
  const models = [];
  const workspaces = [];
  const sandboxes = [];
  const noValueOptions = new Set([
    "--experimental-json",
    "--json",
    "--skip-git-repo-check",
    "--strict-config",
    "--help",
    "-h",
    "--version",
    "-V",
  ]);
  const valueOptions = new Map([
    ["--config", "config"],
    ["-c", "config"],
    ["--model", "model"],
    ["-m", "model"],
    ["--sandbox", "sandbox"],
    ["-s", "sandbox"],
    ["--cd", "workspace"],
    ["-C", "workspace"],
    ["--output-schema", "output-schema"],
    ["--color", "color"],
  ]);

  for (let index = 0; index < options.length; index += 1) {
    const argument = options[index];
    if (argument === "--ephemeral") continue;
    if (argument === "--") {
      fail("invocation", "positional-arguments-forbidden", 64);
    }
    if (noValueOptions.has(argument)) {
      forwardedOptions.push(argument);
      continue;
    }

    let option = argument;
    let value;
    if (argument.startsWith("--") && argument.includes("=")) {
      const equals = argument.indexOf("=");
      option = argument.slice(0, equals);
      value = argument.slice(equals + 1);
    }
    const kind = valueOptions.get(option);
    if (!kind) {
      fail("invocation", "unsupported-codex-option", 64);
    }
    if (value === undefined) {
      if (index + 1 >= options.length) {
        fail("invocation", `${kind}-value-required`, 64);
      }
      value = options[index + 1];
      index += 1;
      forwardedOptions.push(argument, value);
    } else {
      forwardedOptions.push(argument);
    }
    if (!value) fail("invocation", `${kind}-value-required`, 64);
    if (value.startsWith("-")) {
      fail("invocation", `option-like-${kind}-value`, 64);
    }
    if (kind === "config") configOverrides.push(value);
    if (kind === "model") models.push(value);
    if (kind === "workspace") workspaces.push(value);
    if (kind === "sandbox") sandboxes.push(value);
  }

  if (workspaces.length !== 1) {
    fail("invocation", "exactly-one-cd-required", 64);
  }
  if (sandboxes.length !== 1 || sandboxes[0] !== "workspace-write") {
    fail("invocation", "unsafe-sandbox-mode", 64);
  }
  return {
    configOverrides,
    forwardedOptions,
    models,
    workspace: workspaces[0],
  };
}

const parsedInvocation = parseSdkInvocation(argv.slice(1));
const workspace = parsedInvocation.workspace;
const sandboxWorkspace = "/workspace";
const sandboxCodexHome = "/runtime/codex-home";
const sandboxPrivateTmp = "/runtime/tmp";

function rewriteConfigWorkspace(override) {
  return override.replaceAll(workspace, sandboxWorkspace);
}

function rewriteForwardedOptions(options) {
  const rewritten = [];
  for (let index = 0; index < options.length; index += 1) {
    const argument = options[index];
    if (argument === "--cd" || argument === "-C") {
      rewritten.push(argument, sandboxWorkspace);
      index += 1;
      continue;
    }
    if (argument.startsWith("--cd=")) {
      rewritten.push(`--cd=${sandboxWorkspace}`);
      continue;
    }
    if (argument === "--config" || argument === "-c") {
      rewritten.push(argument, rewriteConfigWorkspace(options[index + 1]));
      index += 1;
      continue;
    }
    if (argument.startsWith("--config=")) {
      rewritten.push(
        `--config=${rewriteConfigWorkspace(argument.slice("--config=".length))}`,
      );
      continue;
    }
    rewritten.push(argument);
  }
  return rewritten;
}

const optionArgs = rewriteForwardedOptions(parsedInvocation.forwardedOptions);
const discoveryCanaryRaw = process.env.CODE_QUALITY_CODEX_DISCOVERY_CANARY;
if (discoveryCanaryRaw !== undefined && discoveryCanaryRaw !== "1") {
  fail("configuration", "invalid-discovery-canary", 64);
}
const discoveryCanary = discoveryCanaryRaw === "1";
const discoveryPrompt = process.env.CODE_QUALITY_CODEX_DISCOVERY_PROMPT;
if (
  (discoveryCanary &&
    (!discoveryPrompt || Buffer.byteLength(discoveryPrompt) > 65_536)) ||
  (!discoveryCanary && discoveryPrompt !== undefined)
) {
  fail("configuration", "invalid-discovery-prompt", 64);
}
const enforcedSafetyArgs = [
  "--config",
  "sandbox_workspace_write.network_access=false",
  "--config",
  'web_search="disabled"',
  "--config",
  'approval_policy="never"',
  "--config",
  'shell_environment_policy.inherit="none"',
  "--config",
  "shell_environment_policy.experimental_use_profile=false",
  "--config",
  "shell_environment_policy.ignore_default_excludes=false",
];
const forwardedArgv = discoveryCanary
  ? [
      "debug",
      "prompt-input",
      "--config",
      "features.goals=false",
      "--config",
      "features.shell_snapshot=false",
      "--config",
      'history.persistence="none"',
      "--",
      discoveryPrompt,
    ]
  : ["exec", "--ephemeral", ...optionArgs, ...enforcedSafetyArgs];

const bwrap = process.env.CODE_QUALITY_BWRAP_BIN;
const codexHome = process.env.CODEX_HOME;
const privateTmp = process.env.TMPDIR;
const realCodex = process.env.CODE_QUALITY_CODEX_REAL_BIN;
const expectedSha256 = process.env.CODE_QUALITY_CODEX_EXPECTED_SHA256;
const expectedVersion = process.env.CODE_QUALITY_CODEX_EXPECTED_VERSION;
const expectedResourceBwrapSha256 =
  process.env.CODE_QUALITY_CODEX_RESOURCE_BWRAP_EXPECTED_SHA256;
const expectedRgSha256 = process.env.CODE_QUALITY_CODEX_RG_EXPECTED_SHA256;
const safeToolPath = process.env.CODE_QUALITY_TOOL_PATH;
const apiKey = process.env.OPENAI_API_KEY;
const timeout = process.env.CODE_QUALITY_TIMEOUT_BIN;
const prlimit = process.env.CODE_QUALITY_PRLIMIT_BIN;
const systemdRun = process.env.CODE_QUALITY_SYSTEMD_RUN_BIN;
const nixStoreClosureFile = process.env.CODE_QUALITY_NIX_STORE_CLOSURE;
const nixStoreClosureExpectedSha256 =
  process.env.CODE_QUALITY_NIX_STORE_CLOSURE_EXPECTED_SHA256;
const bwrapExpectedSha256 = process.env.CODE_QUALITY_BWRAP_EXPECTED_SHA256;
const timeoutExpectedSha256 = process.env.CODE_QUALITY_TIMEOUT_EXPECTED_SHA256;
const prlimitExpectedSha256 = process.env.CODE_QUALITY_PRLIMIT_EXPECTED_SHA256;
const systemdRunExpectedSha256 =
  process.env.CODE_QUALITY_SYSTEMD_RUN_EXPECTED_SHA256;
const wallTimeoutSeconds = process.env.CODE_QUALITY_WALL_TIMEOUT_SECONDS;
const outputMaxBytesRaw = process.env.CODE_QUALITY_OUTPUT_MAX_BYTES;
const workspaceMaxBytesRaw = process.env.CODE_QUALITY_WORKSPACE_MAX_BYTES;
const workspaceMaxEntriesRaw = process.env.CODE_QUALITY_WORKSPACE_MAX_ENTRIES;
if (
  !bwrap ||
  !codexHome ||
  !privateTmp ||
  !realCodex ||
  !expectedSha256 ||
  !expectedVersion ||
  !expectedResourceBwrapSha256 ||
  !expectedRgSha256 ||
  !safeToolPath ||
  !apiKey ||
  !timeout ||
  !prlimit ||
  !systemdRun ||
  !nixStoreClosureFile ||
  !nixStoreClosureExpectedSha256 ||
  !bwrapExpectedSha256 ||
  !timeoutExpectedSha256 ||
  !prlimitExpectedSha256 ||
  !systemdRunExpectedSha256 ||
  !wallTimeoutSeconds ||
  !outputMaxBytesRaw ||
  !workspaceMaxBytesRaw ||
  !workspaceMaxEntriesRaw
) {
  fail("configuration", "missing-runtime-path", 64);
}
const invocationConfig = assertSafeConfigOverrides(
  parsedInvocation.configOverrides,
  workspace,
  safeToolPath,
);
const outputMaxBytes = boundedPositiveInteger(
  outputMaxBytesRaw,
  "output-max-bytes",
  67_108_864,
);
const workspaceMaxBytes = boundedPositiveInteger(
  workspaceMaxBytesRaw,
  "workspace-max-bytes",
  4_294_967_296,
);
const workspaceMaxEntries = boundedPositiveInteger(
  workspaceMaxEntriesRaw,
  "workspace-max-entries",
  200_000,
);
if (
  !/^[1-9][0-9]*$/.test(wallTimeoutSeconds) ||
  Number(wallTimeoutSeconds) > 7_200
) {
  fail("configuration", "invalid-wall-timeout", 64);
}
if (!/^[0-9a-f]{64}$/.test(expectedSha256)) {
  fail("configuration", "invalid-expected-sha256", 64);
}

try {
  const binaryMetadata = lstatSync(realCodex);
  if (
    !isAbsolute(realCodex) ||
    resolve(realCodex) !== realCodex ||
    realpathSync(realCodex) !== realCodex ||
    !binaryMetadata.isFile()
  ) {
    fail("configuration", "codex-binary-not-canonical", 64);
  }
  accessSync(realCodex, constants.R_OK | constants.X_OK);
} catch {
  fail("configuration", "codex-binary-not-canonical", 64);
}

let runtimeRoot;
let packageManifestPath;
let packageManifestContents;
let resourceBwrap;
let resourceRg;
try {
  runtimeRoot = realpathSync(dirname(dirname(realCodex)));
  packageManifestPath = join(runtimeRoot, "codex-package.json");
  packageManifestContents = readFileSync(packageManifestPath, "utf8");
  const packageManifest = JSON.parse(packageManifestContents);
  if (
    packageManifest.layoutVersion !== 1 ||
    packageManifest.entrypoint !== "bin/codex" ||
    packageManifest.variant !== "codex" ||
    packageManifest.resourcesDir !== "codex-resources" ||
    packageManifest.pathDir !== "codex-path" ||
    expectedVersion !== `codex-cli ${packageManifest.version}` ||
    !packageManifest.target?.endsWith("linux-musl")
  ) {
    fail("configuration", "codex-package-invalid", 64);
  }
  if (realCodex !== join(runtimeRoot, packageManifest.entrypoint)) {
    fail("configuration", "codex-entrypoint-mismatch", 64);
  }
  resourceBwrap = join(runtimeRoot, packageManifest.resourcesDir, "bwrap");
  resourceRg = join(runtimeRoot, packageManifest.pathDir, "rg");
} catch {
  fail("configuration", "codex-package-invalid", 64);
}
resourceBwrap = assertPinnedFile(
  resourceBwrap,
  expectedResourceBwrapSha256,
  "codex-resource-bwrap",
);
resourceRg = assertPinnedFile(resourceRg, expectedRgSha256, "codex-rg");

const workspaceMetadata = canonicalDirectory(
  workspace,
  "workspace-not-canonical",
);
const codexHomeMetadata = canonicalDirectory(
  codexHome,
  "codex-home-not-canonical",
);
const privateTmpMetadata = canonicalDirectory(
  privateTmp,
  "private-tmp-not-canonical",
);
const canonicalBwrap = pinnedRuntimeExecutable(
  bwrap,
  bwrapExpectedSha256,
  "bwrap",
);
const canonicalTimeout = pinnedRuntimeExecutable(
  timeout,
  timeoutExpectedSha256,
  "timeout",
);
const canonicalPrlimit = pinnedRuntimeExecutable(
  prlimit,
  prlimitExpectedSha256,
  "prlimit",
);
const canonicalSystemdRun = pinnedRuntimeExecutable(
  systemdRun,
  systemdRunExpectedSha256,
  "systemd-run",
);
for (const runtimeTool of [
  canonicalBwrap,
  canonicalTimeout,
  canonicalPrlimit,
  canonicalSystemdRun,
]) {
  if (
    pathsOverlap(workspace, runtimeTool) ||
    pathsOverlap(codexHome, runtimeTool) ||
    pathsOverlap(privateTmp, runtimeTool)
  ) {
    fail("configuration", "runtime-tool-overlaps-writable-state", 64);
  }
}
const repositoryRoot = dirname(
  dirname(dirname(fileURLToPath(import.meta.url))),
);
if (pathsOverlap(workspace, repositoryRoot)) {
  fail("configuration", "workspace-overlaps-repository", 64);
}
if (
  pathsOverlap(workspace, codexHome) ||
  pathsOverlap(workspace, privateTmp) ||
  pathsOverlap(codexHome, privateTmp) ||
  pathsOverlap(runtimeRoot, workspace) ||
  pathsOverlap(runtimeRoot, codexHome) ||
  pathsOverlap(runtimeRoot, privateTmp)
) {
  fail("configuration", "path-overlap", 64);
}
if ((codexHomeMetadata.mode & 0o077) !== 0) {
  fail("configuration", "codex-home-not-private", 64);
}
if ((privateTmpMetadata.mode & 0o077) !== 0) {
  fail("configuration", "private-tmp-not-private", 64);
}
const codexHomeMarker = canonicalRegularFile(
  join(codexHome, ".ai-plugins-eval-home"),
  "codex-home-marker-invalid",
);
try {
  if (
    readFileSync(codexHomeMarker, "utf8") !== "ai-plugins Codex eval home\n"
  ) {
    fail("configuration", "codex-home-marker-invalid", 64);
  }
} catch {
  fail("configuration", "codex-home-marker-invalid", 64);
}
const codexHomeConfig = canonicalRegularFile(
  join(codexHome, "config.toml"),
  "codex-home-config-invalid",
);
const codexHomeExecutionSurface = canonicalRegularFile(
  join(codexHome, ".ai-plugins-execution-surface.json"),
  "codex-home-execution-surface-invalid",
);
let executionSurface;
let codexHomeConfigContents;
try {
  const executionSurfaceBytes = readFileSync(codexHomeExecutionSurface);
  executionSurface = parseExecutionSurface(
    JSON.parse(executionSurfaceBytes.toString("utf8")),
  );
  if (
    executionSurfaceBytes.toString("utf8") !==
    `${canonicalJson(executionSurface, 2)}\n`
  ) {
    fail("configuration", "codex-home-execution-surface-invalid", 64);
  }
  codexHomeConfigContents = readFileSync(codexHomeConfig, "utf8");
} catch {
  fail("configuration", "codex-home-execution-surface-invalid", 64);
}
if (!discoveryCanary) {
  if (parsedInvocation.models.length !== 1) {
    fail("invocation", "exactly-one-model-required", 64);
  }
  if (parsedInvocation.models[0] !== executionSurface.model) {
    fail("invocation", "model-does-not-match-execution-surface", 64);
  }
  if (invocationConfig.reasoningEfforts.length !== 1) {
    fail("invocation", "exactly-one-reasoning-effort-required", 64);
  }
  if (
    invocationConfig.reasoningEfforts[0] !==
    JSON.stringify(executionSurface.reasoningEffort)
  ) {
    fail(
      "invocation",
      "reasoning-effort-does-not-match-execution-surface",
      64,
    );
  }
  if (invocationConfig.pluginFlags.length !== 1) {
    fail("invocation", "exactly-one-plugin-flag-required", 64);
  }
  const expectedPluginFlag = /^\[plugins\."[a-z0-9-]+@ai-plugins"\]$/m.test(
    codexHomeConfigContents,
  )
    ? "true"
    : "false";
  if (invocationConfig.pluginFlags[0] !== expectedPluginFlag) {
    fail("invocation", "plugin-flag-does-not-match-runtime", 64);
  }
}
const codexHomePlugins = optionalCanonicalDirectory(
  join(codexHome, "plugins"),
  "codex-home-plugins-unsafe",
);
if (
  codexHomePlugins &&
  scanWritableRoots(
    [{ label: "plugins", path: codexHomePlugins, rejectSymlinks: true }],
    workspaceMaxBytes,
    workspaceMaxEntries,
  )
) {
  fail("configuration", "codex-home-plugins-unsafe", 64);
}
const codexHomeSystemSkills = join(codexHome, "skills/.system");
canonicalDirectory(codexHomeSystemSkills, "codex-home-system-skills-unsafe");
if (
  scanWritableRoots(
    [
      {
        label: "system-skills",
        path: codexHomeSystemSkills,
        rejectSymlinks: true,
      },
    ],
    workspaceMaxBytes,
    workspaceMaxEntries,
  )
) {
  fail("configuration", "codex-home-system-skills-unsafe", 64);
}
const codexHomeMarketplace = join(codexHome, "marketplace");
canonicalDirectory(codexHomeMarketplace, "codex-home-marketplace-unsafe");
if (
  scanWritableRoots(
    [
      {
        label: "marketplace",
        path: codexHomeMarketplace,
        rejectSymlinks: true,
      },
    ],
    workspaceMaxBytes,
    workspaceMaxEntries,
  )
) {
  fail("configuration", "codex-home-marketplace-unsafe", 64);
}
try {
  if (
    readFileSync(
      join(workspace, ".git/.ai-plugins-code-quality-workspace"),
      "utf8",
    ) !== "ai-plugins downstream code-quality workspace\n"
  ) {
    fail("configuration", "workspace-marker-invalid", 64);
  }
} catch {
  fail("configuration", "workspace-marker-invalid", 64);
}

const writableRoots = [
  {
    label: "workspace",
    path: workspace,
    rejectHardlinks: true,
    rejectSymlinks: true,
  },
  { label: "state", path: codexHome, rejectSymlinks: false },
  { label: "state", path: privateTmp, rejectSymlinks: false },
];
const initialWritableStateSafetyError = scanWritableRoots(
  writableRoots,
  workspaceMaxBytes,
  workspaceMaxEntries,
);
if (initialWritableStateSafetyError) {
  fail("safety", initialWritableStateSafetyError, 77);
}

let actualSha256;
try {
  actualSha256 = createHash("sha256")
    .update(readFileSync(realCodex))
    .digest("hex");
} catch {
  fail("configuration", "codex-binary-unreadable", 64);
}
if (actualSha256 !== expectedSha256) {
  fail("integrity", "sha256-mismatch", 65);
}

const safeToolDirectories = validatedToolDirectories(safeToolPath);
const shell = executableFromSafePath(safeToolDirectories, "bash");
const envTool = executableFromSafePath(safeToolDirectories, "env");
const copyTool = executableFromSafePath(safeToolDirectories, "cp");
const findTool = executableFromSafePath(safeToolDirectories, "find");
const removeTool = executableFromSafePath(safeToolDirectories, "rm");
const tarTool = executableFromSafePath(safeToolDirectories, "tar");
const awkTool = executableFromSafePath(safeToolDirectories, "awk");
const catTool = executableFromSafePath(safeToolDirectories, "cat");
const wcTool = executableFromSafePath(safeToolDirectories, "wc");
const nixStoreClosure = validatedNixStoreClosure(
  nixStoreClosureFile,
  nixStoreClosureExpectedSha256,
  safeToolDirectories,
);
const nixStoreMounts = nixStoreClosure.flatMap((storePath) => [
  "--ro-bind",
  storePath,
  storePath,
]);
const uid = process.getuid?.() ?? 65_534;
const gid = process.getgid?.() ?? 65_534;
const systemdRuntimeDir = `/run/user/${uid}`;
const systemdRuntimeMetadata = canonicalDirectory(
  systemdRuntimeDir,
  "systemd-runtime-dir-unavailable",
);
if (
  systemdRuntimeMetadata.uid !== uid ||
  (systemdRuntimeMetadata.mode & 0o077) !== 0
) {
  fail("configuration", "systemd-runtime-dir-unsafe", 64);
}
const resolverConfig = requiredHostFile(
  "/etc/resolv.conf",
  "resolver-config-unavailable",
);
const certificateBundle = requiredHostFile(
  "/etc/ssl/certs/ca-certificates.crt",
  "certificate-bundle-unavailable",
);
const runtimeSupport = mkdtempSync(join("/tmp", "workspace-runtime-"));
activeRuntimeSupport = runtimeSupport;
chmodSync(runtimeSupport, 0o700);
const packageSnapshot = join(runtimeSupport, "codex-package");
const packageSnapshotBin = join(packageSnapshot, "bin");
const packageSnapshotResources = join(packageSnapshot, "codex-resources");
const packageSnapshotPath = join(packageSnapshot, "codex-path");
const runtimeToolSnapshot = join(runtimeSupport, "runtime-tools");
const versionProbeHome = join(runtimeSupport, "version-probe-home");
const versionProbeTmp = join(runtimeSupport, "version-probe-tmp");
const codexHomeInputs = join(runtimeSupport, "codex-home-inputs");
const codexHomeInputSkills = join(codexHomeInputs, "skills");
for (const directory of [
  packageSnapshot,
  packageSnapshotBin,
  packageSnapshotResources,
  packageSnapshotPath,
  runtimeToolSnapshot,
  versionProbeHome,
  versionProbeTmp,
  codexHomeInputs,
  codexHomeInputSkills,
]) {
  mkdirSync(directory, { mode: 0o700 });
}
const snapshotCodexHomeConfig = join(codexHomeInputs, "config.toml");
try {
  copyFileSync(
    codexHomeConfig,
    snapshotCodexHomeConfig,
    constants.COPYFILE_FICLONE,
  );
  chmodSync(snapshotCodexHomeConfig, 0o400);
  if (readFileSync(snapshotCodexHomeConfig, "utf8") !== codexHomeConfigContents) {
    fail("integrity", "codex-home-config-snapshot-mismatch", 65);
  }
} catch {
  fail("runtime", "codex-home-config-snapshot-failed", 70);
}
const snapshotCodexHomeSystemSkills = copyTreeSnapshot(
  codexHomeSystemSkills,
  join(codexHomeInputSkills, ".system"),
  "system-skills",
  workspaceMaxBytes,
  workspaceMaxEntries,
);
const snapshotCodexHomePlugins = codexHomePlugins
  ? copyTreeSnapshot(
      codexHomePlugins,
      join(codexHomeInputs, "plugins"),
      "plugins",
      workspaceMaxBytes,
      workspaceMaxEntries,
    )
  : null;
const snapshotMarketplace = copyTreeSnapshot(
  codexHomeMarketplace,
  join(runtimeSupport, "marketplace"),
  "marketplace",
  workspaceMaxBytes,
  workspaceMaxEntries,
);
const snapshotWorkspaceInput = copyTreeSnapshot(
  workspace,
  join(runtimeSupport, "workspace-input"),
  "workspace",
  workspaceMaxBytes,
  workspaceMaxEntries,
);
const snapshotWorkspaceOutput = copyTreeSnapshot(
  workspace,
  join(runtimeSupport, "workspace-output"),
  "workspace",
  workspaceMaxBytes,
  workspaceMaxEntries,
);
const stagedWorkspaceWritableRoots = [
  {
    label: "workspace",
    path: snapshotWorkspaceOutput,
    rejectHardlinks: true,
    rejectSymlinks: true,
  },
];
function scanBoundaryWritableState(tolerateRaces = false) {
  return (
    scanWritableRoots(
      writableRoots,
      workspaceMaxBytes,
      workspaceMaxEntries,
      tolerateRaces,
    ) ??
    scanWritableRoots(
      stagedWorkspaceWritableRoots,
      workspaceMaxBytes,
      workspaceMaxEntries,
      tolerateRaces,
    )
  );
}
const workspaceExportCompleteMarker = join(
  snapshotWorkspaceOutput,
  ".git/.workspace-export-complete",
);
function publishStagedWorkspace() {
  try {
    if (readFileSync(workspaceExportCompleteMarker, "utf8") !== "complete\n") {
      return "workspace-export-incomplete";
    }
    if (
      readFileSync(
        join(snapshotWorkspaceOutput, ".git/.ai-plugins-code-quality-workspace"),
        "utf8",
      ) !== "ai-plugins downstream code-quality workspace\n" ||
      readFileSync(
        join(workspace, ".git/.ai-plugins-code-quality-workspace"),
        "utf8",
      ) !== "ai-plugins downstream code-quality workspace\n"
    ) {
      return "trusted-workspace-marker-lost";
    }
  } catch {
    return "workspace-export-incomplete";
  }

  const safetyError = scanWritableRoots(
    stagedWorkspaceWritableRoots,
    workspaceMaxBytes,
    workspaceMaxEntries,
  );
  if (safetyError) return safetyError;

  try {
    for (const name of readdirSync(workspace)) {
      if (name === ".git") continue;
      rmSync(join(workspace, name), { force: true, recursive: true });
    }
    for (const name of readdirSync(snapshotWorkspaceOutput)) {
      if (name === ".git") continue;
      cpSync(join(snapshotWorkspaceOutput, name), join(workspace, name), {
        errorOnExist: true,
        force: false,
        preserveTimestamps: true,
        recursive: true,
        verbatimSymlinks: true,
      });
    }
  } catch {
    return "workspace-publish-failed";
  }
  return null;
}
const snapshotCodex = snapshotPinnedFile(
  realCodex,
  join(packageSnapshotBin, "codex"),
  expectedSha256,
  "codex",
);
const versionProbe = spawnSync(snapshotCodex, ["--version"], {
  encoding: "utf8",
  env: {
    CODEX_HOME: versionProbeHome,
    HOME: versionProbeHome,
    LANG: "C.UTF-8",
    LC_ALL: "C.UTF-8",
    PATH: safeToolPath,
    TMPDIR: versionProbeTmp,
  },
  stdio: ["ignore", "pipe", "ignore"],
  timeout: 5_000,
});
if (versionProbe.error || versionProbe.status !== 0) {
  fail("integrity", "version-probe-failed", 65);
}
if (versionProbe.stdout.trim() !== expectedVersion) {
  fail("integrity", "version-mismatch", 65);
}
const snapshotResourceBwrap = snapshotPinnedFile(
  resourceBwrap,
  join(packageSnapshotResources, "bwrap"),
  expectedResourceBwrapSha256,
  "codex-resource-bwrap",
);
const snapshotResourceRg = snapshotPinnedFile(
  resourceRg,
  join(packageSnapshotPath, "rg"),
  expectedRgSha256,
  "codex-rg",
);
const snapshotPackageManifest = join(packageSnapshot, "codex-package.json");
writeFileSync(snapshotPackageManifest, packageManifestContents, {
  mode: 0o400,
});
const snapshotBwrap = snapshotPinnedFile(
  canonicalBwrap,
  join(runtimeToolSnapshot, "bwrap"),
  bwrapExpectedSha256,
  "bwrap",
);
const snapshotTimeout = snapshotPinnedFile(
  canonicalTimeout,
  join(runtimeToolSnapshot, "timeout"),
  timeoutExpectedSha256,
  "timeout",
);
const snapshotPrlimit = snapshotPinnedFile(
  canonicalPrlimit,
  join(runtimeToolSnapshot, "prlimit"),
  prlimitExpectedSha256,
  "prlimit",
);
const snapshotSystemdRun = snapshotPinnedFile(
  canonicalSystemdRun,
  join(runtimeToolSnapshot, "systemd-run"),
  systemdRunExpectedSha256,
  "systemd-run",
);
const scopeEntryMarker = join(runtimeSupport, "resource-scope-entered");
const resourceScopeEntry = join(runtimeSupport, "resource-scope-entry");
writeFileSync(
  resourceScopeEntry,
  [
    `#!${shell}`,
    "set -eu",
    "cgroup_path=",
    "while IFS=: read -r hierarchy controllers candidate; do",
    '  if [ "$hierarchy" = 0 ] && [ -z "$controllers" ]; then',
    "    cgroup_path=$candidate",
    "    break",
    "  fi",
    "done </proc/self/cgroup",
    'case "$cgroup_path" in',
    "  /*) ;;",
    "  *) exit 70 ;;",
    "esac",
    'oom_group="/sys/fs/cgroup${cgroup_path}/memory.oom.group"',
    '[ -f "$oom_group" ] && [ ! -L "$oom_group" ]',
    'printf "%s\\n" 1 >"$oom_group"',
    'IFS= read -r oom_group_value <"$oom_group"',
    '[ "$oom_group_value" = 1 ]',
    "scope_entry_marker=$1",
    "shift",
    'printf "%s\\n" entered >"$scope_entry_marker"',
    "unset XDG_RUNTIME_DIR",
    'exec "$@"',
    "",
  ].join("\n"),
  { mode: 0o500 },
);
const workspaceExporter = join(runtimeSupport, "workspace-exporter");
writeFileSync(
  workspaceExporter,
  `#!${shell}
set -uo pipefail
awk_tool=${JSON.stringify(awkTool)}
cat_tool=${JSON.stringify(catTool)}
copy_tool=${JSON.stringify(copyTool)}
find_tool=${JSON.stringify(findTool)}
remove_tool=${JSON.stringify(removeTool)}
tar_tool=${JSON.stringify(tarTool)}
wc_tool=${JSON.stringify(wcTool)}
workspace=$1
workspace_max_bytes=$2
workspace_max_entries=$3
shift 3

if ! "$copy_tool" -a -- /runtime/workspace-input/. "$workspace"/; then
  printf "%s\\n" "CODE_QUALITY_BOUNDARY_ERROR:safety:workspace-copy-in-failed" >&2
  exit 77
fi
if ! "$remove_tool" -f -- "$workspace/.git/.ai-plugins-code-quality-workspace"; then
  printf "%s\\n" "CODE_QUALITY_BOUNDARY_ERROR:safety:workspace-copy-in-failed" >&2
  exit 77
fi

set +e
/runtime/codex-package/codex-resources/bwrap \
  --unshare-user \
  --unshare-pid \
  --die-with-parent \
  --new-session \
  --ro-bind / / \
  --proc /proc \
  --dev-bind /dev /dev \
  --tmpfs /runtime/workspace-input \
  --tmpfs /runtime/workspace-output \
  --ro-bind /dev/null /runtime/workspace-exporter \
  --bind "$workspace" "$workspace" \
  --bind "$CODEX_HOME" "$CODEX_HOME" \
  --bind "$TMPDIR" "$TMPDIR" \
  --size 1048576 \
  --tmpfs /runtime/workspace-output \
  --chdir "$workspace" \
  -- \
  /runtime/codex-package/bin/codex "$@"
codex_status=$?
set -e

unsafe_entry="$("$find_tool" "$workspace" -xdev -path "$workspace/.git" -prune -o ! -type d ! -type f -print -quit)"
unsafe_hardlink="$("$find_tool" "$workspace" -xdev -path "$workspace/.git" -prune -o -type f -links +1 -print -quit)"
if [ -n "$unsafe_entry" ] || [ -n "$unsafe_hardlink" ]; then
  printf "%s\\n" "CODE_QUALITY_BOUNDARY_ERROR:safety:workspace-export-unsafe" >&2
  exit 77
fi

entry_count="$("$find_tool" "$workspace" -xdev -mindepth 1 -path "$workspace/.git" -prune -o -printf x | "$wc_tool" -c)"
if [ "$entry_count" -gt "$workspace_max_entries" ]; then
  printf "%s\\n" "CODE_QUALITY_BOUNDARY_ERROR:safety:workspace-entry-limit-exceeded" >&2
  exit 77
fi
if ! "$find_tool" "$workspace" -xdev -path "$workspace/.git" -prune -o -type f -printf "%s\\n" |
  "$awk_tool" -v limit="$workspace_max_bytes" '{ total += $1; if (total > limit) exit 1 }'; then
  printf "%s\\n" "CODE_QUALITY_BOUNDARY_ERROR:safety:workspace-byte-limit-exceeded" >&2
  exit 77
fi

if [ "$("$cat_tool" /runtime/workspace-output/.git/.ai-plugins-code-quality-workspace)" != "ai-plugins downstream code-quality workspace" ]; then
  printf "%s\\n" "CODE_QUALITY_BOUNDARY_ERROR:safety:trusted-workspace-marker-lost" >&2
  exit 77
fi
if ! "$find_tool" /runtime/workspace-output -mindepth 1 -maxdepth 1 ! -name .git -exec "$remove_tool" -rf -- {} +; then
  printf "%s\\n" "CODE_QUALITY_BOUNDARY_ERROR:safety:workspace-copy-out-failed" >&2
  exit 77
fi
if ! (cd "$workspace" && "$tar_tool" --create --format=posix --exclude=./.git --file=- .) |
  "$tar_tool" --extract --file=- --directory=/runtime/workspace-output --no-same-owner; then
  printf "%s\\n" "CODE_QUALITY_BOUNDARY_ERROR:safety:workspace-copy-out-failed" >&2
  exit 77
fi
if ! printf "%s\\n" complete >/runtime/workspace-output/.git/.workspace-export-complete; then
  printf "%s\\n" "CODE_QUALITY_BOUNDARY_ERROR:safety:workspace-copy-out-failed" >&2
  exit 77
fi
exit "$codex_status"
`,
  { mode: 0o500 },
);
const safeShell = join(runtimeSupport, "safe-shell");
writeFileSync(
  safeShell,
  `#!${shell}\n` +
    "unset OPENAI_API_KEY CODEX_API_KEY ANTHROPIC_API_KEY AZURE_OPENAI_API_KEY\n" +
    `exec ${shell} "$@"\n`,
  { mode: 0o500 },
);
const runtimeFiles = {
  group: `codex:x:${gid}:\n`,
  hosts: "127.0.0.1 localhost\n::1 localhost\n",
  "nsswitch.conf": "passwd: files\ngroup: files\nhosts: files dns\n",
  passwd: `codex:x:${uid}:${gid}:Workspace user:${sandboxCodexHome}:/bin/bash\n`,
};
for (const [name, contents] of Object.entries(runtimeFiles)) {
  writeFileSync(join(runtimeSupport, name), contents, { mode: 0o400 });
}

const immutableCodexHomeMounts = [
  "--ro-bind",
  snapshotCodexHomeConfig,
  join(sandboxCodexHome, "config.toml"),
  "--dir",
  join(sandboxCodexHome, "skills"),
  "--ro-bind",
  snapshotCodexHomeSystemSkills,
  join(sandboxCodexHome, "skills/.system"),
];
if (snapshotCodexHomePlugins) {
  immutableCodexHomeMounts.push(
    "--ro-bind",
    snapshotCodexHomePlugins,
    join(sandboxCodexHome, "plugins"),
  );
}

const bwrapArgv = [
  "--unshare-all",
  "--share-net",
  "--die-with-parent",
  "--new-session",
  "--hostname",
  "workspace",
  "--size",
  "67108864",
  "--tmpfs",
  "/tmp",
  "--dev",
  "/dev",
  "--proc",
  "/proc",
  "--dir",
  "/bin",
  "--ro-bind",
  safeShell,
  "/bin/bash",
  "--symlink",
  "bash",
  "/bin/sh",
  "--dir",
  "/usr",
  "--dir",
  "/usr/bin",
  "--ro-bind",
  envTool,
  "/usr/bin/env",
  "--dir",
  "/etc",
  "--ro-bind",
  join(runtimeSupport, "passwd"),
  "/etc/passwd",
  "--ro-bind",
  join(runtimeSupport, "group"),
  "/etc/group",
  "--ro-bind",
  join(runtimeSupport, "hosts"),
  "/etc/hosts",
  "--ro-bind",
  join(runtimeSupport, "nsswitch.conf"),
  "/etc/nsswitch.conf",
  "--ro-bind",
  resolverConfig,
  "/etc/resolv.conf",
  "--dir",
  "/etc/ssl",
  "--dir",
  "/etc/ssl/certs",
  "--ro-bind",
  certificateBundle,
  "/etc/ssl/certs/ca-certificates.crt",
  "--dir",
  "/nix",
  "--dir",
  "/nix/store",
  ...nixStoreMounts,
  "--dir",
  "/runtime",
  "--dir",
  "/runtime/marketplace",
  "--ro-bind",
  snapshotMarketplace,
  "/runtime/marketplace",
  "--dir",
  "/runtime/codex-package",
  "--dir",
  "/runtime/codex-package/bin",
  "--dir",
  "/runtime/codex-package/codex-resources",
  "--dir",
  "/runtime/codex-package/codex-path",
  "--ro-bind",
  snapshotCodex,
  "/runtime/codex-package/bin/codex",
  "--ro-bind",
  snapshotPackageManifest,
  "/runtime/codex-package/codex-package.json",
  "--ro-bind",
  snapshotResourceBwrap,
  "/runtime/codex-package/codex-resources/bwrap",
  "--ro-bind",
  snapshotResourceRg,
  "/runtime/codex-package/codex-path/rg",
  "--dir",
  "/runtime/workspace-input",
  "--ro-bind",
  snapshotWorkspaceInput,
  "/runtime/workspace-input",
  "--dir",
  "/runtime/workspace-output",
  "--bind",
  snapshotWorkspaceOutput,
  "/runtime/workspace-output",
  "--ro-bind",
  workspaceExporter,
  "/runtime/workspace-exporter",
  "--dir",
  sandboxCodexHome,
  "--size",
  "134217728",
  "--tmpfs",
  sandboxCodexHome,
  ...immutableCodexHomeMounts,
  "--dir",
  sandboxPrivateTmp,
  "--size",
  "134217728",
  "--tmpfs",
  sandboxPrivateTmp,
  "--dir",
  sandboxWorkspace,
  "--size",
  `${workspaceMaxBytes}`,
  "--tmpfs",
  sandboxWorkspace,
  "--remount-ro",
  "/",
  "--setenv",
  "CODEX_HOME",
  sandboxCodexHome,
  "--setenv",
  "HOME",
  sandboxCodexHome,
  "--setenv",
  "TMPDIR",
  sandboxPrivateTmp,
  "--setenv",
  "PATH",
  safeToolPath,
  "--setenv",
  "SHELL",
  "/bin/bash",
  "--setenv",
  "LANG",
  "C.UTF-8",
  "--setenv",
  "LC_ALL",
  "C.UTF-8",
  "--setenv",
  "SSL_CERT_FILE",
  "/etc/ssl/certs/ca-certificates.crt",
  "--setenv",
  "NIX_SSL_CERT_FILE",
  "/etc/ssl/certs/ca-certificates.crt",
  "--setenv",
  "CODEX_INTERNAL_ORIGINATOR_OVERRIDE",
  "codex_sdk_ts",
  "--chdir",
  sandboxWorkspace,
  "--",
  "/runtime/workspace-exporter",
  sandboxWorkspace,
  `${workspaceMaxBytes}`,
  `${workspaceMaxEntries}`,
  ...forwardedArgv,
];

if (cancellationSignal) {
  fail("runtime", `cancelled-${cancellationSignal}`, 70);
}
const resourceScopeUnit = `ai-plugins-code-quality-${process.pid}`;
const child = spawn(
  snapshotSystemdRun,
  [
    "--user",
    "--scope",
    "--quiet",
    "--collect",
    "--expand-environment=false",
    `--unit=${resourceScopeUnit}`,
    "--property=MemoryMax=8589934592",
    "--property=MemorySwapMax=0",
    "--property=TasksMax=512",
    "--property=CPUQuota=400%",
    "--property=KillMode=control-group",
    "--",
    resourceScopeEntry,
    scopeEntryMarker,
    snapshotTimeout,
    "--signal=TERM",
    "--kill-after=5s",
    `${wallTimeoutSeconds}s`,
    snapshotPrlimit,
    "--as=8589934592",
    "--cpu=1800",
    "--fsize=1073741824",
    "--nproc=512",
    "--nofile=1024",
    "--core=0",
    "--",
    snapshotBwrap,
    ...bwrapArgv,
  ],
  {
    detached: true,
    env: {
      CODEX_API_KEY: apiKey,
      OPENAI_API_KEY: apiKey,
      PATH: safeToolPath,
      XDG_RUNTIME_DIR: systemdRuntimeDir,
    },
    stdio: ["inherit", "pipe", "pipe"],
  },
);
activeChildProcess = child;
if (cancellationSignal) {
  signalActiveChild(cancellationSignal);
  forceKillTimer = setTimeout(() => signalActiveChild("SIGKILL"), 5_000);
  forceKillTimer.unref();
}

let outputBytes = 0;
let outputLimitBreached = false;
let writableStateSafetyError;
let writableStateMonitor;

function monitorWritableState() {
  if (writableStateSafetyError) return;
  writableStateSafetyError = scanBoundaryWritableState(true);
  if (!writableStateSafetyError) return;
  signalActiveChild("SIGTERM");
  forceKillTimer = setTimeout(() => signalActiveChild("SIGKILL"), 5_000);
  forceKillTimer.unref();
}

monitorWritableState();
writableStateMonitor = setInterval(monitorWritableState, 100);
writableStateMonitor.unref();

function forwardBounded(source, destination) {
  return new Promise((resolveForwarding) => {
    let forwardingFinished = false;
    let pendingWrites = 0;
    let sourceFinished = false;

    function finishForwardingWhenDrained() {
      if (forwardingFinished || !sourceFinished || pendingWrites > 0) return;
      forwardingFinished = true;
      resolveForwarding();
    }

    function queueWrite(chunk) {
      pendingWrites += 1;
      const accepted = destination.write(chunk, () => {
        pendingWrites -= 1;
        finishForwardingWhenDrained();
      });
      if (!accepted) {
        source.pause();
        destination.once("drain", () => source.resume());
      }
    }

    function finishSource() {
      sourceFinished = true;
      finishForwardingWhenDrained();
    }

    source.once("end", finishSource);
    source.once("error", finishSource);
    source.on("data", (chunk) => {
      if (outputLimitBreached) return;
      const remaining = outputMaxBytes - outputBytes;
      if (chunk.length > remaining) {
        if (remaining > 0) queueWrite(chunk.subarray(0, remaining));
        outputBytes = outputMaxBytes;
        outputLimitBreached = true;
        signalActiveChild("SIGTERM");
        forceKillTimer = setTimeout(() => signalActiveChild("SIGKILL"), 5_000);
        forceKillTimer.unref();
        return;
      }
      outputBytes += chunk.length;
      queueWrite(chunk);
    });
  });
}

const forwardedOutputCompletion = Promise.all([
  forwardBounded(child.stdout, process.stdout),
  forwardBounded(child.stderr, process.stderr),
]);

let terminalHandled = false;

function finishDestination(destination) {
  if (destination.destroyed || destination.writableEnded) {
    return Promise.resolve();
  }
  return new Promise((resolveDestination) => {
    let resolved = false;
    function finish() {
      if (resolved) return;
      resolved = true;
      destination.off("finish", finish);
      destination.off("error", finish);
      destination.off("close", finish);
      resolveDestination();
    }
    destination.once("finish", finish);
    destination.once("error", finish);
    destination.once("close", finish);
    destination.end();
  });
}

function finishAfterOutputDrains(exitCode) {
  activeChildProcess = undefined;
  forwardedOutputCompletion
    .then(() =>
      Promise.all([
        finishDestination(process.stdout),
        finishDestination(process.stderr),
      ]),
    )
    .then(() => {
      process.exitCode = exitCode;
    }, handleInternalFailure);
}

child.once("error", (error) => {
  if (terminalHandled) return;
  terminalHandled = true;
  if (writableStateMonitor) clearInterval(writableStateMonitor);
  const cleanupSucceeded = cleanupRuntimeSupport();
  if (!cleanupSucceeded) {
    console.error("CODE_QUALITY_BOUNDARY_ERROR:runtime:cleanup-failed");
    finishAfterOutputDrains(70);
    return;
  }
  console.error(
    `CODE_QUALITY_BOUNDARY_ERROR:runtime:${error.code ?? "spawn-failed"}`,
  );
  finishAfterOutputDrains(69);
});

child.once("close", (code, signal) => {
  if (terminalHandled) return;
  terminalHandled = true;
  if (forceKillTimer) clearTimeout(forceKillTimer);
  if (writableStateMonitor) clearInterval(writableStateMonitor);
  let resourceScopeEntered = false;
  try {
    resourceScopeEntered =
      readFileSync(scopeEntryMarker, "utf8") === "entered\n";
  } catch {
    resourceScopeEntered = false;
  }
  function finishWithCleanup(exitCode) {
    if (!cleanupRuntimeSupport()) {
      console.error("CODE_QUALITY_BOUNDARY_ERROR:runtime:cleanup-failed");
      finishAfterOutputDrains(70);
      return;
    }
    finishAfterOutputDrains(exitCode);
  }
  const finalWritableStateSafetyError = scanBoundaryWritableState();
  if (cancellationSignal) {
    console.error(
      `CODE_QUALITY_BOUNDARY_ERROR:runtime:cancelled-${cancellationSignal}`,
    );
    finishWithCleanup(70);
    return;
  }
  if (outputLimitBreached) {
    console.error("CODE_QUALITY_BOUNDARY_ERROR:safety:output-limit-exceeded");
    finishWithCleanup(77);
    return;
  }
  if (!resourceScopeEntered) {
    console.error(
      "CODE_QUALITY_BOUNDARY_ERROR:runtime:resource-scope-unavailable",
    );
    finishWithCleanup(69);
    return;
  }
  const safetyError = writableStateSafetyError ?? finalWritableStateSafetyError;
  if (safetyError) {
    console.error(`CODE_QUALITY_BOUNDARY_ERROR:safety:${safetyError}`);
    finishWithCleanup(77);
    return;
  }
  if (signal) {
    console.error("CODE_QUALITY_BOUNDARY_ERROR:runtime:terminated-by-signal");
    finishWithCleanup(70);
    return;
  }
  if (code === 124) {
    console.error(
      `CODE_QUALITY_BOUNDARY_ERROR:timeout:wall-${wallTimeoutSeconds}s`,
    );
    finishWithCleanup(124);
    return;
  }
  const workspacePublishError = publishStagedWorkspace();
  if (workspacePublishError) {
    console.error(
      `CODE_QUALITY_BOUNDARY_ERROR:safety:${workspacePublishError}`,
    );
    finishWithCleanup(77);
    return;
  }
  finishWithCleanup(code ?? 70);
});
