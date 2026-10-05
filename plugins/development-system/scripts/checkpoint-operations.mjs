// Internal derivation and receipt support. Only the shell writer owns the task
// lock and publishes .latest; this module never runs tests, commits, or pushes.
import fs from "node:fs";
import path from "node:path";
import os from "node:os";
import crypto from "node:crypto";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const scriptRoot = path.dirname(fileURLToPath(import.meta.url));
const [command, target, ...args] = process.argv.slice(2);
const digest = (bytes) =>
  crypto.createHash("sha256").update(bytes).digest("hex");
const fail = (message) => {
  throw new Error(message);
};
const requireThat = (condition, message) => {
  if (!condition) fail(message);
};
const nonblank = (value) => typeof value === "string" && /\S/u.test(value);
const equal = (a, b) => JSON.stringify(a) === JSON.stringify(b);
const readJson = (file) => JSON.parse(fs.readFileSync(file, "utf8"));
let repositoryRoot;
const git = (...argv) =>
  execFileSync("git", argv, {
    cwd: repositoryRoot,
    maxBuffer: 64 * 1024 * 1024,
  });
const gitText = (...argv) =>
  git(...argv)
    .toString("utf8")
    .trim();
const pending = `${target}.pending-operation`;
const receipts = `${target}.operations`;
const emptyHash = digest("");
let objectFormat;
const checkpointBytes = (record) => `checkpoint-v1 ${JSON.stringify(record)}\n`;
const validId = (id) =>
  typeof id === "string" && /^[A-Za-z0-9._-]{1,128}$/u.test(id);
const receiptPath = (id) => {
  requireThat(validId(id), "invalid operation id");
  return path.join(receipts, `${id}.json`);
};

function syncFile(file) {
  const fd = fs.openSync(file, "r");
  try {
    fs.fsyncSync(fd);
  } finally {
    fs.closeSync(fd);
  }
}

function atomicJson(file, value) {
  const temporary = `${file}.${crypto.randomUUID()}.tmp`;
  try {
    fs.writeFileSync(temporary, `${JSON.stringify(value)}\n`, {
      mode: 0o600,
      flag: "wx",
    });
    syncFile(temporary);
    fs.renameSync(temporary, file);
    syncFile(path.dirname(file));
  } finally {
    fs.rmSync(temporary, { force: true });
  }
}

function recover() {
  if (!fs.existsSync(pending)) return;
  const receipt = readJson(pending);
  requireThat(
    validId(receipt.operation_id) &&
      digest(checkpointBytes(receipt.record)) === receipt.record_sha256,
    "malformed pending operation receipt; recovery hold",
  );
  const currentDigest = fs.existsSync(target)
    ? digest(fs.readFileSync(target))
    : null;
  if (currentDigest === receipt.record_sha256) {
    fs.mkdirSync(receipts, { recursive: true, mode: 0o700 });
    const destination = receiptPath(receipt.operation_id);
    if (fs.existsSync(destination)) {
      requireThat(
        equal(readJson(destination), receipt),
        "conflicting durable operation receipt",
      );
      fs.unlinkSync(pending);
    } else fs.renameSync(pending, destination);
    syncFile(receipts);
    syncFile(path.dirname(target));
  } else {
    requireThat(
      currentDigest === receipt.record.predecessor_sha256,
      "pending operation does not match authoritative checkpoint; recovery hold",
    );
    // The candidate never became authoritative. A later operation may retry it.
    fs.unlinkSync(pending);
    syncFile(path.dirname(target));
  }
}

function canonical(value) {
  if (Array.isArray(value)) return value.map(canonical);
  if (value && typeof value === "object")
    return Object.fromEntries(
      Object.keys(value)
        .sort()
        .map((key) => [key, canonical(value[key])]),
    );
  return value;
}

function pathDigestUpdate(hash, name, { mode, oid }) {
  hash.update(`${mode}\0`).update(name).update(`\0${oid}\n`);
}

