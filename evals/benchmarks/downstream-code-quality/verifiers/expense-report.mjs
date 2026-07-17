#!/usr/bin/env node
import { spawn } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

import {
  NixStoreClosureError,
  nixStoreMountArgs,
  validatedNixStoreClosure,
} from "./nix-store-closure.mjs";

const sandboxLimits = Object.freeze({
  addressSpaceBytes: 128 * 1024 * 1024,
  coreBytes: 0,
  cpuSeconds: 3,
  fileBytes: 1024 * 1024,
  openFiles: 64,
  processes: 8,
  scratchBytes: 16 * 1024 * 1024,
});
const workspaceMarker = ".git/.ai-plugins-code-quality-workspace";
const workspaceMarkerContents =
  "ai-plugins downstream code-quality workspace\n";
const aggregateScopeChildArgument = "--ai-plugins-aggregate-scope-child";
const aggregateScopeLimits = Object.freeze({
  outputBytes: 256 * 1024,
  timeoutMilliseconds: 150_000,
});
const cleanupObservationLimits = Object.freeze({
  pollMilliseconds: 10,
  timeoutMilliseconds: 1_000,
});

class PublicVerifierOperationalError extends Error {
  constructor(code) {
    super(code);
    this.code = code;
  }
}

function operational(code) {
  throw new PublicVerifierOperationalError(code);
}

function requireDirectory(candidate, description) {
  if (!fs.existsSync(candidate) || !fs.statSync(candidate).isDirectory()) {
    throw new Error(`${description} is missing: ${candidate}`);
  }
  return fs.realpathSync(candidate);
}

function requirePreparedWorkspace(workspace) {
  const gitDirectory = path.join(workspace, ".git");
  const gitStat = fs.lstatSync(gitDirectory, { throwIfNoEntry: false });
  const marker = path.join(workspace, workspaceMarker);
  const markerStat = fs.lstatSync(marker, { throwIfNoEntry: false });
  if (
    !gitStat?.isDirectory() ||
    gitStat.isSymbolicLink() ||
    !markerStat?.isFile() ||
    markerStat.isSymbolicLink() ||
    fs.readFileSync(marker, "utf8") !== workspaceMarkerContents
  ) {
    throw new Error(
      `prepared benchmark workspace marker is missing or invalid: ${marker}`,
    );
  }
}

function requireNixTool(name, environmentVariable, expectedPath) {
  if (process.platform !== "linux") {
    throw new Error("expense-report verification requires Linux sandboxing");
  }
  const configured = process.env[environmentVariable];
  if (!configured || !path.isAbsolute(configured)) {
    throw new Error(
      `${environmentVariable} must be set by the ai-plugins Nix devshell`,
    );
  }
  let executable;
  try {
    fs.accessSync(configured, fs.constants.X_OK);
    executable = fs.realpathSync(configured);
  } catch {
    throw new Error(`${name} flake-selected executable is unavailable`);
  }
  if (!expectedPath.test(executable)) {
    throw new Error(`${name} is not the flake-selected Nix package executable`);
  }
  return executable;
}

function requirePinnedSystemdRun() {
  if (process.platform !== "linux") {
    operational("linux-sandbox-required");
  }
  const configured = process.env.CODE_QUALITY_SYSTEMD_RUN_BIN;
  if (!configured || !path.isAbsolute(configured)) {
    operational("systemd-run-path-missing");
  }
  let executable;
  let metadata;
  let contents;
  try {
    fs.accessSync(configured, fs.constants.X_OK);
    executable = fs.realpathSync(configured);
    metadata = fs.lstatSync(executable);
    contents = fs.readFileSync(executable);
  } catch {
    operational("systemd-run-unavailable");
  }
  if (
    !/^\/nix\/store\/[0-9a-z]{32}-systemd-[^/]+\/bin\/systemd-run$/u.test(
      executable,
    ) ||
    !metadata.isFile() ||
    metadata.isSymbolicLink() ||
    metadata.uid !== 0 ||
    (metadata.mode & 0o022) !== 0
  ) {
    operational("systemd-run-not-flake-selected");
  }
  const expectedSha256 = process.env.CODE_QUALITY_SYSTEMD_RUN_EXPECTED_SHA256;
  if (!/^[0-9a-f]{64}$/u.test(expectedSha256 ?? "")) {
    operational("systemd-run-sha256-missing");
  }
  if (
    crypto.createHash("sha256").update(contents).digest("hex") !==
    expectedSha256
  ) {
    operational("systemd-run-integrity-mismatch");
  }
  return executable;
}

