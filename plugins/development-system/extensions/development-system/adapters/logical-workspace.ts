import { execFile } from "node:child_process";
import { randomUUID } from "node:crypto";
import { readFile, realpath, stat, writeFile } from "node:fs/promises";
import path from "node:path";
import { promisify } from "node:util";
import type { ExtensionContext } from "@earendil-works/pi-coding-agent";
import {
  listWorktrees,
  type WorktreeInventory,
  type WorktreeRecord,
} from "./worktrees.ts";

const execFileAsync = promisify(execFile);

export const LOGICAL_WORKSPACE_ENTRY =
  "development-system-logical-workspace-state";

export type LogicalWorkspaceState = Readonly<{
  schemaVersion: 1;
  primary: string;
  path: string;
  kind: "primary" | "linked";
  branch: string | null;
  gitDirectory: string;
  gitDirectoryDevice: string;
  gitDirectoryInode: string;
  registrationMarker: string;
}>;

export type LogicalWorkspaceRestoreResult = Readonly<{
  status: "defaulted" | "restored" | "rejected";
  code?: string;
}>;

type WorkspaceContext = Pick<ExtensionContext, "cwd" | "sessionManager">;

function isState(value: unknown): value is LogicalWorkspaceState {
  if (!value || typeof value !== "object") return false;
  const state = value as Partial<LogicalWorkspaceState>;
  return (
    state.schemaVersion === 1 &&
    typeof state.primary === "string" &&
    typeof state.path === "string" &&
    (state.kind === "primary" || state.kind === "linked") &&
    (state.branch === null || typeof state.branch === "string") &&
    typeof state.gitDirectory === "string" &&
    typeof state.gitDirectoryDevice === "string" &&
    typeof state.gitDirectoryInode === "string" &&
    typeof state.registrationMarker === "string"
  );
}

async function registrationMarker(
  record: WorktreeRecord,
  gitDirectory: string,
  create: boolean,
): Promise<string> {
  if (record.primary) return "primary";
  const markerPath = path.join(
    gitDirectory,
    "development-system-registration-id",
  );
  try {
    const marker = (await readFile(markerPath, "utf8")).trim();
    if (/^[0-9a-f-]{36}$/i.test(marker)) return marker;
    throw new Error("development_system.logical_workspace_marker_invalid");
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== "ENOENT" || !create)
      throw error;
  }
  const marker = randomUUID();
  try {
    await writeFile(markerPath, `${marker}\n`, { flag: "wx", mode: 0o600 });
    return marker;
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== "EEXIST") throw error;
    const winner = (await readFile(markerPath, "utf8")).trim();
    if (/^[0-9a-f-]{36}$/i.test(winner)) return winner;
    throw new Error("development_system.logical_workspace_marker_invalid");
  }
}

async function stateFor(
  record: WorktreeRecord,
  primary: string,
  createMarker: boolean,
): Promise<LogicalWorkspaceState> {
  const { stdout } = await execFileAsync("git", [
    "-C",
    record.path,
    "rev-parse",
    "--path-format=absolute",
    "--git-dir",
  ]);
  const gitDirectory = await realpath(stdout.trim());
  const identity = await stat(gitDirectory, { bigint: true });
  return Object.freeze({
    schemaVersion: 1,
    primary,
    path: record.path,
    kind: record.primary ? "primary" : "linked",
    branch: record.branch,
    gitDirectory,
    gitDirectoryDevice: identity.dev.toString(10),
    gitDirectoryInode: identity.ino.toString(10),
    registrationMarker: await registrationMarker(
      record,
      gitDirectory,
      createMarker,
    ),
  });
}

function sameRegistration(
  left: LogicalWorkspaceState,
  right: LogicalWorkspaceState,
): boolean {
  return (
    left.primary === right.primary &&
    left.path === right.path &&
    left.kind === right.kind &&
    left.branch === right.branch &&
    left.gitDirectory === right.gitDirectory &&
    left.gitDirectoryDevice === right.gitDirectoryDevice &&
    left.gitDirectoryInode === right.gitDirectoryInode &&
    left.registrationMarker === right.registrationMarker
  );
}

/** Session-scoped authority for the repository path used by tools and guards. */
export class LogicalWorkspaceAuthority {
  private active: LogicalWorkspaceState | null = null;
  private transitioning = false;
  private readonly appendEntry: (customType: string, data: unknown) => void;

  constructor(appendEntry: (customType: string, data: unknown) => void) {
    this.appendEntry = appendEntry;
  }

  path(hostCwd: string): string {
    return this.active?.path ?? hostCwd;
  }

  state(): LogicalWorkspaceState | null {
    return this.active;
  }

  async resolve(hostCwd: string): Promise<string> {
    if (!this.active) return realpath(hostCwd);
    const inventory = await listWorktrees(hostCwd);
    if (inventory.primary !== this.active.primary)
      throw new Error("development_system.logical_workspace_stale_repository");
    const selected = inventory.worktrees.find(
      (worktree) => worktree.path === this.active?.path,
    );
    if (
      !selected ||
      selected.primary !== (this.active.kind === "primary") ||
      selected.branch !== this.active.branch
    )
      throw new Error(
        "development_system.logical_workspace_stale_registration",
      );
    let current: LogicalWorkspaceState;
    try {
      current = await stateFor(selected, inventory.primary, false);
    } catch {
      throw new Error(
        "development_system.logical_workspace_stale_registration",
      );
    }
    if (!sameRegistration(current, this.active))
      throw new Error(
        "development_system.logical_workspace_stale_registration",
      );
    return current.path;
  }

