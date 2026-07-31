#!/usr/bin/env node
import { execFileSync, spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import { pathToFileURL } from "node:url";

const PACKAGE_ROOT = path.resolve(import.meta.dirname, "..");
const packageBin = path.join(PACKAGE_ROOT, "bin");
let bdExecutable = path.join(packageBin, "bd");
let bdEnvironment = process.env;

function configureTools(project) {
  const installed = spawnSync(
    process.execPath,
    [
      path.join(packageBin, "install-development-tool.mjs"),
      "install",
      "--json",
      "bd",
    ],
    {
      cwd: project,
      encoding: "utf8",
      timeout: 180_000,
    },
  );
  if (installed.status !== 0)
    throw new Error(
      installed.stderr.trim() || "development_system.beads_install_failed",
    );
  const policy = JSON.parse(installed.stdout);
  const bd = policy.tools.find((tool) => tool.name === "bd");
  if (!bd?.executable || bd.status !== "compatible")
    throw new Error("development_system.beads_install_failed");
  bdExecutable = bd.executable;
  bdEnvironment = {
    ...process.env,
    PATH: `${policy.destination}${path.delimiter}${process.env.PATH ?? ""}`,
  };
}

function scalar(value) {
  const trimmed = value.trim();
  if (
    (trimmed.startsWith('"') && trimmed.endsWith('"')) ||
    (trimmed.startsWith("'") && trimmed.endsWith("'"))
  ) {
    if (trimmed.startsWith('"')) {
      try {
        return JSON.parse(trimmed);
      } catch {
        // Legacy Tiber documents may contain nonconforming YAML. Preserve text.
      }
    }
    return trimmed.slice(1, -1).replaceAll("''", "'");
  }
  return trimmed;
}

function array(value) {
  const trimmed = value.trim();
  if (trimmed === "[]") return [];
  if (!trimmed.startsWith("[") || !trimmed.endsWith("]")) return [];
  return trimmed
    .slice(1, -1)
    .split(",")
    .map(scalar)
    .map((item) => item.trim())
    .filter(Boolean);
}

function section(source, heading, nextHeadings) {
  const start = source.indexOf(`## ${heading}`);
  if (start < 0) return "";
  const bodyStart = source.indexOf("\n", start);
  if (bodyStart < 0) return "";
  let end = source.length;
  for (const next of nextHeadings) {
    const candidate = source.indexOf(`\n## ${next}`, bodyStart);
    if (candidate >= 0 && candidate < end) end = candidate;
  }
  return source.slice(bodyStart + 1, end).trim();
}

export function parseTiberTask(file, source) {
  const match = file.match(
    /^(backlog|in-progress|done|abandoned)\/([^/]+)\.md$/,
  );
  if (!match)
    throw new Error(`development_system.tiber_task_path_invalid path=${file}`);
  const frontmatter = source.match(/^---\n([\s\S]*?)\n---(?:\n|$)/);
  if (!frontmatter)
    throw new Error(
      `development_system.tiber_frontmatter_missing path=${file}`,
    );
  const fields = new Map();
  for (const line of frontmatter[1].split("\n")) {
    const assignment = line.match(/^([a-z_]+):\s?(.*)$/);
    if (assignment) fields.set(assignment[1], assignment[2]);
  }
  const title = scalar(fields.get("title") ?? "");
  if (!title)
    throw new Error(`development_system.tiber_title_missing path=${file}`);
  return Object.freeze({
    id: match[2],
    status: match[1],
    title,
    blockedBy: array(fields.get("blocked_by") ?? "[]"),
    blocks: array(fields.get("blocks") ?? "[]"),
    tags: array(fields.get("tags") ?? "[]"),
    prMrUrl: scalar(fields.get("pr_mr_url") ?? ""),
    prMrStatus: scalar(fields.get("pr_mr_status") ?? ""),
    summary: section(source, "Summary", [
      "Context / Why",
      "Acceptance criteria",
      "Subtasks",
      "Notes / Log",
    ]),
    context: section(source, "Context / Why", [
      "Acceptance criteria",
      "Subtasks",
      "Notes / Log",
    ]),
    acceptanceCriteria: section(source, "Acceptance criteria", [
      "Subtasks",
      "Notes / Log",
    ]),
    subtasks: section(source, "Subtasks", ["Notes / Log"]),
    notes: section(source, "Notes / Log", []),
  });
}

function issueId(prefix, tiberId) {
  return `${prefix}-tiber-${tiberId}`;
}

function issueStatus(status) {
  if (status === "in-progress") return "in_progress";
  if (status === "backlog") return "open";
  return "closed";
}

function issueType(tags) {
  return (
    ["bug", "feature", "epic", "chore", "task"].find((type) =>
      tags.includes(type),
    ) ?? "task"
  );
}

function issueDescription(task) {
  return [
    task.summary,
    task.context && `## Context / Why\n\n${task.context}`,
    task.subtasks && `## Legacy subtasks\n\n${task.subtasks}`,
  ]
    .filter(Boolean)
    .join("\n\n");
}

export function convertTiberBoard({ prefix, orderedOpenIds, tasks }) {
  const ids = new Set(tasks.map((task) => task.id));
  const order = new Map(orderedOpenIds.map((id, index) => [id, index]));
  const dependenciesByIssue = new Map();
  const addDependency = (dependent, blocker) => {
    if (!ids.has(dependent) || !ids.has(blocker)) return;
    const issue = issueId(prefix, dependent);
    const dependencies = dependenciesByIssue.get(issue) ?? [];
    const dependency = {
      issue_id: issue,
      depends_on_id: issueId(prefix, blocker),
      type: "blocks",
    };
    if (
      !dependencies.some(
        (candidate) =>
          candidate.depends_on_id === dependency.depends_on_id &&
          candidate.type === dependency.type,
      )
    )
      dependencies.push(dependency);
    dependenciesByIssue.set(issue, dependencies);
  };
  for (const task of tasks) {
    for (const blocked of task.blocks) addDependency(blocked, task.id);
    for (const blocker of task.blockedBy) addDependency(task.id, blocker);
  }

  return tasks.map((task) => {
    const id = issueId(prefix, task.id);
    const labels = [
      ...new Set(
        [
          ...task.tags,
          "migrated-from-tiber",
          task.status === "abandoned" ? "legacy-tiber:abandoned" : null,
        ].filter(Boolean),
      ),
    ];
    const metadata = {
      legacy_tiber: {
        id: task.id,
        status: task.status,
        order: order.get(task.id) ?? null,
        pr_mr_url: task.prMrUrl || null,
        pr_mr_status: task.prMrStatus || null,
      },
    };
    const issue = {
      id,
      title: task.title,
      description: issueDescription(task),
      acceptance_criteria: task.acceptanceCriteria,
      notes: task.notes,
      status: issueStatus(task.status),
      priority: 2,
      issue_type: issueType(task.tags),
      labels,
      external_ref: task.id,
      source_system: "tiber",
      metadata,
    };
    const dependencies = dependenciesByIssue.get(id);
    if (dependencies?.length) issue.dependencies = dependencies;
    return issue;
  });
}

export function migrateTiberPolicy(source) {
  if (
    !/^schema_version\s*=\s*1\s*$/m.test(source) ||
    !/^tiber\s*=\s*(true|false)\s*$/m.test(source)
  )
    return source;
  const delivery =
    source.match(/^mode\s*=\s*"([^"]+)"\s*$/m)?.[1] ?? "direct-to-trunk";
  const workflow =
    delivery === "pull-request"
      ? "development-change-pr"
      : delivery === "local-only"
        ? "development-change-local"
        : "development-change-direct";
  const output = [];
  let section = "";
  for (const line of source.split("\n")) {
    const sectionMatch = line.trim().match(/^\[([a-z0-9_.-]+)]$/);
    if (sectionMatch) section = sectionMatch[1];
    if (section === "tiber") continue;
    output.push(
      line
        .replace(/^schema_version\s*=\s*1\s*$/, "schema_version = 2")
        .replace(/^(\s*)tiber(\s*=\s*(?:true|false)\s*)$/, "$1beads$2"),
    );
  }
  while (output.at(-1) === "") output.pop();
  output.push("", "[beads]", `workflow = "${workflow}"`, "");
  return output.join("\n");
}