function requireNixEnvTool() {
  for (const directory of (process.env.PATH || "").split(path.delimiter)) {
    if (!path.isAbsolute(directory)) continue;
    const candidate = path.join(directory, "env");
    try {
      fs.accessSync(candidate, fs.constants.X_OK);
      const canonicalDirectory = fs.realpathSync(directory);
      const canonical = fs.realpathSync(candidate);
      if (
        /^\/nix\/store\/[0-9a-z]{32}-[^/]+\/bin$/u.test(canonicalDirectory) &&
        /^\/nix\/store\/[0-9a-z]{32}-[^/]+\/bin\/[^/]+$/u.test(canonical)
      ) {
        return path.join(canonicalDirectory, "env");
      }
    } catch {
      // Continue until the flake-selected immutable env executable is found.
    }
  }
  operational("nix-tool-env-missing");
}

function requireSystemdRuntimeDirectory() {
  const runtimeDirectory = `/run/user/${process.getuid()}`;
  try {
    const metadata = fs.lstatSync(runtimeDirectory);
    if (
      fs.realpathSync(runtimeDirectory) !== runtimeDirectory ||
      !metadata.isDirectory() ||
      metadata.isSymbolicLink() ||
      metadata.uid !== process.getuid() ||
      (metadata.mode & 0o077) !== 0
    ) {
      operational("systemd-runtime-directory-unsafe");
    }
  } catch (error) {
    if (error instanceof PublicVerifierOperationalError) throw error;
    operational("systemd-runtime-directory-unavailable");
  }
  return runtimeDirectory;
}

function readCgroupValue(cgroupRoot, name) {
  try {
    const value = fs.readFileSync(path.join(cgroupRoot, name), "utf8").trim();
    if (!value || value.length > 128) operational("resource-scope-invalid");
    return value;
  } catch (error) {
    if (error instanceof PublicVerifierOperationalError) throw error;
    operational("resource-scope-invalid");
  }
}

function setAndRequireAggregateOomGroup(cgroupRoot) {
  const oomGroup = path.join(cgroupRoot, "memory.oom.group");
  try {
    const metadata = fs.lstatSync(oomGroup);
    if (!metadata.isFile() || metadata.isSymbolicLink()) {
      operational("resource-scope-invalid");
    }
    const descriptor = fs.openSync(
      oomGroup,
      fs.constants.O_WRONLY | fs.constants.O_NOFOLLOW,
    );
    try {
      fs.writeFileSync(descriptor, "1\n", { encoding: "utf8" });
    } finally {
      fs.closeSync(descriptor);
    }
  } catch (error) {
    if (error instanceof PublicVerifierOperationalError) throw error;
    operational("resource-scope-invalid");
  }
  if (readCgroupValue(cgroupRoot, "memory.oom.group") !== "1") {
    operational("resource-scope-invalid");
  }
}

