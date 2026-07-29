import { constants } from "node:fs";
import { access, readFile, realpath } from "node:fs/promises";
import path from "node:path";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import {
  StatusFlow,
  type ComponentStatus,
  type DevelopmentSystemStatus,
  type HarnessMode,
  type RepositoryIdentity,
  type StatusEffect,
} from "../core/status.ts";

const execFileAsync = promisify(execFile);

function targetFor(
  component: "tiber" | "development-discipline",
): string | null {
  const key = `${process.platform}:${process.arch}`;
  const targets: Record<string, readonly [string, string]> = {
    "linux:x64": ["x86_64-unknown-linux-gnu", "x86_64-unknown-linux-musl"],
    "linux:arm64": ["aarch64-unknown-linux-gnu", "aarch64-unknown-linux-musl"],
    "darwin:x64": ["x86_64-apple-darwin", "x86_64-apple-darwin"],
    "darwin:arm64": ["aarch64-apple-darwin", "aarch64-apple-darwin"],
  };
  const selected = targets[key];
  return selected?.[component === "tiber" ? 0 : 1] ?? null;
}

async function resolveRepository(project: string): Promise<RepositoryIdentity> {
  const projectPath = await realpath(project);
  const [
    { stdout: gitDirectory },
    { stdout: commonDirectory },
    { stdout: topLevel },
  ] = await Promise.all([
    execFileAsync("git", [
      "-C",
      projectPath,
      "rev-parse",
      "--path-format=absolute",
      "--git-dir",
    ]),
    execFileAsync("git", [
      "-C",
      projectPath,
      "rev-parse",
      "--path-format=absolute",
      "--git-common-dir",
    ]),
    execFileAsync("git", ["-C", projectPath, "rev-parse", "--show-toplevel"]),
  ]);
  const git = await realpath(gitDirectory.trim());
  const common = await realpath(commonDirectory.trim());
  const current = await realpath(topLevel.trim());
  const primary = path.dirname(common);
  return Object.freeze({
    current,
    primary,
    kind: git === common ? "primary" : "linked",
  });
}

async function componentStatus(
  packageRoot: string,
  component: "tiber" | "development-discipline",
): Promise<ComponentStatus> {
  const target = targetFor(component);
  const binary = component === "tiber" ? "tiber" : "development-discipline-mcp";
  const entrypoint = path.join(packageRoot, "bin", binary);
  if (target === null)
    return Object.freeze({
      available: false,
      entrypoint,
      target,
      error: "development_system.unsupported_platform",
    });
  const bundled = path.join(
    packageRoot,
    "components",
    component,
    "dist",
    target,
    binary,
  );
  try {
    await Promise.all([
      access(entrypoint, constants.X_OK),
      access(bundled, constants.X_OK),
    ]);
    return Object.freeze({ available: true, entrypoint, target });
  } catch {
    return Object.freeze({
      available: false,
      entrypoint,
      target,
      error: "development_system.component_unavailable",
    });
  }
}

async function perform(
  effect: StatusEffect,
  project: string,
  packageRoot: string,
): Promise<unknown> {
  switch (effect.type) {
    case "read-policy":
      try {
        return {
          source: await readFile(
            path.join(effect.primary, ".development-system.toml"),
            "utf8",
          ),
        };
      } catch (error) {
        if ((error as NodeJS.ErrnoException).code === "ENOENT")
          return { source: null };
        throw error;
      }
    case "resolve-repository":
      return resolveRepository(project);
    case "resolve-components":
      return Object.freeze({
        tiber: await componentStatus(packageRoot, "tiber"),
        "development-discipline": await componentStatus(
          packageRoot,
          "development-discipline",
        ),
      });
    case "run-doctor": {
      try {
        const { stdout } = await execFileAsync(
          path.join(packageRoot, "bin/development-system"),
          ["doctor", "--project", effect.primary, "--harness", "pi"],
        );
        return stdout
          .split("\n")
          .map((line) => line.trim())
          .filter(Boolean);
      } catch (error) {
        return [`development_system.doctor_failed ${(error as Error).message}`];
      }
    }
  }
}

export async function resolveStatus(
  project: string,
  packageRoot: string,
  mode: HarnessMode,
): Promise<DevelopmentSystemStatus> {
  const flow = new StatusFlow(mode);
  for (;;) {
    const current = flow.step();
    if (current.done) return current.value;
    flow.resume(await perform(current.effect, project, packageRoot));
  }
}