function installFormulas(project) {
  const sourceDirectory = path.join(PACKAGE_ROOT, "formulas");
  const targetDirectory = path.join(project, ".beads", "formulas");
  fs.mkdirSync(targetDirectory, { recursive: true });
  const added = [];
  for (const name of fs
    .readdirSync(sourceDirectory)
    .filter((entry) => entry.endsWith(".formula.toml"))) {
    const source = path.join(sourceDirectory, name);
    const target = path.join(targetDirectory, name);
    if (fs.existsSync(target)) {
      if (!fs.readFileSync(source).equals(fs.readFileSync(target)))
        throw new Error(
          `development_system.beads_formula_conflict path=${target}`,
        );
      continue;
    }
    fs.copyFileSync(source, target, fs.constants.COPYFILE_EXCL);
    added.push(target);
  }
  return added;
}

function git(project, args, options = {}) {
  return execFileSync("git", ["-C", project, ...args], {
    encoding: "utf8",
    maxBuffer: 16 * 1024 * 1024,
    ...options,
  }).trim();
}

function loadBoard(project) {
  const files = git(project, ["ls-tree", "-r", "--name-only", "tasks"])
    .split("\n")
    .filter((file) =>
      /^(backlog|in-progress|done|abandoned)\/.+\.md$/.test(file),
    );
  const tasks = files.map((file) =>
    parseTiberTask(file, git(project, ["show", `tasks:${file}`])),
  );
  let orderedOpenIds = [];
  try {
    orderedOpenIds = git(project, ["show", "tasks:order.md"])
      .split("\n")
      .map((line) => line.trim())
      .filter(Boolean);
  } catch {
    // A missing order file is valid for old Tiber boards.
  }
  return { tasks, orderedOpenIds };
}

