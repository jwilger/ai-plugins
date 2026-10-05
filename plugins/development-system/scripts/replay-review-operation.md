# One-operation final-review persistence recovery

`replay-review-operation.sh REPOSITORY_ROOT FINAL_REVIEW_TOOL < arguments.json`
reuses the version-matched installed Development Discipline binary. The JSON
file is the exact failed tool's arguments object, including its original
session identity, state reference, and evidence. The helper does not install or
upgrade binaries. Verify the connected MCP's runtime before choosing recovery.

Run this exact command through the host's supported approval mechanism when an
otherwise authorized native call cannot persist. Approval belongs to the host;
the helper never grants it. An explicit rejection must be respected. Native MCP
configuration and its sandbox remain unchanged.

The Linux helper executes a single allowlisted `final_review.*` operation using
the normal coordinator inside bubblewrap. It retains source, the index, Git
configuration, other branch refs, and the rest of the filesystem as read-only,
and isolates the IP network. The writable set is fixed:

- Common Git `objects`, `refs/tiber`, `logs/refs/tiber`, and
  `plugin-advisory-final-review` directories.
- The exact repository/work-item report directory and repository snapshot
  object directory under the existing XDG state root.
- One private temporary scratch directory.

SQLite requires a per-binding report directory to create and remove WAL files.
Existing flat projections rebuild from authoritative Git events. A genuine old
legacy session row remains readable through the existing guarded import path;
a flat projection never supplies new review credit. Exact-file SQLite mounts
were rejected after a real WAL fixture failed with a disk I/O error.

The helper preserves configured Git signing and the signing-agent socket. A
signing failure remains a failure. Filesystem diagnostics identify the phase,
path, and recovery condition. The helper does not automatically retry after a
failed operation. Source and state directories must be separate; symlinked
writable directory components are rejected. The boundary assumes cooperative
local processes, not a hostile same-user race against mount setup. Existing
Unix signing-agent sockets remain usable; IP networking is unavailable.

The normal EventCore adapter owns locking, compare-and-swap publication, and
pending-candidate reconciliation. When recovery confirms a retained candidate,
the response contains `publication_reconciled: true`, `operation_replayed:
false`, and `required_action: final_review.resume_latest`. Resume authoritative
state before any further submission. After interruption or an ambiguous output,
read authoritative status first; never fabricate a replacement state reference.

The allowlist includes assessment, planning, advance, split confirmation,
reopening, continuation, and review recovery/report reads. The coordinator still
validates whether an operation is supported and legal. Arbitrary tools, shell
commands, service selection, paths to other repositories, and extra JSON
requests are not accepted by the helper. Internal child dispatch is an
implementation detail; it supplies no host authorization and is not the public
recovery command.

The native integration tests use compiled production binaries and disposable
Git repositories under `/tmp`. They exercise individual denied paths, real SSH
signing, signing failure, interruption and retry, retained publication recovery,
projection rebuild, linked worktrees, path whitespace/newlines, and actual
source/index/config/ref/network denial from a signing subprocess. Bats tests
cover version matching and argument transport. These are deterministic
infrastructure checks; live provider-backed evaluations are not applicable.
