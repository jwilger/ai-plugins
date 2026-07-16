#!/usr/bin/env node
import { spawn } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

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
  return {
    bwrap: requireNixTool(
      "bwrap",
      "AI_PLUGINS_BWRAP_BIN",
      /^\/nix\/store\/[0-9a-z]{32}-bubblewrap-[^/]+\/bin\/bwrap$/,
    ),
    executable: canonicalExecutable,
    prlimit: requireNixTool(
      "prlimit",
      "AI_PLUGINS_PRLIMIT_BIN",
      /^\/nix\/store\/[0-9a-z]{32}-util-linux-[^/]+\/bin\/prlimit$/,
    ),
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
    "--dir",
    "/nix",
    "--ro-bind",
    "/nix/store",
    "/nix/store",
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

    function finish(status, signal) {
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
      resolve({
        cleanup:
          trackedProcesses.length > 0
            ? {
                trackedProcesses: trackedProcesses.length,
                survivingProcesses: survivingProcessCount(trackedProcesses),
              }
            : undefined,
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
          finish(null, "SIGKILL");
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
      if (!child.pid) finish(null, null);
    });
    child.on("close", (status, signal) => {
      finish(status, signal);
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

try {
  const target = parseArgs(process.argv.slice(2));
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
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 2;
}