// checkpoint-v1 uses Git's path-aware clean conversion for regular untracked
// files. Keep this routine shared by derivation and both publication checks.
function snapshot(root) {
  const tracked = digest(git("diff", "--binary", "--full-index", "HEAD", "--"));
  const stream = crypto.createHash("sha256");
  for (const name of splitPaths(
    git("ls-files", "--full-name", "--others", "--exclude-standard", "-z"),
  )) {
    const absolute = Buffer.concat([Buffer.from(`${root}/`), name]);
    const stat = fs.lstatSync(absolute);
    let identity;
    if (stat.isSymbolicLink()) identity = fileIdentity(root, name);
    else {
      requireThat(
        stat.isFile(),
        "unsupported untracked file type; recovery hold",
      );
      // --stdin-paths understands Git C quoting. Octal-quote every byte so paths
      // with newlines, quotes, or invalid UTF-8 never pass through JS argv text.
      const quoted = `"${[...name].map((byte) => `\\${byte.toString(8).padStart(3, "0")}`).join("")}"\n`;
      const oid = execFileSync("git", ["hash-object", "--stdin-paths"], {
        cwd: root,
        input: quoted,
      })
        .toString()
        .trim();
      // Preserve the checkpoint-v1 shell publisher's `test -x` contract,
      // including owner/group access and ACLs. Raw source mode uses bits below.
      let executable = false;
      try {
        fs.accessSync(absolute, fs.constants.X_OK);
        executable = true;
      } catch (error) {
        if (error.code !== "EACCES") throw error;
      }
      identity = { mode: executable ? "100755" : "100644", oid };
    }
    pathDigestUpdate(stream, name, identity);
  }
  return {
    head_oid: gitText("rev-parse", "HEAD"),
    tracked_sha256: tracked,
    untracked_sha256: stream.digest("hex"),
  };
}

function splitPaths(bytes) {
  const result = [];
  let start = 0;
  for (let i = 0; i < bytes.length; i++)
    if (bytes[i] === 0) {
      result.push(bytes.subarray(start, i));
      start = i + 1;
    }
  requireThat(start === bytes.length, "malformed Git path inventory");
  return result;
}

function sameFile(a, b) {
  return ["dev", "ino", "mode", "size", "mtimeNs", "ctimeNs"].every(
    (key) => a[key] === b[key],
  );
}

function fileIdentity(root, name) {
  const absolute = Buffer.concat([Buffer.from(`${root}/`), name]);
  const stat = fs.lstatSync(absolute, { bigint: true });
  requireThat(
    stat.isFile() || stat.isSymbolicLink(),
    "unsupported source file type; recovery hold",
  );
  objectFormat ??= gitText("rev-parse", "--show-object-format");
  const hash = crypto.createHash(objectFormat);
  let mode;
  if (stat.isSymbolicLink()) {
    const bytes = fs.readlinkSync(absolute, { encoding: "buffer" });
    hash.update(`blob ${bytes.length}\0`).update(bytes);
    mode = "120000";
  } else {
    const fd = fs.openSync(
      absolute,
      fs.constants.O_RDONLY | fs.constants.O_NOFOLLOW | fs.constants.O_NONBLOCK,
    );
    try {
      const opened = fs.fstatSync(fd, { bigint: true });
      requireThat(
        opened.isFile() && sameFile(stat, opened),
        "source file changed before hashing; recovery hold",
      );
      hash.update(`blob ${opened.size}\0`);
      const chunk = Buffer.allocUnsafe(64 * 1024);
      let total = 0n;
      for (;;) {
        const size = fs.readSync(fd, chunk, 0, chunk.length, null);
        if (size === 0) break;
        total += BigInt(size);
        requireThat(
          total <= opened.size,
          "source file grew while hashing; recovery hold",
        );
        hash.update(chunk.subarray(0, size));
      }
      requireThat(
        total === opened.size &&
          sameFile(opened, fs.fstatSync(fd, { bigint: true })),
        "source file changed while hashing; recovery hold",
      );
    } finally {
      fs.closeSync(fd);
    }
    mode = (stat.mode & 0o111n) !== 0n ? "100755" : "100644";
  }
  requireThat(
    sameFile(stat, fs.lstatSync(absolute, { bigint: true })),
    "source path changed while hashing; recovery hold",
  );
  return { mode, oid: hash.digest("hex") };
}

function indexEntries(bytes) {
  return splitPaths(bytes).map((entry) => {
    const tab = entry.indexOf(9);
    requireThat(tab > 0, "malformed Git index entry");
    const [mode, oid, stage] = entry
      .subarray(0, tab)
      .toString("ascii")
      .split(" ");
    requireThat(
      stage === "0",
      "unmerged index; resolve conflicts before checkpointing",
    );
    return { name: entry.subarray(tab + 1), mode, oid };
  });
}