function requireAggregateScope(unit) {
  if (
    !/^ai-plugins-code-quality-public-verifier-[1-9][0-9]*-[0-9a-f]{16}$/u.test(
      unit,
    )
  ) {
    operational("resource-scope-invalid");
  }
  let cgroupPath;
  try {
    const unified = fs
      .readFileSync("/proc/self/cgroup", "utf8")
      .trim()
      .split("\n")
      .find((line) => line.startsWith("0::"));
    cgroupPath = unified?.slice(3);
  } catch {
    operational("resource-scope-invalid");
  }
  if (
    !cgroupPath ||
    !cgroupPath.split("/").includes(`${unit}.scope`) ||
    cgroupPath.includes("\0")
  ) {
    operational("resource-scope-invalid");
  }
  const cgroupRoot = path.join("/sys/fs/cgroup", cgroupPath);
  try {
    const canonical = fs.realpathSync(cgroupRoot);
    if (
      canonical !== path.resolve(cgroupRoot) ||
      !canonical.startsWith("/sys/fs/cgroup/") ||
      !fs.lstatSync(canonical).isDirectory()
    ) {
      operational("resource-scope-invalid");
    }
  } catch (error) {
    if (error instanceof PublicVerifierOperationalError) throw error;
    operational("resource-scope-invalid");
  }
  if (
    readCgroupValue(cgroupRoot, "memory.max") !== "8589934592" ||
    readCgroupValue(cgroupRoot, "memory.swap.max") !== "0" ||
    readCgroupValue(cgroupRoot, "pids.max") !== "512"
  ) {
    operational("resource-scope-invalid");
  }
  const [quota, period, ...extra] = readCgroupValue(
    cgroupRoot,
    "cpu.max",
  ).split(" ");
  if (
    extra.length !== 0 ||
    !/^[1-9][0-9]*$/u.test(quota) ||
    !/^[1-9][0-9]*$/u.test(period) ||
    Number(quota) !== 4 * Number(period)
  ) {
    operational("resource-scope-invalid");
  }
  setAndRequireAggregateOomGroup(cgroupRoot);
}

function publicVerifierEnvironment() {
  const environment = {
    AI_PLUGINS_BWRAP_BIN: process.env.AI_PLUGINS_BWRAP_BIN ?? "",
    AI_PLUGINS_PRLIMIT_BIN: process.env.AI_PLUGINS_PRLIMIT_BIN ?? "",
    CODE_QUALITY_NIX_STORE_CLOSURE:
      process.env.CODE_QUALITY_NIX_STORE_CLOSURE ?? "",
    CODE_QUALITY_NIX_STORE_CLOSURE_EXPECTED_SHA256:
      process.env.CODE_QUALITY_NIX_STORE_CLOSURE_EXPECTED_SHA256 ?? "",
    LANG: "C.UTF-8",
    LC_ALL: "C.UTF-8",
  };
  return environment;
}

function requireExactPublicVerifierEnvironment() {
  const expected = publicVerifierEnvironment();
  if (
    JSON.stringify(Object.keys(process.env).sort()) !==
      JSON.stringify(Object.keys(expected).sort()) ||
    Object.keys(expected).some((name) => process.env[name] !== expected[name])
  ) {
    operational("resource-scope-environment-invalid");
  }
}

function writeOperationalError(code) {
  process.stderr.write(`expense-report:operational-failure:${code}\n`);
  process.exitCode = 2;
}

function parseArgs(argv) {
  if (argv.length !== 4 || argv[0] !== "--workspace" || argv[2] !== "--bin") {
    throw new Error(
      "usage: expense-report.mjs --workspace <directory> --bin <executable>",
    );
  }
  const workspace = requireDirectory(
    path.resolve(argv[1]),
    "expense-report workspace",
  );
  requirePreparedWorkspace(workspace);
  const executable = path.resolve(argv[3]);
  if (!fs.existsSync(executable) || !fs.statSync(executable).isFile()) {
    throw new Error(`expense-report executable is missing: ${executable}`);
  }
  try {
    fs.accessSync(executable, fs.constants.X_OK);
  } catch {
    throw new Error(`expense-report file is not executable: ${executable}`);
  }
  const canonicalExecutable = fs.realpathSync(executable);
  const relativeExecutable = path.relative(workspace, canonicalExecutable);
  if (
    relativeExecutable === ".." ||
    relativeExecutable.startsWith(`..${path.sep}`) ||
    path.isAbsolute(relativeExecutable)
  ) {
    throw new Error("expense-report executable must be inside its workspace");
  }
  const bwrap = requireNixTool(
    "bwrap",
    "AI_PLUGINS_BWRAP_BIN",
    /^\/nix\/store\/[0-9a-z]{32}-bubblewrap-[^/]+\/bin\/bwrap$/,
  );
  const prlimit = requireNixTool(
    "prlimit",
    "AI_PLUGINS_PRLIMIT_BIN",
    /^\/nix\/store\/[0-9a-z]{32}-util-linux-[^/]+\/bin\/prlimit$/,
  );
  const nixStoreClosure = validatedNixStoreClosure({
    expectedSha256: process.env.CODE_QUALITY_NIX_STORE_CLOSURE_EXPECTED_SHA256,
    manifest: process.env.CODE_QUALITY_NIX_STORE_CLOSURE,
    requiredPaths: [prlimit],
  });
  return {
    bwrap,
    executable: canonicalExecutable,
    nixStoreClosure,
    prlimit,
    relativeExecutable,
    workspace,
  };
}

