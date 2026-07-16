# ADR-0001: Contain writable agent benchmarks

## Status

Accepted

## Date

2026-07-16

## Context

The downstream code-quality benchmark gives a remote coding agent a disposable
project and permits it to edit files and run project commands. The agent's
commands, generated source, and generated tests are untrusted inputs even though
the benchmark is a single-owner development tool. A successful benchmark must
therefore establish both code-quality evidence and that the candidate could not
discover hidden controls, change its assigned skill condition, escape into the
host checkout, or consume unbounded host resources.

The protected assets are:

- the dedicated provider credential;
- the repository, sibling worktrees, home directory, and host configuration;
- hidden scorer controls, locked tests, baseline outputs, and the differences
  between benchmark conditions;
- host availability; and
- private raw transcripts and provider artifacts.

The repository owner, checked-in orchestration, Nix-selected toolchain, Linux
kernel, Bubblewrap, the user's systemd manager, Promptfoo, coding-harness SDKs,
and remote model providers are trusted. A malicious same-UID local process,
compromised host toolchain, compromised kernel, or an owner intentionally
bypassing the runner is outside this local benchmark's threat model. Ordinary
agent mistakes, adversarial generated commands, stale or malformed benchmark
state, cancellation, crashes, and resource exhaustion remain in scope.

## Decision

Run writable coding turns only through a fail-closed Linux boundary assembled
by repository-owned orchestration:

1. Materialize each condition in a private, marker-owned run root. Extract the
   bundled system skills from the verified package-native Codex binary, and
   build a minimal local marketplace and cache containing only direct
   `SKILL.md`-backed skill directories and their runtime files. Exclude
   repository-only `.plugin-eval` data. Bind those exact bytes, the rendered
   prompt, provider configuration, model-facing files, and tool versions into
   hashed runtime and workspace manifests. Validate the complete Promptfoo and
   per-condition Codex provider configuration semantically, rather than treating
   its hash as sufficient evidence.
2. Start Promptfoo with a clean, explicit environment. The host orchestration
   receives only the dedicated benchmark provider credential required for the
   selected provider. Candidate commands do not receive provider credentials or
   a network namespace with external connectivity.
3. Run every candidate turn in Bubblewrap namespaces with a disposable writable
   workspace, home, and temporary directory. Remap model-visible locations to
   constant `/workspace`, `/runtime/codex-home`, and `/runtime/tmp` paths so host
   roots, sample numbers, and condition labels are not experimental confounds.
   Expose the selected configuration, bundled system skills, sanitized
   marketplace, and marketplace cache read-only. Mount an empty `/nix/store` and
   then only the validated, hash-bound Nix closure required by the declared
   tools; never expose the host's full store or source snapshots. Validate
   ownership and execution-surface markers outside the model namespace, remove
   them from the candidate copy, mask the host input/output mirrors, and use
   neutral Git and operating-system identity text. Stage every nonstandard bind
   source beneath a neutral random runtime root so Linux mount metadata cannot
   reveal the original repository, sample, condition, or benchmark paths.
4. Place Promptfoo orchestration, every candidate turn, and trusted scoring in
   fixed systemd user scopes that cap aggregate memory, swap, process count, and
   CPU. Also enforce finite wall time, output, open-file, and workspace-size
   limits. Propagate cancellation to the whole Promptfoo scope, wait for that
   scope to terminate, and only then scan or remove private state.
5. Copy candidate output into a hidden staged mirror, mark it complete only from
   the trusted outer boundary, and publish it after the sandbox exits and all
   higher-priority cancellation, resource, and safety checks pass. Preserve the
   trusted host-owned Git metadata and reject paths, links, special files,
   excessive hard links, or sizes outside the declared output contract.
6. Keep the public verifier deterministic and non-secret so an agent may use it
   while developing. Run locked tests, baseline replay, diff-scope checks, and
   the final scorer afterward in a separate trusted boundary whose controls are
   not present in the candidate workspace. The scorer receives only the
   candidate output and the exact declared tool closure.
7. Store raw provider output only beneath the private run root. Publish only the
   allowlisted, secret-scanned evidence needed to reproduce and interpret the
   result: composition and input hashes, tool and model versions, sanitized diff
   evidence, skill activations, latency, token and cost data, and aggregate
   metrics. Derive activation names only from successful structured Codex turn
   commands that reference exact installed `SKILL.md` files; treat this as a
   path-reference heuristic, discard the raw commands, and treat unavailable
   telemetry as a provenance failure. Keep
   provider, operational, provenance, safety, and candidate-code failures
   distinct.

The runner refuses to start or publish a result when a required isolation
primitive, marker, manifest, hash, closure entry, ownership condition, or
sanitization invariant is absent. A dry-run may validate wiring, but it is not
accepted as behavior evidence.

## Consequences

### Positive

- Writable turns can exercise realistic implementation behavior without giving
  model-generated commands ambient access to the developer environment.
- Exact runtime, input, and closure hashes make condition comparisons auditable.
- Constant model-visible paths and neutral task wording avoid disclosing sample
  or treatment bookkeeping beyond the skills that constitute the treatment.
  Neutral repository/runtime identity and hidden ownership metadata extend that
  blinding to ordinary candidate inspection, including Linux mount metadata.
- A trusted scorer measures functional behavior, code quality, regression
  safety, and change scope independently of the candidate's own claims.
- Aggregate resource controls and bounded copy-out constrain denial-of-service
  and artifact-amplification failures.
- Sanitized summaries can be shared without exposing private transcripts or
  credentials.

### Negative

- Canonical writable runs require Linux, Bubblewrap, Nix, and a functioning
  systemd user manager.
- The runner must maintain explicit runtime and tool-closure contracts as Codex,
  Promptfoo, or the benchmark toolchain changes.
- Candidate behavior can differ slightly from an unconstrained interactive
  harness because network access and host state are intentionally absent.
- Provider-backed evidence is slower and more expensive than a structural
  dry-run.

## Alternatives Considered

### Rely on the coding harness's sandbox

The harness sandbox is not the repository-owned security boundary and does not
establish the exact host, credential, resource, artifact, and scorer invariants
needed for comparable benchmark evidence.

### Mount the full Nix store read-only

Read-only access prevents mutation but leaks unrelated source derivations,
including possible repository and verifier snapshots. An exact declared closure
is narrower and auditable.

### Use per-process limits and post-run secret scanning

Per-process limits do not cap a process tree, and scanning detects some leaks
only after exposure. They remain useful defense-in-depth but cannot replace
namespace, credential, and aggregate cgroup boundaries.

### Run the benchmark in Docker

Docker could provide a similar boundary but would add a daemon and a second
environment model to a repository already standardized on Nix. Bubblewrap plus
systemd scopes supplies the required local isolation with fewer moving parts.

### Use read-only or patch-only evaluations

Those evaluations are safer and cheaper, but they do not measure whether a
coding agent can discover, implement, test, and refine a real change. They may
supplement but cannot replace the writable benchmark.

## Revisit when

- the benchmark runs for multiple mutually untrusted users or on shared hosted
  infrastructure;
- non-Linux execution becomes a requirement;
- a coding harness exposes a stable, attestable boundary covering the same
  filesystem, network, credential, resource, and scorer controls;
- the benchmark needs controlled network access or external services; or
- the trusted toolchain or provider boundary changes materially.

## Related

- `docs/rules/proportional-threat-modeling.md`
- `evals/benchmarks/downstream-code-quality/README.md`
