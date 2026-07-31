#!/usr/bin/env node
import { createHash, randomUUID } from "node:crypto";
import { createReadStream, createWriteStream } from "node:fs";
import { constants } from "node:fs";
import {
  access,
  chmod,
  copyFile,
  link,
  lstat,
  mkdir,
  mkdtemp,
  readFile,
  realpath,
  rename,
  rm,
} from "node:fs/promises";
import { get as httpsGet } from "node:https";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import { fileURLToPath, pathToFileURL } from "node:url";
import { execFile, spawn } from "node:child_process";
import { promisify } from "node:util";
import { pipeline } from "node:stream/promises";
import { Transform } from "node:stream";

const execFileAsync = promisify(execFile);
const packageBin = path.dirname(fileURLToPath(import.meta.url));
const defaultManifest = path.join(packageBin, "tool-releases.json");
const DEFAULT_DOWNLOAD_TIMEOUT_MS = 120_000;
const DEFAULT_MAX_ARCHIVE_BYTES = 256 * 1024 * 1024;
const DEFAULT_VERSION_TIMEOUT_MS = 5_000;
const TOOL_NAME = /^[a-z0-9][a-z0-9._-]*$/;
const SEMVER = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;

export class ToolInstallError extends Error {
  constructor(code, details = "") {
    super(`${code}${details ? ` ${details}` : ""}`);
    this.code = code;
  }
}

function positiveInteger(environment, name, fallback) {
  const raw = environment[name];
  if (raw === undefined) return fallback;
  const parsed = Number(raw);
  if (!Number.isSafeInteger(parsed) || parsed <= 0)
    throw new ToolInstallError(
      "development_system.tool_limit_invalid",
      `name=${name}`,
    );
  return parsed;
}

function targetKey(environment = process.env) {
  const platform =
    environment.DEVELOPMENT_SYSTEM_TOOL_PLATFORM ?? process.platform;
  const architecture = environment.DEVELOPMENT_SYSTEM_TOOL_ARCH ?? process.arch;
  const operatingSystem = { linux: "linux", darwin: "darwin" }[platform];
  const cpu = { x64: "x86_64", arm64: "aarch64" }[architecture];
  if (!operatingSystem || !cpu)
    throw new ToolInstallError(
      "development_system.tool_platform_unsupported",
      `os=${platform} arch=${architecture}`,
    );
  return `${cpu}-${operatingSystem}`;
}

export function userGlobalBin(environment = process.env) {
  if (environment.DEVELOPMENT_SYSTEM_USER_BIN)
    return path.resolve(environment.DEVELOPMENT_SYSTEM_USER_BIN);
  const home = environment.HOME ? path.resolve(environment.HOME) : os.homedir();
  return path.join(home, ".local", "bin");
}

async function readManifest(environment = process.env) {
  const source = environment.DEVELOPMENT_SYSTEM_TOOL_RELEASES
    ? path.resolve(environment.DEVELOPMENT_SYSTEM_TOOL_RELEASES)
    : defaultManifest;
  let parsed;
  try {
    parsed = JSON.parse(await readFile(source, "utf8"));
  } catch (error) {
    throw new ToolInstallError(
      "development_system.tool_manifest_invalid",
      `detail=${JSON.stringify(error instanceof Error ? error.message : String(error))}`,
    );
  }
  if (
    parsed?.schemaVersion !== 2 ||
    !parsed.tools ||
    typeof parsed.tools !== "object" ||
    Array.isArray(parsed.tools)
  )
    throw new ToolInstallError("development_system.tool_manifest_invalid");
  for (const [name, definition] of Object.entries(parsed.tools)) {
    if (
      !TOOL_NAME.test(name) ||
      !SEMVER.test(definition?.version ?? "") ||
      !Array.isArray(definition?.requiredFor) ||
      !definition.requiredFor.every(
        (feature) => typeof feature === "string" && feature.length > 0,
      ) ||
      !Array.isArray(definition?.versionCommand) ||
      definition.versionCommand.length === 0 ||
      !definition.versionCommand.every(
        (argument) => typeof argument === "string" && argument.length > 0,
      ) ||
      typeof definition?.versionPattern !== "string" ||
      !definition.releases ||
      typeof definition.releases !== "object"
    )
      throw new ToolInstallError(
        "development_system.tool_manifest_invalid",
        `tool=${name}`,
      );
    let expression;
    try {
      expression = new RegExp(definition.versionPattern);
    } catch {
      throw new ToolInstallError(
        "development_system.tool_manifest_invalid",
        `tool=${name} field=versionPattern`,
      );
    }
    if (expression.exec("")?.[1] !== undefined)
      throw new ToolInstallError(
        "development_system.tool_manifest_invalid",
        `tool=${name} field=versionPattern`,
      );
  }
  return parsed;
}