const scenarios = [
  {
    id: "totals-duplicate-order",
    args: ["totals"],
    input: "food,100\ntravel,200\nfood,50\n",
    status: 0,
    stdout: "food,150\ntravel,200\n",
  },
  {
    id: "totals-orders-by-category-not-amount",
    args: ["totals"],
    input: "food,300\ntravel,100\n",
    status: 0,
    stdout: "food,300\ntravel,100\n",
  },
  {
    id: "totals-inclusive-minimum",
    args: ["totals", "--minimum-cents", "150"],
    input: "travel,200\nfood,100\nfood,50\n",
    status: 0,
    stdout: "food,150\ntravel,200\n",
  },
  {
    id: "totals-filters-below-minimum",
    args: ["totals", "--minimum-cents", "151"],
    input: "travel,200\nfood,100\nfood,50\n",
    status: 0,
    stdout: "travel,200\n",
  },
  {
    id: "totals-smallest-valid-minimum",
    args: ["totals", "--minimum-cents", "1"],
    input: "food,1\n",
    status: 0,
    stdout: "food,1\n",
  },
  {
    id: "totals-minimum-larger-than-u8",
    args: ["totals", "--minimum-cents", "256"],
    input: "food,256\n",
    status: 0,
    stdout: "food,256\n",
  },
  {
    id: "totals-minimum-larger-than-u32",
    args: ["totals", "--minimum-cents", "4294967296"],
    input: "food,4294967296\n",
    status: 0,
    stdout: "food,4294967296\n",
  },
  {
    id: "totals-maximum-minimum",
    args: ["totals", "--minimum-cents", "18446744073709551615"],
    input: "food,18446744073709551615\n",
    status: 0,
    stdout: "food,18446744073709551615\n",
  },
  {
    id: "totals-empty-input",
    args: ["totals"],
    input: "",
    status: 0,
    stdout: "",
  },
  {
    id: "preserves-validate-command",
    args: ["validate"],
    input: "food,125\ntravel,400\n",
    status: 0,
    stdout: "valid,2\n",
  },
  {
    id: "rejects-missing-minimum",
    args: ["totals", "--minimum-cents"],
    input: "food,1\n",
    status: "failure",
    stdout: "",
  },
  {
    id: "rejects-zero-minimum",
    args: ["totals", "--minimum-cents", "0"],
    input: "food,1\n",
    status: "failure",
    stdout: "",
  },
  {
    id: "rejects-negative-minimum",
    args: ["totals", "--minimum-cents", "-1"],
    input: "food,1\n",
    status: "failure",
    stdout: "",
  },
  {
    id: "rejects-nonnumeric-minimum",
    args: ["totals", "--minimum-cents", "many"],
    input: "food,1\n",
    status: "failure",
    stdout: "",
  },
  {
    id: "rejects-overflowing-minimum",
    args: ["totals", "--minimum-cents", "18446744073709551616"],
    input: "food,1\n",
    status: "failure",
    stdout: "",
  },
  {
    id: "rejects-aggregate-overflow",
    args: ["totals"],
    input: "food,18446744073709551615\nfood,1\n",
    status: "failure",
    stdout: "",
  },
  {
    id: "rejects-late-malformed-record-without-partial-output",
    args: ["totals"],
    input: "food,1\nmalformed\n",
    status: "failure",
    stdout: "",
  },
];

