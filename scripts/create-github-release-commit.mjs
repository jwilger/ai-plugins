#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { pathToFileURL } from "node:url";

const root = path.resolve(import.meta.dirname, "..");
const graphqlUrl = "https://api.github.com/graphql";
const mutation = `
mutation CreateSignedReleaseCommit($input: CreateCommitOnBranchInput!) {
  createCommitOnBranch(input: $input) {
    commit {
      oid
      url
      signature {
        isValid
        state
        signer { login }
      }
    }
  }
}`;

export const releaseFiles = Object.freeze([
  "plugins/development-system/package.json",
  "plugins/development-system/.claude-plugin/plugin.json",
  "plugins/development-system/.codex-plugin/plugin.json",
  "plugins/development-system/.mcp.json",
  "plugins/development-system/components/agentic-systems-engineering/.mcp.json",
  "plugins/development-system/components/development-discipline/.mcp.json",
  "plugins/development-system/components/tiber/.mcp.json",
  ".claude-plugin/marketplace.json",
  ".agents/plugins/marketplace.json",
  "README.md",
  "plugins/development-system/README.md",
]);

export function releaseCommitInput({
  repository,
  branch,
  expectedHeadOid,
  version,
}) {
  if (!/^[^/]+\/[^/]+$/.test(repository))
    throw new Error("development_system.release_repository_invalid");
  if (!branch || !/^[0-9a-f]{40}$/.test(expectedHeadOid))
    throw new Error("development_system.release_branch_identity_invalid");
  if (!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(version))
    throw new Error("development_system.release_version_invalid");
  return {
    branch: { repositoryNameWithOwner: repository, branchName: branch },
    message: { headline: `chore(release): development-system v${version}` },
    fileChanges: {
      additions: releaseFiles.map((relative) => ({
        path: relative,
        contents: fs.readFileSync(path.join(root, relative)).toString("base64"),
      })),
    },
    expectedHeadOid,
  };
}

function argument(name) {
  const index = process.argv.indexOf(name);
  return index >= 0 ? process.argv[index + 1] : undefined;
}

async function createCommit({ token, input }) {
  const response = await fetch(graphqlUrl, {
    method: "POST",
    headers: {
      Accept: "application/vnd.github+json",
      Authorization: `Bearer ${token}`,
      "Content-Type": "application/json",
      "User-Agent": "ai-plugins-development-system-release",
      "X-GitHub-Api-Version": "2022-11-28",
    },
    body: JSON.stringify({ query: mutation, variables: { input } }),
  });
  const payload = await response.json();
  if (!response.ok || payload.errors?.length)
    throw new Error(
      `development_system.release_commit_api_failed status=${response.status} errors=${JSON.stringify(payload.errors ?? [])}`,
    );
  const commit = payload.data?.createCommitOnBranch?.commit;
  if (
    !/^[0-9a-f]{40}$/.test(commit?.oid ?? "") ||
    commit?.signature?.isValid !== true ||
    commit?.signature?.state !== "VALID" ||
    commit?.signature?.signer?.login !== "web-flow"
  )
    throw new Error("development_system.release_commit_signature_invalid");
  return commit;
}

const invokedDirectly =
  process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href;
if (invokedDirectly) {
  const token = process.env.GITHUB_TOKEN;
  const repository = process.env.GITHUB_REPOSITORY;
  const version = argument("--version");
  const expectedHeadOid = argument("--expected-head");
  const branch = argument("--branch") ?? "main";
  if (!token || !repository || !version || !expectedHeadOid) {
    console.error(
      "usage: GITHUB_TOKEN=... GITHUB_REPOSITORY=owner/repo create-github-release-commit.mjs --version <semver> --expected-head <oid> [--branch main]",
    );
    process.exit(2);
  }
  const input = releaseCommitInput({
    repository,
    branch,
    expectedHeadOid,
    version,
  });
  const commit = await createCommit({ token, input });
  process.stdout.write(`${JSON.stringify(commit)}\n`);
}