export async function requiredToolVersions(environment = process.env) {
  const releases = await readManifest(environment);
  return Object.fromEntries(
    Object.entries(releases.tools).map(([name, definition]) => [
      name,
      definition.version,
    ]),
  );
}

function semver(value) {
  const match = value.match(SEMVER);
  if (!match) return null;
  return [Number(match[1]), Number(match[2]), Number(match[3])];
}

function compareVersions(left, right) {
  const leftParts = semver(left);
  const rightParts = semver(right);
  if (!leftParts || !rightParts)
    throw new ToolInstallError("development_system.tool_manifest_invalid");
  for (let index = 0; index < 3; index += 1) {
    if (leftParts[index] < rightParts[index]) return -1;
    if (leftParts[index] > rightParts[index]) return 1;
  }
  return 0;
}

async function executable(file) {
  try {
    await access(file, constants.X_OK);
    return true;
  } catch {
    return false;
  }
}

async function executableOnPath(name, environment) {
  for (const directory of (environment.PATH ?? "").split(path.delimiter)) {
    const candidate = path.resolve(directory || ".", name);
    if (!(await executable(candidate))) continue;
    const canonical = await realpath(candidate);
    const packageLauncher = path.join(packageBin, name);
    if (canonical === packageLauncher) continue;
    return canonical;
  }
  return null;
}

async function observedVersion(executablePath, definition, environment) {
  try {
    const { stdout } = await execFileAsync(
      executablePath,
      definition.versionCommand,
      {
        timeout: positiveInteger(
          environment,
          "DEVELOPMENT_SYSTEM_TOOL_VERSION_TIMEOUT_MS",
          DEFAULT_VERSION_TIMEOUT_MS,
        ),
        maxBuffer: 256 * 1024,
        env: environment,
      },
    );
    const match = new RegExp(definition.versionPattern).exec(stdout);
    return match?.[1] && SEMVER.test(match[1]) ? match[1] : null;
  } catch {
    return null;
  }
}

function pathIncludes(directory, environment) {
  const expected = path.resolve(directory);
  return (environment.PATH ?? "")
    .split(path.delimiter)
    .some((entry) => path.resolve(entry || ".") === expected);
}

export async function inspectToolPolicy(
  environment = process.env,
  requiredFeatures = null,
) {
  const releases = await readManifest(environment);
  const destination = userGlobalBin(environment);
  const selected = Object.entries(releases.tools).filter(
    ([, definition]) =>
      !requiredFeatures ||
      definition.requiredFor.some((feature) =>
        requiredFeatures.includes(feature),
      ),
  );
  const tools = [];
  for (const [name, definition] of selected) {
    const ambient = await executableOnPath(name, environment);
    const userExecutable = path.join(destination, name);
    const candidates = [];
    if (ambient) candidates.push({ executable: ambient, source: "path" });
    if (
      (await executable(userExecutable)) &&
      (!ambient || (await realpath(userExecutable)) !== ambient)
    )
      candidates.push({
        executable: await realpath(userExecutable),
        source: "user-global",
      });
    let observed = null;
    let compatible = null;
    for (const candidate of candidates) {
      const version = await observedVersion(
        candidate.executable,
        definition,
        environment,
      );
      const record = { ...candidate, version };
      if (!observed) observed = record;
      if (version && compareVersions(version, definition.version) >= 0) {
        compatible = record;
        break;
      }
    }
    const chosen = compatible ?? observed;
    tools.push(
      Object.freeze({
        name,
        requiredFor: Object.freeze([...definition.requiredFor]),
        targetVersion: definition.version,
        status: compatible
          ? "compatible"
          : chosen?.version
            ? "outdated"
            : chosen
              ? "invalid"
              : "missing",
        currentVersion: chosen?.version ?? null,
        executable: compatible?.executable ?? null,
        source: compatible?.source ?? null,
      }),
    );
  }
  const inheritedPathIncludesDestination = pathIncludes(
    destination,
    environment,
  );
  return Object.freeze({
    schemaVersion: 1,
    destination,
    installationScope: "user-global",
    requiresSudo: false,
    inheritedPathIncludesDestination,
    usesUserGlobal: tools.some((tool) => tool.source === "user-global"),
    pathAction: inheritedPathIncludesDestination
      ? null
      : `Add ${destination} to PATH (for example: export PATH=\"$HOME/.local/bin:$PATH\") and restart the shell.`,
    tools: Object.freeze(tools),
  });
}