function observedStatus(result) {
  if (result.error) return `error:${result.error}`;
  if (result.signal) return `signal:${result.signal}`;
  return result.status;
}

function outputEvidence(value) {
  return {
    bytes: Buffer.byteLength(value),
    sha256: crypto.createHash("sha256").update(value).digest("hex"),
  };
}

function sandboxArgs(target, args) {
  const sandboxExecutable = "/workspace/expense-report";
  return [
    "--unshare-all",
    "--unshare-user",
    "--disable-userns",
    "--assert-userns-disabled",
    "--cap-drop",
    "ALL",
    "--new-session",
    "--die-with-parent",
    "--hostname",
    "ai-plugins-benchmark",
    "--json-status-fd",
    "3",
    "--clearenv",
    "--setenv",
    "HOME",
    "/tmp/home",
    "--setenv",
    "TMPDIR",
    "/tmp",
    "--setenv",
    "PATH",
    "/workspace",
    "--setenv",
    "LANG",
    "C.UTF-8",
    "--setenv",
    "LC_ALL",
    "C.UTF-8",
    "--size",
    String(sandboxLimits.scratchBytes),
    "--tmpfs",
    "/",
    ...nixStoreMountArgs(target.nixStoreClosure),
    "--dir",
    "/workspace",
    "--ro-bind",
    target.executable,
    sandboxExecutable,
    "--proc",
    "/proc",
    "--dir",
    "/dev",
    "--dev-bind",
    "/dev/null",
    "/dev/null",
    "--dev-bind",
    "/dev/zero",
    "/dev/zero",
    "--dev-bind",
    "/dev/random",
    "/dev/random",
    "--dev-bind",
    "/dev/urandom",
    "/dev/urandom",
    "--symlink",
    "/proc/self/fd",
    "/dev/fd",
    "--symlink",
    "/proc/self/fd/0",
    "/dev/stdin",
    "--symlink",
    "/proc/self/fd/1",
    "/dev/stdout",
    "--symlink",
    "/proc/self/fd/2",
    "/dev/stderr",
    "--size",
    String(sandboxLimits.scratchBytes),
    "--tmpfs",
    "/tmp",
    "--dir",
    "/tmp/home",
    "--remount-ro",
    "/",
    "--chdir",
    "/workspace",
    target.prlimit,
    `--cpu=${sandboxLimits.cpuSeconds}:${sandboxLimits.cpuSeconds}`,
    `--as=${sandboxLimits.addressSpaceBytes}:${sandboxLimits.addressSpaceBytes}`,
    `--nproc=${sandboxLimits.processes}:${sandboxLimits.processes}`,
    `--nofile=${sandboxLimits.openFiles}:${sandboxLimits.openFiles}`,
    `--fsize=${sandboxLimits.fileBytes}:${sandboxLimits.fileBytes}`,
    `--core=${sandboxLimits.coreBytes}:${sandboxLimits.coreBytes}`,
    "--",
    sandboxExecutable,
    ...args,
  ];
}

function parseSandboxStatus(serialized) {
  const events = serialized
    .trim()
    .split("\n")
    .filter(Boolean)
    .map((line) => JSON.parse(line));
  const childStarted = events.some((event) =>
    Object.hasOwn(event, "child-pid"),
  );
  const exitEvent = events.findLast((event) =>
    Object.hasOwn(event, "exit-code"),
  );
  return {
    childStarted,
    exitCode: exitEvent?.["exit-code"],
  };
}

function processIdentity(pid) {
  try {
    const stat = fs.readFileSync(`/proc/${pid}/stat`, "utf8");
    const commandEnd = stat.lastIndexOf(")");
    if (commandEnd === -1) return undefined;
    const fields = stat
      .slice(commandEnd + 2)
      .trim()
      .split(/\s+/);
    return {
      pid,
      startTime: fields[19],
      state: fields[0],
    };
  } catch {
    return undefined;
  }
}

