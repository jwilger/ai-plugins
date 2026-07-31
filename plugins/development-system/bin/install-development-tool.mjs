#!/usr/bin/env node
import { createHash, randomUUID } from "node:crypto";
import { createReadStream, createWriteStream } from "node:fs";
import {
  access,
  chmod,
  copyFile,
  mkdir,
  mkdtemp,
  readFile,
  rename,
  rm,
} from "node:fs/promises";
import { constants } from "node:fs";
import { get as httpsGet } from "node:https";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import { fileURLToPath, pathToFileURL } from "node:url";
import { spawn } from "node:child_process";
import { pipeline } from "node:stream/promises";
import { Transform } from "node:stream";

const packageBin = path.dirname(fileURLToPath(import.meta.url));
const defaultManifest = path.join(packageBin, "tool-releases.json");
const supportedTools = new Set(["bd", "dolt"]);
const DEFAULT_DOWNLOAD_TIMEOUT_MS = 120_000;
const DEFAULT_MAX_ARCHIVE_BYTES = 256 * 1024 * 1024;

class ToolInstallError extends Error {
  constructor(code, details = "") {
    super(`${code}${details ? ` ${details}` : ""}`);
    this.code = code;
  }
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

function cacheRoot(environment = process.env) {
  if (environment.DEVELOPMENT_SYSTEM_TOOLS_DIR)
    return path.resolve(environment.DEVELOPMENT_SYSTEM_TOOLS_DIR);
  const base = environment.XDG_CACHE_HOME
    ? path.resolve(environment.XDG_CACHE_HOME)
    : path.join(os.homedir(), ".cache");
  return path.join(base, "development-system", "tools");
}

async function manifest(environment = process.env) {
  const source = environment.DEVELOPMENT_SYSTEM_TOOL_RELEASES
    ? path.resolve(environment.DEVELOPMENT_SYSTEM_TOOL_RELEASES)
    : defaultManifest;
  const parsed = JSON.parse(await readFile(source, "utf8"));
  if (parsed.schemaVersion !== 1 || !parsed.tools)
    throw new ToolInstallError("development_system.tool_manifest_invalid");
  return parsed;
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

function request(url, signal, redirects = 0) {
  return new Promise((resolve, reject) => {
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
        resolve(
          request(
            new URL(response.headers.location, url),
            signal,
            redirects + 1,
          ),
        );
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
      sourceUrl.protocol === "file:"
        ? createReadStream(fileURLToPath(sourceUrl))
        : sourceUrl.protocol === "https:"
          ? await request(sourceUrl, controller.signal)
          : (() => {
              throw new ToolInstallError(
                "development_system.tool_url_unsupported",
                `protocol=${sourceUrl.protocol}`,
              );
            })();
    const contentLength = Number(input.headers?.["content-length"]);
    if (Number.isFinite(contentLength) && contentLength > maxBytes)
      throw new ToolInstallError(
        "development_system.tool_download_size_exceeded",
        `maximum=${maxBytes} observed=${contentLength}`,
      );
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
    throw error;
  } finally {
    clearTimeout(timer);
  }
}

async function executable(file) {
  try {
    await access(file, constants.X_OK);
    return true;
  } catch {
    return false;
  }
}

export async function ensureTool(tool, environment = process.env) {
  if (!supportedTools.has(tool))
    throw new ToolInstallError(
      "development_system.tool_unknown",
      `tool=${tool}`,
    );
  const target = targetKey(environment);
  const releases = await manifest(environment);
  const definition = releases.tools[tool];
  const release = definition?.releases?.[target];
  if (
    !definition?.version ||
    !release?.url ||
    !/^[a-f0-9]{64}$/.test(release.sha256 ?? "") ||
    !release.binaryPath
  )
    throw new ToolInstallError(
      "development_system.tool_release_unsupported",
      `tool=${tool} target=${target}`,
    );
  const directory = path.join(
    cacheRoot(environment),
    tool,
    definition.version,
    target,
  );
  const installed = path.join(directory, tool);
  if (await executable(installed)) return installed;

  await mkdir(directory, { recursive: true, mode: 0o700 });
  const temporary = await mkdtemp(
    path.join(directory, `.install-${randomUUID()}-`),
  );
  try {
    const archive = path.join(temporary, "release.tar.gz");
    await download(release.url, archive, release.sha256, environment);
    const extracted = path.join(temporary, "extracted");
    await mkdir(extracted, { mode: 0o700 });
    const tar = spawn("tar", ["-xzf", archive, "-C", extracted], {
      stdio: ["ignore", "ignore", "pipe"],
    });
    let stderr = "";
    tar.stderr.setEncoding("utf8");
    tar.stderr.on("data", (chunk) => {
      stderr += chunk;
    });
    const status = await new Promise((resolve, reject) => {
      tar.once("error", reject);
      tar.once("close", resolve);
    });
    if (status !== 0)
      throw new ToolInstallError(
        "development_system.tool_extract_failed",
        `tool=${tool} status=${status} detail=${JSON.stringify(stderr.trim())}`,
      );
    const source = path.resolve(extracted, release.binaryPath);
    if (!source.startsWith(`${path.resolve(extracted)}${path.sep}`))
      throw new ToolInstallError("development_system.tool_manifest_invalid");
    const candidate = path.join(temporary, tool);
    await copyFile(source, candidate, constants.COPYFILE_EXCL);
    await chmod(candidate, 0o755);
    await rename(candidate, installed);
  } finally {
    await rm(temporary, { recursive: true, force: true });
  }
  return installed;
}

async function execute(tool, arguments_) {
  const binary = await ensureTool(tool);
  const child = spawn(binary, arguments_, {
    stdio: "inherit",
    env: process.env,
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
  const tool = arguments_.shift();
  if (command !== "exec" || !tool)
    throw new ToolInstallError(
      "development_system.tool_usage",
      "usage=install-development-tool.mjs exec <bd|dolt> [args...]",
    );
  await execute(tool, arguments_);
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  main(process.argv.slice(2)).catch((error) => {
    const value = error instanceof Error ? error.message : String(error);
    process.stderr.write(`${value}\n`);
    process.exitCode = 2;
  });
}