async function systemExecutable(name, environment) {
  const executablePath = await executableOnPath(name, environment);
  if (executablePath) return executablePath;
  throw new ToolInstallError(
    "development_system.tool_extract_tool_missing",
    `tool=${name}`,
  );
}

function request(url, signal, redirects = 0) {
  return new Promise((resolve, reject) => {
    if (url.protocol !== "https:") {
      reject(
        new ToolInstallError(
          "development_system.tool_url_unsupported",
          `protocol=${url.protocol}`,
        ),
      );
      return;
    }
    const operation = httpsGet(url, { signal }, (response) => {
      if (
        response.statusCode >= 300 &&
        response.statusCode < 400 &&
        response.headers.location
      ) {
        response.resume();
        if (redirects >= 5) {
          reject(
            new ToolInstallError("development_system.tool_redirect_limit"),
          );
          return;
        }
        const redirected = new URL(response.headers.location, url);
        if (redirected.protocol !== "https:") {
          reject(
            new ToolInstallError(
              "development_system.tool_url_unsupported",
              `protocol=${redirected.protocol}`,
            ),
          );
          return;
        }
        resolve(request(redirected, signal, redirects + 1));
        return;
      }
      if (response.statusCode !== 200) {
        response.resume();
        reject(
          new ToolInstallError(
            "development_system.tool_download_failed",
            `status=${response.statusCode}`,
          ),
        );
        return;
      }
      resolve(response);
    });
    operation.on("error", reject);
  });
}

async function download(url, target, expectedHash, environment) {
  const timeoutMs = positiveInteger(
    environment,
    "DEVELOPMENT_SYSTEM_TOOL_DOWNLOAD_TIMEOUT_MS",
    DEFAULT_DOWNLOAD_TIMEOUT_MS,
  );
  const maxBytes = positiveInteger(
    environment,
    "DEVELOPMENT_SYSTEM_TOOL_MAX_ARCHIVE_BYTES",
    DEFAULT_MAX_ARCHIVE_BYTES,
  );
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    const sourceUrl = new URL(url);
    const input =
      sourceUrl.protocol === "https:"
        ? await request(sourceUrl, controller.signal)
        : sourceUrl.protocol === "file:" &&
            environment.DEVELOPMENT_SYSTEM_TOOL_ALLOW_FILE_URLS === "1"
          ? createReadStream(fileURLToPath(sourceUrl))
          : (() => {
              throw new ToolInstallError(
                "development_system.tool_url_unsupported",
                `protocol=${sourceUrl.protocol}`,
              );
            })();
    const contentLength = Number(input.headers?.["content-length"]);
    if (Number.isFinite(contentLength) && contentLength > maxBytes) {
      input.destroy();
      throw new ToolInstallError(
        "development_system.tool_download_size_exceeded",
        `maximum=${maxBytes} observed=${contentLength}`,
      );
    }
    const digest = createHash("sha256");
    let observedBytes = 0;
    const observe = new Transform({
      transform(chunk, _encoding, callback) {
        observedBytes += chunk.length;
        if (observedBytes > maxBytes) {
          callback(
            new ToolInstallError(
              "development_system.tool_download_size_exceeded",
              `maximum=${maxBytes} observed=${observedBytes}`,
            ),
          );
          return;
        }
        digest.update(chunk);
        callback(null, chunk);
      },
    });
    await pipeline(
      input,
      observe,
      createWriteStream(target, { flags: "wx", mode: 0o600 }),
      { signal: controller.signal },
    );
    const actualHash = digest.digest("hex");
    if (actualHash !== expectedHash)
      throw new ToolInstallError(
        "development_system.tool_checksum_failed",
        `expected=${expectedHash} actual=${actualHash}`,
      );
  } catch (error) {
    if (controller.signal.aborted)
      throw new ToolInstallError(
        "development_system.tool_download_timeout",
        `milliseconds=${timeoutMs}`,
      );
    if (error instanceof ToolInstallError) throw error;
    throw new ToolInstallError(
      "development_system.tool_download_failed",
      `detail=${JSON.stringify(error instanceof Error ? error.message : String(error))}`,
    );
  } finally {
    clearTimeout(timer);
  }
}