function processTree(rootPid) {
  const identities = [];
  const pending = [rootPid];
  const visited = new Set();
  while (pending.length > 0) {
    const pid = pending.shift();
    if (!Number.isSafeInteger(pid) || pid <= 0 || visited.has(pid)) continue;
    visited.add(pid);
    const identity = processIdentity(pid);
    if (!identity) continue;
    identities.push(identity);
    try {
      const children = fs
        .readFileSync(`/proc/${pid}/task/${pid}/children`, "utf8")
        .trim()
        .split(/\s+/)
        .filter(Boolean)
        .map(Number);
      pending.push(...children);
    } catch {
      // A process that exits while the snapshot is taken is already harmless.
    }
  }
  return identities;
}

function survivingProcessCount(identities) {
  return identities.filter((identity) => {
    const current = processIdentity(identity.pid);
    return (
      current &&
      current.startTime === identity.startTime &&
      current.state !== "Z"
    );
  }).length;
}

async function observeSurvivingProcesses(identities) {
  const deadline = Date.now() + cleanupObservationLimits.timeoutMilliseconds;
  let survivingProcesses = survivingProcessCount(identities);
  while (survivingProcesses > 0 && Date.now() < deadline) {
    await new Promise((resolve) =>
      setTimeout(resolve, cleanupObservationLimits.pollMilliseconds),
    );
    survivingProcesses = survivingProcessCount(identities);
  }
  return survivingProcesses;
}

function runProcess(target, args, input) {
  return new Promise((resolve) => {
    const outputLimit = 64 * 1024;
    const sandboxStatusLimit = 16 * 1024;
    const child = spawn(target.bwrap, sandboxArgs(target, args), {
      detached: process.platform !== "win32",
      stdio: ["pipe", "pipe", "pipe", "pipe"],
      env: {},
    });
    const stdout = [];
    const stderr = [];
    const sandboxStatus = [];
    let outputBytes = 0;
    let sandboxStatusBytes = 0;
    let processError;
    let killedForLimit = false;
    let settled = false;
    let forcedResolutionTimer;
    let sandboxChildPid;
    let sandboxStatusBuffer = "";
    let trackedProcesses = [];

    const timer = setTimeout(() => {
      terminate("TIMEOUT");
    }, 5_000);

    async function finish(status, signal) {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      if (forcedResolutionTimer) clearTimeout(forcedResolutionTimer);
      let candidateStatus = status;
      if (!processError) {
        try {
          const parsed = parseSandboxStatus(sandboxStatus.join(""));
          if (!parsed.childStarted || parsed.exitCode === undefined) {
            processError = "SANDBOX_SETUP";
          } else if (parsed.exitCode >= 128) {
            processError = `SANDBOX_SIGNAL:${parsed.exitCode}`;
          } else if (status !== parsed.exitCode) {
            processError = "SANDBOX_STATUS_MISMATCH";
          } else {
            candidateStatus = parsed.exitCode;
          }
        } catch {
          processError = "SANDBOX_STATUS";
        }
      }
      const cleanup =
        trackedProcesses.length > 0
          ? {
              trackedProcesses: trackedProcesses.length,
              survivingProcesses:
                await observeSurvivingProcesses(trackedProcesses),
            }
          : undefined;
      resolve({
        cleanup,
        status: candidateStatus,
        signal,
        error: processError,
        stdout: stdout.join(""),
        stderr: stderr.join(""),
      });
    }

    function terminate(reason) {
      if (!processError) processError = reason;
      if (trackedProcesses.length === 0 && sandboxChildPid) {
        trackedProcesses = processTree(sandboxChildPid);
      }
      try {
        if (process.platform !== "win32" && child.pid) {
          process.kill(-child.pid, "SIGKILL");
        } else {
          child.kill("SIGKILL");
        }
      } catch {
        child.kill("SIGKILL");
      }
      if (!forcedResolutionTimer) {
        forcedResolutionTimer = setTimeout(() => {
          child.stdin.destroy();
          child.stdout.destroy();
          child.stderr.destroy();
          child.stdio[3].destroy();
          child.unref();
          void finish(null, "SIGKILL");
        }, 250);
      }
    }

    function collect(chunks, chunk) {
      outputBytes += Buffer.byteLength(chunk);
      if (outputBytes > outputLimit) {
        if (!killedForLimit) {
          killedForLimit = true;
          terminate("OUTPUT_LIMIT");
        }
        return;
      }
      chunks.push(chunk);
    }

    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdio[3].setEncoding("utf8");
    child.stdout.on("data", (chunk) => collect(stdout, chunk));
    child.stderr.on("data", (chunk) => collect(stderr, chunk));
    child.stdio[3].on("data", (chunk) => {
      sandboxStatusBytes += Buffer.byteLength(chunk);
      if (sandboxStatusBytes > sandboxStatusLimit) {
        terminate("SANDBOX_STATUS_LIMIT");
        return;
      }
      sandboxStatus.push(chunk);
      sandboxStatusBuffer += chunk;
      for (;;) {
        const newline = sandboxStatusBuffer.indexOf("\n");
        if (newline === -1) break;
        const line = sandboxStatusBuffer.slice(0, newline);
        sandboxStatusBuffer = sandboxStatusBuffer.slice(newline + 1);
        try {
          const event = JSON.parse(line);
          if (Number.isSafeInteger(event["child-pid"])) {
            sandboxChildPid = event["child-pid"];
          }
        } catch {
          terminate("SANDBOX_STATUS");
        }
      }
    });
    child.on("error", (error) => {
      processError = error.code || error.message;
      if (!child.pid) void finish(null, null);
    });
    child.on("close", (status, signal) => {
      void finish(status, signal);
    });
    child.stdin.on("error", () => {});
    child.stdin.end(input);
  });
}