function options(argv) {
  const result = {
    project: ".",
    prefix: null,
    mode: "dry-run",
    confirmed: false,
    output: null,
    push: false,
  };
  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];
    if (["--project", "--prefix", "--output"].includes(token)) {
      const value = argv[index + 1];
      if (!value || value.startsWith("--"))
        throw new Error(
          `development_system.migration_option_missing option=${token}`,
        );
      result[token.slice(2)] = value;
      index += 1;
    } else if (token === "--dry-run") result.mode = "dry-run";
    else if (token === "--apply") result.mode = "apply";
    else if (token === "--yes") result.confirmed = true;
    else if (token === "--push") result.push = true;
    else
      throw new Error(
        `development_system.migration_option_unknown option=${token}`,
      );
  }
  if (result.mode === "apply" && !result.confirmed)
    throw new Error("development_system.migration_requires_confirmation");
  return result;
}

function runBd(project, args, { input } = {}) {
  const result = spawnSync(bdExecutable, args, {
    cwd: project,
    encoding: "utf8",
    input,
    maxBuffer: 16 * 1024 * 1024,
    env: { ...bdEnvironment, BD_JSON_ENVELOPE: "1", BD_NON_INTERACTIVE: "1" },
  });
  if (result.status !== 0)
    throw new Error(
      `development_system.beads_command_failed command=${args[0]} ${result.stderr.trim() || result.stdout.trim()}`,
    );
  return result.stdout.trim();
}