// Committed cleanliness is separate from checkpoint-v1's configurable Git diff
// and from raw reviewed source, which intentionally ignores staging partition.
function committedGitlinks(indexBytes, treeBytes) {
  const indexed = new Map(
    indexEntries(indexBytes)
      .filter((entry) => entry.mode === "160000")
      .map((entry) => [entry.name.toString("hex"), entry.oid]),
  );
  const committed = splitPaths(treeBytes).flatMap((entry) => {
    const tab = entry.indexOf(9);
    requireThat(tab > 0, "malformed Git tree entry");
    const [mode, type, oid] = entry
      .subarray(0, tab)
      .toString("ascii")
      .split(" ");
    if (mode !== "160000") return [];
    requireThat(type === "commit", "malformed Git tree gitlink");
    return [{ name: entry.subarray(tab + 1), oid }];
  });
  requireThat(
    indexed.size === committed.length &&
      committed.every(
        (entry) => indexed.get(entry.name.toString("hex")) === entry.oid,
      ),
    "committed gitlinks do not match the index; stage and commit the reviewed child references before claiming committed delivery",
  );
  return committed;
}

function requireCommittedGitlinks(root, head) {
  const requireHead = () =>
    requireThat(
      gitText("rev-parse", "HEAD") === head,
      "HEAD changed while checking committed gitlinks; recovery hold",
    );
  requireHead();
  const entries = committedGitlinks(
    git("ls-files", "--full-name", "--stage", "-z"),
    git("ls-tree", "--full-tree", "-r", "-z", head),
  );
  for (const entry of entries)
    requireThat(
      gitlinkIdentity(root, entry.name).oid === entry.oid,
      "committed gitlink does not match the actual clean child HEAD; stage and commit the reviewed child reference before claiming committed delivery",
    );
  requireHead();
}

function requiresCommittedGitlinks(operation, previous, ciStatus) {
  if (
    [
      "initialize",
      "commit-success",
      "exact-verify-pass",
      "exact-verify-retry",
      "local-delivery",
      "push-readback",
    ].includes(operation)
  )
    return true;
  // A failed/queued CI observation and explicit gate-failure operations remain
  // recordable. Only successful CI may advance committed readiness.
  if (["ci-register", "ci-observe"].includes(operation))
    return ciStatus === "success";
  if (!["begin-edit", "terminal-review-pass"].includes(operation)) return false;
  const delivery = previous?.delivery;
  return Boolean(
    delivery?.commit_oid ||
    (delivery?.mode !== "local-only" && delivery?.pushed_oid),
  );
}

function gitlinkIdentity(root, name) {
  const absolute = Buffer.concat([Buffer.from(root), Buffer.from("/"), name]);
  const hold =
    "submodule is missing or deinitialized; initialize the submodule at its recorded path before checkpointing";
  let stat;
  try {
    stat = fs.lstatSync(absolute);
  } catch (error) {
    if (error.code === "ENOENT") fail(hold);
    throw error;
  }
  requireThat(stat.isDirectory(), hold);
  // Node decodes Buffer cwd values before spawning. An ASCII-named private
  // symlink lets the OS resolve the original pathname bytes without that loss.
  const temporary = fs.mkdtempSync(
    path.join(os.tmpdir(), "checkpoint-gitlink-"),
  );
  const alias = path.join(temporary, "worktree");
  try {
    fs.symlinkSync(absolute, alias);
    const requireDirectory = () => {
      const current = fs.lstatSync(absolute);
      requireThat(
        current.isDirectory() &&
          current.dev === stat.dev &&
          current.ino === stat.ino,
        "submodule directory changed while hashing; recovery hold",
      );
    };
    const childGit = (...argv) => {
      requireDirectory();
      return execFileSync("git", argv, {
        cwd: alias,
        maxBuffer: 64 * 1024 * 1024,
      });
    };
    let childRoot;
    try {
      childRoot = childGit("rev-parse", "--show-toplevel");
    } catch {
      fail(hold);
    }
    // Remove only Git's newline, retaining all actual pathname bytes.
    requireThat(
      childRoot
        .subarray(0, -1)
        .equals(fs.realpathSync.native(absolute, { encoding: "buffer" })),
      hold,
    );
    const head = childGit("rev-parse", "HEAD").toString().trim();
    requireThat(
      childGit(
        "status",
        "--porcelain=v1",
        "-z",
        "--untracked-files=all",
        "--ignore-submodules=none",
      ).length === 0,
      "submodule has dirty or untracked content; commit or reconcile child changes before checkpointing",
    );
    for (const entry of committedGitlinks(
      childGit("ls-files", "--stage", "-z"),
      childGit("ls-tree", "--full-tree", "-r", "-z", head),
    )) {
      const nested = gitlinkIdentity(absolute, entry.name);
      requireThat(
        nested.oid === entry.oid,
        "nested submodule has dirty HEAD; reconcile child changes before checkpointing",
      );
    }
    requireThat(
      childGit("rev-parse", "HEAD").toString().trim() === head,
      "submodule HEAD changed while hashing; recovery hold",
    );
    requireDirectory();
    return { mode: "160000", oid: head };
  } finally {
    fs.rmSync(temporary, { recursive: true, force: true });
  }
}