async function runScenario(target, scenario) {
  const result = await runProcess(target, scenario.args, scenario.input);
  const statusMatches =
    scenario.status === "failure"
      ? typeof result.status === "number" && result.status !== 0
      : result.status === scenario.status;
  const stdout = result.stdout;
  const stderrMatches = scenario.status !== 0 || result.stderr === "";
  const pass =
    !result.error &&
    statusMatches &&
    stdout === scenario.stdout &&
    stderrMatches;
  return {
    id: scenario.id,
    pass,
    expected: {
      status: scenario.status,
      stdout: scenario.stdout,
    },
    observed: {
      status: observedStatus(result),
      stdout: outputEvidence(stdout),
      stderr: outputEvidence(result.stderr),
      cleanup: result.cleanup,
    },
  };
}

async function executeVerifier(argv) {
  const target = parseArgs(argv);
  const checks = [];
  for (const scenario of scenarios) {
    const check = await runScenario(target, scenario);
    checks.push(check);
    if (String(check.observed.status).startsWith("error:")) break;
  }
  const report = {
    schemaVersion: 1,
    verifier: "expense-report-public-cli",
    isolation: {
      filesystem: "read-only-executable-and-nix-runtime",
      network: "disabled",
      limits: sandboxLimits,
    },
    pass: checks.every((check) => check.pass),
    checks,
  };
  process.stdout.write(`${JSON.stringify(report)}\n`);
  process.exitCode = report.pass ? 0 : 1;
}