  async restore(
    context: WorkspaceContext,
  ): Promise<LogicalWorkspaceRestoreResult> {
    if (this.transitioning)
      return {
        status: "rejected",
        code: "development_system.logical_workspace_transition_busy",
      };
    return this.transition(async () => {
      let inventory: WorktreeInventory;
      try {
        inventory = await listWorktrees(context.cwd);
      } catch {
        this.active = null;
        return { status: "defaulted" };
      }
      const host = inventory.worktrees.find((worktree) => worktree.current);
      if (!host)
        throw new Error(
          "development_system.logical_workspace_host_unregistered",
        );
      this.active = await stateFor(host, inventory.primary, true);

      let persisted: LogicalWorkspaceState | null = null;
      for (const entry of context.sessionManager?.getBranch?.() ?? []) {
        if (
          entry.type === "custom" &&
          entry.customType === LOGICAL_WORKSPACE_ENTRY &&
          isState(entry.data)
        )
          persisted = entry.data;
      }
      if (!persisted) return { status: "defaulted" };
      if (persisted.primary !== inventory.primary)
        return {
          status: "rejected",
          code: "development_system.logical_workspace_stale_repository",
        };
      const selected = inventory.worktrees.find(
        (worktree) => worktree.path === persisted.path,
      );
      if (
        !selected ||
        selected.primary !== (persisted.kind === "primary") ||
        selected.branch !== persisted.branch
      )
        return {
          status: "rejected",
          code: "development_system.logical_workspace_stale_registration",
        };
      let restored: LogicalWorkspaceState;
      try {
        restored = await stateFor(selected, inventory.primary, false);
      } catch {
        return {
          status: "rejected",
          code: "development_system.logical_workspace_stale_registration",
        };
      }
      if (!sameRegistration(restored, persisted))
        return {
          status: "rejected",
          code: "development_system.logical_workspace_stale_registration",
        };
      this.active = restored;
      return { status: "restored" };
    });
  }

  async assertIdentity(expected: LogicalWorkspaceState): Promise<void> {
    const inventory = await listWorktrees(expected.primary);
    const selected = inventory.worktrees.find(
      (worktree) => worktree.path === expected.path,
    );
    if (!selected)
      throw new Error(
        "development_system.logical_workspace_stale_registration",
      );
    let current: LogicalWorkspaceState;
    try {
      current = await stateFor(selected, inventory.primary, false);
    } catch {
      throw new Error(
        "development_system.logical_workspace_stale_registration",
      );
    }
    if (!sameRegistration(current, expected))
      throw new Error(
        "development_system.logical_workspace_stale_registration",
      );
  }

  async resetToHost(context: WorkspaceContext): Promise<LogicalWorkspaceState> {
    return this.transition(async () => {
      const inventory = await listWorktrees(context.cwd);
      const host = inventory.worktrees.find((worktree) => worktree.current);
      if (!host)
        throw new Error(
          "development_system.logical_workspace_host_unregistered",
        );
      const next = await stateFor(host, inventory.primary, true);
      this.appendEntry(LOGICAL_WORKSPACE_ENTRY, next);
      this.active = next;
      return next;
    });
  }

  async finish<P, T>(
    context: Pick<WorkspaceContext, "cwd">,
    prepare: (source: LogicalWorkspaceState) => Promise<P | null>,
    operation: (
      source: LogicalWorkspaceState,
      primary: LogicalWorkspaceState,
      prepared: P,
    ) => Promise<T>,
  ): Promise<T | null> {
    return this.transition(async () => {
      const source = this.active;
      if (!source || source.kind !== "linked")
        throw new Error(
          "development_system.worktree_finish_requires_linked_checkout",
        );
      await this.assertIdentity(source);
      const prepared = await prepare(source);
      if (prepared === null) return null;
      await this.assertIdentity(source);
      const inventory = await listWorktrees(source.path);
      const primaryRecord = inventory.worktrees.find(
        (worktree) => worktree.primary,
      );
      if (!primaryRecord)
        throw new Error(
          "development_system.logical_workspace_primary_unregistered",
        );
      const primary = await stateFor(primaryRecord, inventory.primary, false);
      this.appendEntry(LOGICAL_WORKSPACE_ENTRY, primary);
      this.active = primary;
      return operation(source, primary, prepared);
    });
  }

  async activate(
    context: Pick<WorkspaceContext, "cwd">,
    requestedPath: string,
    expectedBranch?: string | null,
  ): Promise<LogicalWorkspaceState> {
    return this.transition(async () => {
      const target = await realpath(requestedPath).catch(() => {
        throw new Error("development_system.logical_workspace_not_registered");
      });
      const inventory = await listWorktrees(this.path(context.cwd));
      const selected = inventory.worktrees.find(
        (worktree) => worktree.path === target,
      );
      if (!selected)
        throw new Error("development_system.logical_workspace_not_registered");
      if (!selected.primary && selected.branch === null)
        throw new Error("development_system.logical_workspace_detached_head");
      if (expectedBranch !== undefined && selected.branch !== expectedBranch)
        throw new Error(
          "development_system.logical_workspace_branch_identity_changed",
        );
      const next = await stateFor(selected, inventory.primary, true);
      this.appendEntry(LOGICAL_WORKSPACE_ENTRY, next);
      this.active = next;
      return next;
    });
  }

  async activatePrimary(
    context: Pick<WorkspaceContext, "cwd">,
  ): Promise<LogicalWorkspaceState> {
    const inventory = await listWorktrees(this.path(context.cwd));
    return this.activate(context, inventory.primary);
  }

  private async transition<T>(operation: () => Promise<T>): Promise<T> {
    if (this.transitioning)
      throw new Error("development_system.logical_workspace_transition_busy");
    this.transitioning = true;
    try {
      return await operation();
    } finally {
      this.transitioning = false;
    }
  }
}