// Unlike snapshot, raw source identity is independent of HEAD/staging and clean
// filters. Index modes identify gitlinks; their identity uses actual child HEAD.
function sourceIdentity(root) {
  const entries = indexEntries(git("ls-files", "--full-name", "--stage", "-z"));
  const unique = new Map(
    entries.map((entry) => [entry.name.toString("hex"), entry]),
  );
  for (const name of splitPaths(
    git("ls-files", "--full-name", "--others", "--exclude-standard", "-z"),
  ))
    unique.set(name.toString("hex"), { name });
  const stream = crypto.createHash("sha256");
  for (const { name, mode } of [...unique.values()].sort((a, b) =>
    Buffer.compare(a.name, b.name),
  )) {
    let identity;
    if (mode === "160000") identity = gitlinkIdentity(root, name);
    else {
      try {
        identity = fileIdentity(root, name);
      } catch (error) {
        if (error.code === "ENOENT") continue;
        throw error;
      }
    }
    pathDigestUpdate(stream, name, identity);
  }
  return stream.digest("hex");
}

function predecessor() {
  if (!fs.existsSync(target)) return null;
  const bytes = fs.readFileSync(target);
  requireThat(
    bytes[bytes.length - 1] === 10 &&
      !bytes.includes(0) &&
      bytes.toString().split("\n").length === 2 &&
      bytes.subarray(0, 14).toString() === "checkpoint-v1 ",
    "malformed checkpoint predecessor",
  );
  const record = JSON.parse(bytes.subarray(14));
  const variables = {
    generation: JSON.stringify(record.generation),
    predecessor: record.predecessor_sha256 ?? "null",
    current_head: record.snapshot?.head_oid,
    current_tracked: record.snapshot?.tracked_sha256,
    current_untracked: record.snapshot?.untracked_sha256,
  };
  const argv = ["-e"];
  for (const [key, value] of Object.entries(variables)) {
    requireThat(typeof value === "string", "malformed checkpoint predecessor");
    argv.push(key === "generation" ? "--argjson" : "--arg", key, value);
  }
  argv.push("-f", path.join(scriptRoot, "checkpoint-record.jq"));
  execFileSync("jq", argv, {
    input: JSON.stringify(record),
    stdio: ["pipe", "ignore", "pipe"],
  });
  requireThat(
    Number.isSafeInteger(record.generation) && record.generation >= 0,
    "invalid predecessor generation",
  );
  const recordDigest = digest(bytes);
  let context = null;
  if (fs.existsSync(receipts))
    for (const entry of fs.readdirSync(receipts)) {
      if (!entry.endsWith(".json")) continue;
      const receipt = readJson(path.join(receipts, entry));
      if (receipt.record_sha256 === recordDigest) {
        requireThat(
          equal(receipt.record, record),
          "operation receipt differs from checkpoint",
        );
        context = receipt;
        break;
      }
    }
  return { record, digest: recordDigest, context };
}

function verifyRemote(remote, ref, head) {
  requireThat(
    ref.startsWith("refs/heads/"),
    "push readback requires a full branch ref",
  );
  gitText("check-ref-format", ref);
  const lines = gitText("ls-remote", "--exit-code", "--", remote, ref).split(
    "\n",
  );
  requireThat(
    lines.length === 1 && lines[0] === `${head}\t${ref}`,
    "authoritative remote ref does not match verified commit",
  );
}

const evidenceKeys = ["command", "receipt_file"];
const specs = {
  initialize: ["mode", "causal_edit"],
  "begin-edit": [...evidenceKeys, "causal_edit"],
  "edit-pass": evidenceKeys,
  "edit-fail": [...evidenceKeys, "failure_kind", "causal_repair"],
  "edit-invalid-test": [...evidenceKeys, "causal_repair"],
  "lightweight-review-pass": [...evidenceKeys, "route"],
  "lightweight-review-fail": [...evidenceKeys, "causal_repair"],
  "fast-gate-pass": evidenceKeys,
  "fast-gate-fail": [...evidenceKeys, "causal_repair"],
  "hook-failure": [...evidenceKeys, "causal_repair"],
  "commit-success": [...evidenceKeys, "mode"],
  "exact-verify-pass": evidenceKeys,
  "exact-verify-fail": evidenceKeys,
  "exact-verify-retry": evidenceKeys,
  "local-delivery": [],
  "local-snapshot-delivery": evidenceKeys,
  "push-readback": ["remote", "ref"],
  "ci-register": [
    ...evidenceKeys,
    "provider",
    "run_id",
    "commit_oid",
    "status",
  ],
  "ci-observe": [...evidenceKeys, "provider", "run_id", "commit_oid", "status"],
  "ci-recovery": [...evidenceKeys, "causal_repair"],
  "terminal-review-pass": evidenceKeys,
  "terminal-review-remediation": [...evidenceKeys, "causal_repair"],
};