async function executeInAggregateScope(argv) {
  const systemdRun = requirePinnedSystemdRun();
  const systemdRuntimeDirectory = requireSystemdRuntimeDirectory();
  const envTool = requireNixEnvTool();
  const childEnvironment = publicVerifierEnvironment();
  const unit = `ai-plugins-code-quality-public-verifier-${process.pid}-${crypto
    .randomBytes(8)
    .toString("hex")}`;
  const child = spawn(
    systemdRun,
    [
      "--user",
      "--scope",
      "--quiet",
      "--collect",
      "--expand-environment=false",
      `--unit=${unit}`,
      "--property=MemoryMax=8589934592",
      "--property=MemorySwapMax=0",
      "--property=TasksMax=512",
      "--property=CPUQuota=400%",
      "--property=KillMode=control-group",
      "--",
      envTool,
      "-i",
      ...Object.entries(childEnvironment).map(
        ([name, value]) => `${name}=${value}`,
      ),
      process.execPath,
      import.meta.filename,
      aggregateScopeChildArgument,
      unit,
      ...argv,
    ],
    {
      detached: true,
      env: {
        LANG: "C.UTF-8",
        LC_ALL: "C.UTF-8",
        PATH: path.dirname(systemdRun),
        XDG_RUNTIME_DIR: systemdRuntimeDirectory,
      },
      stdio: ["ignore", "pipe", "pipe"],
    },
  );

  await new Promise((resolve) => {
    const stdout = [];
    const stderr = [];
    let outputBytes = 0;
    let terminationReason;
    let forceKillTimer;

    function signalChild(signal) {
      try {
        process.kill(-child.pid, signal);
      } catch {
        try {
          child.kill(signal);
        } catch {
          // The aggregate scope has already exited.
        }
      }
    }

    function terminate(reason) {
      if (terminationReason) return;
      terminationReason = reason;
      signalChild("SIGTERM");
      forceKillTimer = setTimeout(() => signalChild("SIGKILL"), 5_000);
      forceKillTimer.unref();
    }

    const signalHandlers = new Map();
    for (const signal of ["SIGHUP", "SIGINT", "SIGTERM"]) {
      const handler = () => terminate("cancelled");
      signalHandlers.set(signal, handler);
      process.once(signal, handler);
    }
    const timeout = setTimeout(
      () => terminate("timeout"),
      aggregateScopeLimits.timeoutMilliseconds,
    );
    timeout.unref();

    function collect(chunks, chunk) {
      outputBytes += Buffer.byteLength(chunk);
      if (outputBytes > aggregateScopeLimits.outputBytes) {
        terminate("output-limit");
        return;
      }
      chunks.push(chunk);
    }
    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", (chunk) => collect(stdout, chunk));
    child.stderr.on("data", (chunk) => collect(stderr, chunk));
    child.on("error", () => terminate("spawn-failed"));
    child.on("close", (status, signal) => {
      clearTimeout(timeout);
      if (forceKillTimer) clearTimeout(forceKillTimer);
      for (const [name, handler] of signalHandlers) {
        process.removeListener(name, handler);
      }
      if (terminationReason) {
        writeOperationalError(`resource-scope-${terminationReason}`);
      } else if (
        [0, 1].includes(status) &&
        signal === null &&
        stderr.join("") === ""
      ) {
        process.stdout.write(stdout.join(""));
        process.exitCode = status;
      } else if (
        status === 2 &&
        signal === null &&
        stdout.join("") === "" &&
        Buffer.byteLength(stderr.join("")) <= aggregateScopeLimits.outputBytes
      ) {
        process.stderr.write(stderr.join(""));
        process.exitCode = 2;
      } else {
        writeOperationalError("resource-scope-failed");
      }
      resolve();
    });
  });
}

const mainArguments = process.argv.slice(2);
if (mainArguments[0] === aggregateScopeChildArgument) {
  try {
    if (mainArguments.length < 3) operational("resource-scope-invalid");
    requireExactPublicVerifierEnvironment();
    requireAggregateScope(mainArguments[1]);
    await executeVerifier(mainArguments.slice(2));
  } catch (error) {
    if (error instanceof NixStoreClosureError) {
      writeOperationalError(error.code);
    } else if (error instanceof PublicVerifierOperationalError) {
      writeOperationalError(error.code);
    } else {
      console.error(error instanceof Error ? error.message : String(error));
      process.exitCode = 2;
    }
  }
} else {
  try {
    await executeInAggregateScope(mainArguments);
  } catch (error) {
    if (error instanceof PublicVerifierOperationalError) {
      writeOperationalError(error.code);
    } else {
      writeOperationalError("unexpected");
    }
  }
}
