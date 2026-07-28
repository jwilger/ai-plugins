import path from "node:path";

export type WorktreeName = string & { readonly __worktreeName: unique symbol };
export type WorktreeBranch = string & {
  readonly __worktreeBranch: unique symbol;
};

const controls = /[\u0000-\u001f\u007f]/;

export function parseWorktreeName(input: unknown): WorktreeName {
  if (
    typeof input !== "string" ||
    input.length === 0 ||
    input.length > 100 ||
    controls.test(input) ||
    input.startsWith("-") ||
    input === "." ||
    input === ".." ||
    !/^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(input)
  )
    throw new Error("development_system.worktree_name_invalid");
  return input as WorktreeName;
}

export function parseWorktreeBranch(input: unknown): WorktreeBranch {
  if (
    typeof input !== "string" ||
    input.length === 0 ||
    input.length > 255 ||
    controls.test(input) ||
    input.startsWith("-") ||
    input.startsWith("/") ||
    input.endsWith("/") ||
    input.includes("..") ||
    input.includes("@{") ||
    input.includes("//") ||
    input.includes("\\") ||
    !/^[A-Za-z0-9][A-Za-z0-9._/-]*$/.test(input)
  )
    throw new Error("development_system.worktree_branch_invalid");
  return input as WorktreeBranch;
}

export function configuredWorktreeRoot(
  primary: string,
  configured: string,
): string {
  if (
    !configured ||
    path.isAbsolute(configured) ||
    controls.test(configured)
  )
    throw new Error("development_system.worktree_root_not_repository_local");
  const resolved = path.resolve(primary, configured);
  const relative = path.relative(primary, resolved);
  if (
    relative === "" ||
    relative === ".." ||
    relative.startsWith(`..${path.sep}`) ||
    path.isAbsolute(relative)
  )
    throw new Error("development_system.worktree_root_not_repository_local");
  return resolved;
}

export function shellQuote(value: string): string {
  return `'${value.replaceAll("'", `'"'"'`)}'`;
}

export function relaunchCommand(worktreePath: string): string {
  return `cd -- ${shellQuote(worktreePath)} && exec pi`;
}