async function runTar(tarExecutable, arguments_, environment) {
  const child = spawn(tarExecutable, arguments_, {
    stdio: ["ignore", "pipe", "pipe"],
    env: environment,
  });
  let stdout = "";
  let stderr = "";
  child.stdout.setEncoding("utf8");
  child.stderr.setEncoding("utf8");
  child.stdout.on("data", (chunk) => {
    if (stdout.length < 1024 * 1024) stdout += chunk;
  });
  child.stderr.on("data", (chunk) => {
    if (stderr.length < 64 * 1024) stderr += chunk;
  });
  const status = await new Promise((resolve, reject) => {
    child.once("error", reject);
    child.once("close", resolve);
  });
  if (status !== 0)
    throw new ToolInstallError(
      "development_system.tool_extract_failed",
      `status=${status} detail=${JSON.stringify(stderr.trim())}`,
    );
  return stdout;
}

function safeArchiveEntry(entry) {
  if (!entry || entry.includes("\0") || path.posix.isAbsolute(entry))
    return false;
  return !entry.split("/").some((part) => part === "..");
}

async function prepareDestination(directory) {
  await mkdir(directory, { recursive: true, mode: 0o700 });
  const stat = await lstat(directory);
  if (!stat.isDirectory() || stat.isSymbolicLink())
    throw new ToolInstallError(
      "development_system.tool_destination_invalid",
      `path=${directory}`,
    );
}

async function installOne(name, definition, release, environment) {
  const destination = userGlobalBin(environment);
  await prepareDestination(destination);
  const temporary = await mkdtemp(
    path.join(destination, `.development-system-${name}-${randomUUID()}-`),
  );
  const target = path.join(destination, name);
  let replacementStarted = false;
  let backup = null;
  try {
    const archive = path.join(temporary, "release.tar.gz");
    await download(release.url, archive, release.sha256, environment);
    const tarExecutable = await systemExecutable("tar", environment);
    const entries = (
      await runTar(tarExecutable, ["-tzf", archive], environment)
    )
      .split("\n")
      .filter(Boolean);
    if (
      entries.length === 0 ||
      entries.some((entry) => !safeArchiveEntry(entry))
    )
      throw new ToolInstallError(
        "development_system.tool_archive_unsafe",
        `tool=${name}`,
      );
    const extracted = path.join(temporary, "extracted");
    await mkdir(extracted, { mode: 0o700 });
    await runTar(
      tarExecutable,
      [
        "-xzf",
        archive,
        "-C",
        extracted,
        "--no-same-owner",
        "--no-same-permissions",
      ],
      environment,
    );
    const source = path.resolve(extracted, release.binaryPath);
    if (!source.startsWith(`${path.resolve(extracted)}${path.sep}`))
      throw new ToolInstallError("development_system.tool_manifest_invalid");
    const sourceStat = await lstat(source).catch(() => null);
    if (!sourceStat?.isFile())
      throw new ToolInstallError(
        "development_system.tool_extract_failed",
        `tool=${name} binary=${release.binaryPath}`,
      );
    const candidate = path.join(temporary, `${name}.candidate`);
    await copyFile(source, candidate, constants.COPYFILE_EXCL);
    await chmod(candidate, 0o755);
    const candidateVersion = await observedVersion(
      candidate,
      definition,
      environment,
    );
    if (candidateVersion !== definition.version)
      throw new ToolInstallError(
        "development_system.tool_version_mismatch",
        `tool=${name} expected=${definition.version} actual=${candidateVersion ?? "invalid"}`,
      );
    const targetStat = await lstat(target).catch((error) => {
      if (error.code === "ENOENT") return null;
      throw error;
    });
    if (targetStat && !targetStat.isDirectory()) {
      backup = path.join(temporary, `${name}.previous`);
      await link(target, backup);
    }
    await rename(candidate, target);
    replacementStarted = true;
    const installedVersion = await observedVersion(
      target,
      definition,
      environment,
    );
    if (installedVersion !== definition.version)
      throw new ToolInstallError(
        "development_system.tool_installed_version_invalid",
        `tool=${name} expected=${definition.version} actual=${installedVersion ?? "invalid"}`,
      );
    return target;
  } catch (error) {
    if (replacementStarted) {
      try {
        if (backup) await rename(backup, target);
        else await rm(target, { force: true });
      } catch (rollbackError) {
        throw new ToolInstallError(
          "development_system.tool_install_rollback_failed",
          `tool=${name} install=${JSON.stringify(error instanceof Error ? error.message : String(error))} rollback=${JSON.stringify(rollbackError instanceof Error ? rollbackError.message : String(rollbackError))}`,
        );
      }
    }
    throw error;
  } finally {
    await rm(temporary, { recursive: true, force: true });
  }
}