function prepare(operationId, operation, inputFile) {
  requireThat(validId(operationId), "invalid operation id");
  const input = readJson(inputFile);
  requireThat(
    input && typeof input === "object" && !Array.isArray(input),
    "operation input must be an object",
  );
  const keys =
    operation === "initialize" && input.mode !== "local-only"
      ? [...specs.initialize, "remote", "ref"]
      : specs[operation];
  requireThat(
    keys !== undefined || operation === "read",
    "unknown checkpoint operation",
  );
  if (operation === "read") {
    requireThat(Object.keys(input).length === 0, "read takes an empty object");
    return { replayed: true, receipt: readJson(receiptPath(operationId)) };
  }
  const expectedKeys = [
    ...keys,
    "expected_generation",
    "expected_predecessor",
  ].sort();
  requireThat(
    equal(Object.keys(input).sort(), expectedKeys),
    `invalid fields for ${operation}; expected ${expectedKeys.join(", ")}`,
  );
  const requestDigest = digest(JSON.stringify(canonical({ operation, input })));
  if (fs.existsSync(receiptPath(operationId))) {
    const receipt = readJson(receiptPath(operationId));
    requireThat(
      receipt.request_sha256 === requestDigest,
      "operation id already used for different input",
    );
    return { replayed: true, receipt };
  }
  requireThat(
    Number.isSafeInteger(input.expected_generation) &&
      input.expected_generation >= 0,
    "invalid expected generation",
  );
  requireThat(
    input.expected_predecessor === null ||
      /^[0-9a-f]{64}$/u.test(input.expected_predecessor),
    "invalid expected predecessor",
  );
  for (const key of keys)
    requireThat(nonblank(input[key]), `${key} must be nonblank`);
  const current = predecessor();
  requireThat(
    input.expected_generation ===
      (current ? current.record.generation + 1 : 0) &&
      input.expected_predecessor === (current?.digest ?? null),
    "stale checkpoint generation or predecessor",
  );
  requireThat(
    (operation === "initialize") === (current === null),
    "initialize requires a missing checkpoint; other operations require a predecessor",
  );
  const root = gitText("rev-parse", "--show-toplevel");
  repositoryRoot = root;
  const identity = snapshot(root);
  const source = sourceIdentity(root);
  let evidence = null;
  if (keys.includes("receipt_file")) {
    const receiptFile = fs.realpathSync(input.receipt_file);
    requireThat(
      receiptFile !== root && !receiptFile.startsWith(`${root}/`),
      "evidence must be outside the worktree",
    );
    requireThat(
      fs.statSync(receiptFile).isFile(),
      "evidence must be a regular file",
    );
    const bytes = fs.readFileSync(receiptFile);
    requireThat(bytes.length > 0, "evidence must be nonempty");
    evidence = `${receiptFile} sha256=${digest(bytes)}`;
  }
  const gates = {
    lightweight_review_receipt: null,
    fast_gate_receipt: null,
    exact_identity_verification_receipt: null,
  };
  let record = current
    ? structuredClone(current.record)
    : {
        baseline_oid: identity.head_oid,
        snapshot: identity,
        state: "pushed-or-delivery-mode-equivalent",
        test: null,
        gates,
        delivery: null,
        ci: { runs: [], terminal_success_run_id: null },
        next_action: null,
      };
  const action = current?.record.next_action;
  const requireAction = (...actions) =>
    requireThat(
      actions.includes(action),
      `${operation} cannot perform predecessor next_action ${action}`,
    );
  const requireSnapshot = () =>
    requireThat(
      equal(identity, current.record.snapshot) &&
        (!current.context || current.context.source_sha256 === source),
      "source or HEAD changed since predecessor; recovery hold",
    );
  const requireHead = () =>
    requireThat(
      identity.head_oid === current.record.snapshot.head_oid,
      "HEAD changed since predecessor; recovery hold",
    );
  const failGate = (kind, invalidTest = false) => {
    record.state = "failing";
    record.test = {
      command: input.command,
      receipt_ref: evidence,
      outcome: invalidTest ? "pass" : "fail",
      failure_kind: kind,
    };
    record.gates = gates;
    record.delivery = null;
    record.ci.terminal_success_run_id = null;
    record.next_action = `${invalidTest ? "rewrite-invalid-test" : "causal-edit"}: ${input.causal_repair}`;
  };
  const mode =
    input.mode ??
    current?.context?.delivery_mode ??
    current?.record.delivery?.mode;
  if (input.mode)
    requireThat(
      ["local-only", "direct-to-trunk", "pull-request"].includes(input.mode),
      "invalid delivery mode",
    );
  const previousMode =
    current?.context?.delivery_mode ?? current?.record.delivery?.mode;
  requireThat(
    !current ||
      ["local-only", "direct-to-trunk", "pull-request"].includes(previousMode),
    "delivery mode is unknown; continue with the compatibility full-record workflow and explicitly reconcile legacy delivery and review evidence before typed continuation",
  );
  if (input.mode && previousMode)
    requireThat(
      input.mode === previousMode ||
        (operation === "commit-success" &&
          ["direct-to-trunk", "pull-request"].includes(previousMode) &&
          ["direct-to-trunk", "pull-request"].includes(input.mode)),
      "delivery mode rebind cannot cross local-only and remote delivery gates",
    );
  if (requiresCommittedGitlinks(operation, current?.record, input.status))
    requireCommittedGitlinks(root, identity.head_oid);
  switch (operation) {
    case "initialize":
      requireThat(
        identity.tracked_sha256 === emptyHash &&
          identity.untracked_sha256 === emptyHash,
        "initialization requires a clean baseline",
      );
      if (mode !== "local-only")
        verifyRemote(input.remote, input.ref, identity.head_oid);
      record.delivery = {
        mode,
        commit_oid: null,
        pushed_oid: mode === "local-only" ? null : identity.head_oid,
        local_snapshot: mode === "local-only" ? source : null,
      };
      record.next_action = `causal-edit: ${input.causal_edit}`;
      break;
    case "begin-edit":
      requireThat(
        action !== "enter-ci-recovery",
        "begin-edit cannot bypass failed CI; use ci-recovery with actual failure evidence",
      );
      requireAction(
        "register-exact-sha-ci-monitor",
        "monitor-exact-sha-ci",
        "terminal-review",
        "complete",
      );
      requireSnapshot();
      requireThat(
        record.state === "pushed-or-delivery-mode-equivalent" &&
          record.gates.exact_identity_verification_receipt?.outcome ===
            "pass" &&
          current.context?.source_sha256 === source &&
          (mode === "local-only"
            ? record.delivery?.local_snapshot === source
            : record.delivery?.pushed_oid === identity.head_oid),
        "begin-edit requires unchanged verified and delivered source",
      );
      record.state = "awaiting-causal-edit";
      record.test = null;
      record.gates = gates;
      record.delivery = null;
      record.ci.terminal_success_run_id = null;
      record.next_action = `causal-edit: ${input.causal_edit}`;
      break;
    case "edit-pass":
    case "edit-fail":
    case "edit-invalid-test":
      requireThat(
        /^(causal-edit|rewrite-invalid-test): \S/u.test(action ?? ""),
        "edit result requires a pending causal edit",
      );
      requireHead();
      if (operation !== "edit-pass")
        failGate(
          operation === "edit-invalid-test"
            ? "invalid-test"
            : input.failure_kind,
          operation === "edit-invalid-test",
        );
      else {
        record.state = "passing-awaiting-gates-or-review";
        record.test = {
          command: input.command,
          receipt_ref: evidence,
          outcome: "pass",
          failure_kind: null,
        };
        record.gates = gates;
        record.delivery = null;
        record.ci.terminal_success_run_id = null;
        record.next_action = "lightweight-review";
      }
      break;
    case "lightweight-review-pass":
      requireAction("lightweight-review");
      requireSnapshot();
      requireThat(
        ["commit", "local-snapshot"].includes(input.route),
        "invalid review route",
      );
      requireThat(
        input.route !== "local-snapshot" || mode === "local-only",
        "local snapshot requires local-only mode",
      );
      record.gates.lightweight_review_receipt = evidence;
      record.next_action =
        input.route === "commit"
          ? "commit-through-pre-commit-hook"
          : "fast-gate";
      break;
    case "lightweight-review-fail":
    case "fast-gate-fail":
    case "hook-failure": {
      const [required, kind] = {
        "lightweight-review-fail": ["lightweight-review", "lightweight-review"],
        "fast-gate-fail": ["fast-gate", "fast-gate"],
        "hook-failure": ["commit-through-pre-commit-hook", "pre-commit-hook"],
      }[operation];
      requireAction(required);
      requireHead();
      failGate(kind);
      break;
    }
    case "fast-gate-pass":
      requireAction("fast-gate");
      requireSnapshot();
      record.gates.fast_gate_receipt = evidence;
      record.next_action = "commit-or-record-local-snapshot";
      break;
    case "commit-success":
      requireAction(
        "commit-through-pre-commit-hook",
        "commit-or-record-local-snapshot",
      );
      requireThat(
        identity.head_oid !== current.record.snapshot.head_oid &&
          gitText(
            "-c",
            "log.showSignature=false",
            "show",
            "-s",
            "--format=%P",
            "HEAD",
          ) === current.record.snapshot.head_oid,
        "commit-success requires an actual direct successor commit",
      );
      requireThat(
        identity.tracked_sha256 === emptyHash &&
          identity.untracked_sha256 === emptyHash,
        "commit-success requires the complete reviewed source to be committed",
      );
      requireThat(
        current.context?.source_sha256 === source,
        "commit does not match retained reviewed source identity",
      );
      record.state = "committed";
      record.gates.fast_gate_receipt ??= evidence;
      record.delivery = {
        mode,
        commit_oid: identity.head_oid,
        pushed_oid: null,
        local_snapshot: null,
      };
      record.next_action = "verify-exact-commit";
      break;
    case "exact-verify-pass":
    case "exact-verify-fail":
      requireAction("verify-exact-commit");
      requireSnapshot();
      record.gates.exact_identity_verification_receipt = {
        receipt_ref: evidence,
        outcome: operation === "exact-verify-pass" ? "pass" : "fail",
      };
      record.next_action =
        operation === "exact-verify-fail"
          ? "repair-exact-identity-verification"
          : mode === "local-only"
            ? "record-local-delivery"
            : "push";
      break;
    case "exact-verify-retry":
      requireAction("repair-exact-identity-verification");
      requireThat(
        identity.tracked_sha256 === emptyHash &&
          identity.untracked_sha256 === emptyHash &&
          current.context?.source_sha256 === source,
        "exact verification repair changed source; recovery hold",
      );
      if (identity.head_oid !== current.record.snapshot.head_oid) {
        requireThat(
          gitText(
            "-c",
            "log.showSignature=false",
            "show",
            "-s",
            "--format=%P",
            "HEAD",
          ) ===
            gitText(
              "-c",
              "log.showSignature=false",
              "show",
              "-s",
              "--format=%P",
              current.record.snapshot.head_oid,
            ),
          "exact verification repair must preserve commit ancestry",
        );
      }
      record.delivery.commit_oid = identity.head_oid;
      record.gates.exact_identity_verification_receipt = null;
      record.next_action = "verify-exact-commit";
      break;
    case "local-delivery":
      requireAction("record-local-delivery");
      requireSnapshot();
      record.state = "pushed-or-delivery-mode-equivalent";
      record.delivery.local_snapshot = source;
      record.next_action = "terminal-review";
      break;
    case "local-snapshot-delivery":
      requireAction("commit-or-record-local-snapshot");
      requireSnapshot();
      requireThat(
        mode === "local-only",
        "local snapshot requires local-only mode",
      );
      record.state = "pushed-or-delivery-mode-equivalent";
      record.gates.exact_identity_verification_receipt = {
        receipt_ref: evidence,
        outcome: "pass",
      };
      record.delivery = {
        mode,
        commit_oid: null,
        pushed_oid: null,
        local_snapshot: source,
      };
      record.next_action = "terminal-review";
      break;
    case "push-readback": {
      requireAction("push");
      requireSnapshot();
      verifyRemote(input.remote, input.ref, identity.head_oid);
      record.state = "pushed-or-delivery-mode-equivalent";
      record.delivery.pushed_oid = identity.head_oid;
      record.next_action = "register-exact-sha-ci-monitor";
      break;
    }
    case "ci-register":
    case "ci-observe": {
      requireAction(
        "register-exact-sha-ci-monitor",
        "monitor-exact-sha-ci",
        "enter-ci-recovery",
        "terminal-review",
      );
      requireSnapshot();
      requireThat(
        record.delivery.mode !== "local-only" &&
          input.commit_oid === record.delivery.pushed_oid,
        "CI must identify the exact pushed commit",
      );
      requireThat(
        ["queued", "running", "success", "failure"].includes(input.status),
        "invalid CI status",
      );
      const history = record.ci.runs.filter(
        (run) => run.provider === input.provider && run.run_id === input.run_id,
      );
      requireThat(
        operation === "ci-register" ? history.length === 0 : history.length > 0,
        "CI register/observe does not match retained run history",
      );
      if (history.length > 0) {
        const last = history.at(-1);
        requireThat(
          last.commit_oid === input.commit_oid &&
            !["success", "failure"].includes(last.status),
          "terminal CI run is immutable; register a new run for a retry",
        );
        requireThat(
          !(last.status === "running" && input.status === "queued"),
          "CI status cannot regress",
        );
      }
      record.ci.runs.push({
        provider: input.provider,
        run_id: input.run_id,
        commit_oid: input.commit_oid,
        status: input.status,
      });
      const latest = new Map();
      for (const run of record.ci.runs.filter(
        (run) => run.commit_oid === input.commit_oid,
      ))
        latest.set(`${run.provider}\0${run.run_id}`, run);
      const observations = [...latest.values()];
      // A terminal-success observation is the caller's retained authoritative
      // required-build result. Keep failed older attempts in the append-only log.
      const successful = input.status === "success";
      record.ci.terminal_success_run_id = successful ? input.run_id : null;
      record.next_action = successful
        ? "terminal-review"
        : observations.some((run) => run.status === "failure")
          ? "enter-ci-recovery"
          : "monitor-exact-sha-ci";
      break;
    }
    case "ci-recovery":
      requireAction("enter-ci-recovery");
      requireSnapshot();
      failGate("ci");
      break;
    case "terminal-review-pass":
      requireAction("terminal-review");
      requireSnapshot();
      record.next_action = "complete";
      break;
    case "terminal-review-remediation":
      requireAction("terminal-review");
      requireSnapshot();
      failGate("terminal-review");
      break;
    default:
      fail("unsupported operation");
  }
  record.generation = input.expected_generation;
  record.predecessor_sha256 = input.expected_predecessor;
  record.snapshot = identity;
  return {
    operation_id: operationId,
    operation,
    request_sha256: requestDigest,
    record,
    source_sha256: source,
    delivery_mode: mode,
    evidence_ref: evidence,
  };
}