function main(argv) {
  const selected = options(argv);
  const project = fs.realpathSync(selected.project);
  const gitDir = fs.realpathSync(
    git(project, ["rev-parse", "--path-format=absolute", "--git-dir"]),
  );
  const commonDir = fs.realpathSync(
    git(project, ["rev-parse", "--path-format=absolute", "--git-common-dir"]),
  );
  if (gitDir !== commonDir)
    throw new Error(
      `development_system.migration_requires_primary_checkout primary_checkout=${path.dirname(commonDir)}`,
    );
  const inferredPrefix = path
    .basename(project)
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 20)
    .replace(/-+$/g, "");
  const prefix = selected.prefix ?? (inferredPrefix || "beads");
  const trackedStatus = git(project, [
    "status",
    "--porcelain=v1",
    "--untracked-files=no",
  ]);
  if (selected.mode === "apply" && trackedStatus)
    throw new Error("development_system.migration_tracked_changes_present");
  const board = loadBoard(project);
  const issues = convertTiberBoard({ prefix, ...board });
  const configPath = path.join(project, ".development-system.toml");
  const configSource = fs.existsSync(configPath)
    ? fs.readFileSync(configPath, "utf8")
    : null;
  const migratedConfig =
    configSource === null ? null : migrateTiberPolicy(configSource);
  const jsonl = `${issues.map((issue) => JSON.stringify(issue)).join("\n")}\n`;
  if (selected.output)
    fs.writeFileSync(selected.output, jsonl, { flag: "wx", mode: 0o600 });

  if (selected.mode === "dry-run") {
    process.stdout.write(
      `${JSON.stringify({ mode: "dry-run", project, prefix, issues: issues.length, migratesPolicy: migratedConfig !== configSource, formulas: fs.readdirSync(path.join(PACKAGE_ROOT, "formulas")).filter((name) => name.endsWith(".formula.toml")).length, output: selected.output })}\n`,
    );
    return;
  }
  const sourceHead = git(project, ["rev-parse", "HEAD"]);
  const beadsDirectory = path.join(project, ".beads");
  if (fs.existsSync(beadsDirectory))
    throw new Error("development_system.migration_beads_workspace_exists");
  configureTools(project);
  let sourceCommitted = false;
  try {
    runBd(project, [
      "init",
      "--non-interactive",
      "--skip-agents",
      "--skip-hooks",
      "--prefix",
      prefix,
    ]);
    const initializedHead = git(project, ["rev-parse", "HEAD"]);
    if (initializedHead !== sourceHead) {
      if (git(project, ["rev-parse", `${initializedHead}^`]) !== sourceHead)
        throw new Error("development_system.beads_init_history_unexpected");
      git(project, ["reset", "--soft", sourceHead]);
    }
    if (migratedConfig !== null && migratedConfig !== configSource)
      fs.writeFileSync(configPath, migratedConfig, { mode: 0o600 });
    installFormulas(project);
    runBd(project, ["config", "set", "dolt.auto-commit", "on"]);
    const temporary = fs.mkdtempSync(path.join(os.tmpdir(), "tiber-to-beads-"));
    const inputFile = path.join(temporary, "issues.jsonl");
    try {
      fs.writeFileSync(inputFile, jsonl, { mode: 0o600 });
      const preview = runBd(project, [
        "import",
        inputFile,
        "--dry-run",
        "--json",
      ]);
      const imported = runBd(project, ["import", inputFile, "--json"]);
      runBd(project, [
        "dolt",
        "commit",
        "-m",
        "Migrate legacy Tiber task history",
      ]);
      git(project, [
        "add",
        "-f",
        "--",
        ...(migratedConfig !== null ? [".development-system.toml"] : []),
        ".beads/.gitignore",
        ".beads/README.md",
        ".beads/config.yaml",
        ".beads/metadata.json",
        ".beads/formulas",
      ]);
      const staged = git(project, ["diff", "--cached", "--name-only"]);
      let commit = null;
      if (staged) {
        git(project, [
          "commit",
          "--no-verify",
          "-m",
          "chore: migrate task workflows to Beads",
          "-m",
          "Replace the retired tracker with Dolt-backed Beads state and install the repository workflow formulas.",
        ]);
        commit = git(project, ["rev-parse", "HEAD"]);
      }
      sourceCommitted = true;
      if (selected.push) runBd(project, ["dolt", "push"]);
      process.stdout.write(
        `${JSON.stringify({ mode: "applied", project, prefix, issues: issues.length, preview: JSON.parse(preview), imported: JSON.parse(imported), commit, pushedDolt: selected.push })}\n`,
      );
    } finally {
      fs.rmSync(temporary, { recursive: true, force: true });
    }
  } catch (error) {
    if (!sourceCommitted) {
      try {
        runBd(project, ["dolt", "stop"]);
      } catch {}
      try {
        git(project, ["reset", "--hard", sourceHead]);
      } catch {}
      fs.rmSync(beadsDirectory, { recursive: true, force: true });
    }
    throw error;
  }
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  try {
    main(process.argv.slice(2));
  } catch (error) {
    process.stderr.write(
      `${error instanceof Error ? error.message : String(error)}\n`,
    );
    process.exitCode = 2;
  }
}
