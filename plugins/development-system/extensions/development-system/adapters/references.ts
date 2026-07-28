import { readFile, realpath } from "node:fs/promises";
import path from "node:path";

export const PI_REFERENCES = Object.freeze({
  extensions: "docs/extensions.md",
  tui: "docs/tui.md",
  packages: "docs/packages.md",
  skills: "docs/skills.md",
  rpc: "docs/rpc.md",
  json: "docs/json.md",
  keybindings: "docs/keybindings.md",
  sdk: "docs/sdk.md",
  models: "docs/models.md",
  "environment-variables": "docs/environment-variables.md",
  "custom-provider": "docs/custom-provider.md",
});

export type PiReferenceName = keyof typeof PI_REFERENCES;

async function piPackageRoot(entrypoint: string): Promise<string> {
  let candidate = path.dirname(await realpath(entrypoint));
  for (let depth = 0; depth < 12; depth += 1) {
    try {
      const manifest = JSON.parse(
        await readFile(path.join(candidate, "package.json"), "utf8"),
      ) as { name?: string };
      if (manifest.name === "@earendil-works/pi-coding-agent") return candidate;
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
    }
    const parent = path.dirname(candidate);
    if (parent === candidate) break;
    candidate = parent;
  }
  throw new Error("development_system.pi_documentation_root_unavailable");
}

export async function readPiReference(
  input: Readonly<{
    document: unknown;
    offset?: unknown;
    limit?: unknown;
    piEntrypoint?: string;
  }>,
): Promise<
  Readonly<{
    document: PiReferenceName;
    path: string;
    offset: number;
    lines: readonly string[];
    totalLines: number;
    nextOffset: number | null;
  }>
> {
  if (
    typeof input.document !== "string" ||
    !(input.document in PI_REFERENCES)
  )
    throw new Error("development_system.pi_reference_invalid");
  const offset = input.offset === undefined ? 1 : input.offset;
  const limit = input.limit === undefined ? 400 : input.limit;
  if (
    !Number.isInteger(offset) ||
    (offset as number) < 1 ||
    !Number.isInteger(limit) ||
    (limit as number) < 1 ||
    (limit as number) > 2_000
  )
    throw new Error("development_system.pi_reference_range_invalid");
  const root = await piPackageRoot(input.piEntrypoint ?? process.argv[1]);
  const document = input.document as PiReferenceName;
  const referencePath = path.join(root, PI_REFERENCES[document]);
  const source = await readFile(referencePath, "utf8");
  const allLines = source.split("\n");
  const selected = allLines.slice(
    (offset as number) - 1,
    (offset as number) - 1 + (limit as number),
  );
  let bytes = 0;
  const bounded: string[] = [];
  for (const line of selected) {
    const nextBytes = Buffer.byteLength(line) + 1;
    if (bytes + nextBytes > 50 * 1024) break;
    bounded.push(line);
    bytes += nextBytes;
  }
  const consumed = bounded.length;
  const nextOffset =
    (offset as number) - 1 + consumed < allLines.length
      ? (offset as number) + consumed
      : null;
  return {
    document,
    path: referencePath,
    offset: offset as number,
    lines: bounded,
    totalLines: allLines.length,
    nextOffset,
  };
}