try {
  switch (command) {
    case "snapshot":
      repositoryRoot = gitText("rev-parse", "--show-toplevel");
      process.stdout.write(`${JSON.stringify(snapshot(repositoryRoot))}\n`);
      break;
    case "compatibility-route": {
      const [predecessorDigest, operationId] = args;
      const candidates = fs.existsSync(receipts)
        ? fs
            .readdirSync(receipts)
            .filter((name) => name.endsWith(".json"))
            .map((name) => path.join(receipts, name))
        : [];
      if (fs.existsSync(pending)) candidates.push(pending);
      const typed = candidates.some((file) => {
        const receipt = readJson(file);
        return (
          receipt.operation_id === operationId ||
          receipt.record_sha256 === predecessorDigest
        );
      });
      process.stdout.write(typed ? "typed\n" : "legacy\n");
      break;
    }
    case "recover":
      recover();
      break;
    case "read":
      process.stdout.write(
        `${JSON.stringify(readJson(receiptPath(args[0])))}\n`,
      );
      break;
    case "prepare":
      process.stdout.write(`${JSON.stringify(prepare(...args))}\n`);
      break;
    case "stage": {
      const receipt = JSON.parse(fs.readFileSync(0, "utf8"));
      const bytes = fs.readFileSync(args[0]);
      requireThat(
        equal(JSON.parse(bytes.subarray(14)), receipt.record),
        "candidate differs from operation result",
      );
      // The writer's checkpoint-v1 checks deliberately apply Git filters.
      // Recheck raw bytes at the publication boundary as well, before creating
      // a durable intent that could otherwise recover stale source credit.
      repositoryRoot = gitText("rev-parse", "--show-toplevel");
      requireThat(
        sourceIdentity(repositoryRoot) === receipt.source_sha256,
        "raw source changed while publishing checkpoint; recovery hold",
      );
      if (
        requiresCommittedGitlinks(
          receipt.operation,
          predecessor()?.record,
          receipt.record.ci.runs.at(-1)?.status,
        )
      )
        requireCommittedGitlinks(
          repositoryRoot,
          receipt.record.snapshot.head_oid,
        );
      receipt.record_sha256 = digest(bytes);
      // The writer serializes JSON through jq. Preserve those exact bytes for
      // interrupted-publication reconciliation, independent of key ordering.
      receipt.record = JSON.parse(bytes.subarray(14));
      fs.mkdirSync(receipts, { recursive: true, mode: 0o700 });
      atomicJson(pending, receipt);
      break;
    }
    default:
      fail("invalid internal checkpoint operation command");
  }
} catch (error) {
  process.stderr.write(`checkpoint operation: ${error.message}\n`);
  process.exitCode = 3;
}