export async function installUserGlobalTools(
  environment = process.env,
  selectedNames = null,
) {
  const releases = await readManifest(environment);
  const target = targetKey(environment);
  const before = await inspectToolPolicy(environment);
  const requested = selectedNames ?? Object.keys(releases.tools);
  for (const name of requested) {
    const definition = releases.tools[name];
    if (!definition)
      throw new ToolInstallError(
        "development_system.tool_unknown",
        `tool=${name}`,
      );
    const current = before.tools.find((tool) => tool.name === name);
    if (current?.status === "compatible") continue;
    const release = definition.releases[target];
    if (
      !release?.url ||
      !/^[a-f0-9]{64}$/.test(release.sha256 ?? "") ||
      typeof release.binaryPath !== "string" ||
      !release.binaryPath
    )
      throw new ToolInstallError(
        "development_system.tool_release_unsupported",
        `tool=${name} target=${target}`,
      );
    await installOne(name, definition, release, environment);
  }
  const destination = userGlobalBin(environment);
  const effectiveEnvironment = {
    ...environment,
    PATH: `${destination}${path.delimiter}${environment.PATH ?? ""}`,
  };
  const after = await inspectToolPolicy(effectiveEnvironment);
  const incompatible = after.tools.filter(
    (tool) => requested.includes(tool.name) && tool.status !== "compatible",
  );
  if (incompatible.length > 0)
    throw new ToolInstallError(
      "development_system.tool_install_verification_failed",
      `tools=${incompatible.map((tool) => tool.name).join(",")}`,
    );
  const installed = requested.filter((name) => {
    const previous = before.tools.find((tool) => tool.name === name);
    return previous?.status !== "compatible";
  });
  return Object.freeze({
    ...after,
    inheritedPathIncludesDestination: before.inheritedPathIncludesDestination,
    usesUserGlobal:
      installed.length > 0 ||
      requested.some(
        (name) =>
          before.tools.find((tool) => tool.name === name)?.source ===
          "user-global",
      ),
    pathAction: before.pathAction,
    installed: Object.freeze(installed),
  });
}

async function execute(tool, arguments_, environment = process.env) {
  const destination = userGlobalBin(environment);
  const effectiveEnvironment = {
    ...environment,
    PATH: `${destination}${path.delimiter}${environment.PATH ?? ""}`,
  };
  const policy = await inspectToolPolicy(effectiveEnvironment);
  const status = policy.tools.find((candidate) => candidate.name === tool);
  if (!status)
    throw new ToolInstallError(
      "development_system.tool_unknown",
      `tool=${tool}`,
    );
  if (status.status !== "compatible" || !status.executable)
    throw new ToolInstallError(
      "development_system.tool_unavailable",
      `tool=${tool} minimum=${status.targetVersion} install="development-system setup --project <repo> --apply --yes"`,
    );
  const child = spawn(status.executable, arguments_, {
    stdio: "inherit",
    env: effectiveEnvironment,
  });
  const result = await new Promise((resolve, reject) => {
    child.once("error", reject);
    child.once("exit", (code, signal) => resolve({ code, signal }));
  });
  if (result.signal) process.kill(process.pid, result.signal);
  process.exitCode = result.code ?? 1;
}

async function main(arguments_) {
  const command = arguments_.shift();
  if (command === "status") {
    if (arguments_.length > 1 || (arguments_[0] && arguments_[0] !== "--json"))
      throw new ToolInstallError("development_system.tool_usage");
    process.stdout.write(
      `${JSON.stringify(await inspectToolPolicy(), null, 2)}\n`,
    );
    return;
  }
  if (command === "install") {
    const jsonIndex = arguments_.indexOf("--json");
    if (jsonIndex >= 0) arguments_.splice(jsonIndex, 1);
    const result = await installUserGlobalTools(
      process.env,
      arguments_.length > 0 ? arguments_ : null,
    );
    process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
    return;
  }
  if (command === "exec") {
    const tool = arguments_.shift();
    if (!tool) throw new ToolInstallError("development_system.tool_usage");
    await execute(tool, arguments_);
    return;
  }
  throw new ToolInstallError(
    "development_system.tool_usage",
    "usage=install-development-tool.mjs <status|install|exec>",
  );
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  main(process.argv.slice(2)).catch((error) => {
    const value = error instanceof Error ? error.message : String(error);
    process.stderr.write(`${value}\n`);
    process.exitCode = 2;
  });
}
