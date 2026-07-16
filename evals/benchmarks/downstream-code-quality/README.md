# Downstream code-quality benchmark

This benchmark produces non-promotional, directional evidence about whether
marketplace skills improve the code Codex writes in a disposable downstream
Rust project. It deliberately measures one synthetic feature task; it is not a
general claim about plugin quality or model capability.

The canonical diagnostic runs three independent samples for each condition:

- no marketplace skills, while retaining Codex-bundled system skills;
- the declared quality-core marketplace skills; and
- every marketplace plugin's skills-only projection.

Each turn receives a fresh Git repository, Codex home, and temporary directory.
The outer boundary exposes only the selected skills, the writable fixture, the
pinned toolchain, and network access required by the Codex inference process.
Model shell commands receive no provider credentials and no network access.
Ownership markers, runtime manifests, host workspace mirrors, and execution
surface metadata are validated outside the turn and hidden from the candidate.
The visible Git identity, commit message, hostname, and account metadata are
neutral. Every nonstandard bind source is first copied beneath a neutral random
runtime root, so Linux mountinfo cannot disclose repository roots, sample
numbers, or condition labels. Candidate output lands in a hidden staged mirror;
only after the boundary exits cleanly does trusted code validate it and replace
the host working tree while preserving host-owned Git metadata.

## Run it

Inspect the nine-turn plan without writing anything:

```shell
nix develop -c scripts/evals/run-code-quality-benchmark.sh --dry-run
```

Inspect the exact candidate command path and Nix runtime closure separately:

```shell
nix develop -c scripts/evals/run-code-quality-benchmark.sh --runtime-preflight
```

Live execution requires a dedicated API key. Normal Codex login files are not
copied or used:

```shell
CODE_QUALITY_OPENAI_API_KEY=… \
  nix develop -c scripts/evals/run-code-quality-benchmark.sh
```

The provider run can take many hours. Every turn has finite resource and wall
limits, and the outer run also has a finite timeout. Raw Promptfoo results,
transcripts, logs, and assertion artifacts remain in a private temporary run
root. They are secret-scanned and deleted after a strict allowlist sanitizer
writes `evals/out/downstream-code-quality/results.json`.

Canonical execution is Linux-only and requires the Nix devshell, Bubblewrap,
and a functioning systemd user manager. The runner fails closed if any required
namespace, cgroup, package identity, exact provider setting, tool-closure entry,
or cancellation control is unavailable. Interrupting the runner terminates and
waits for the complete Promptfoo scope before private state is scanned and
removed.

The output file is created without overwriting prior evidence. Move or remove a
previous result deliberately before starting another canonical run.

## Interpret it

The public result contains per-condition success rate, empirical pass@3
capability (at least one of three samples passed), and pass^3 reliability (all
three passed). Candidate failures are measurement outcomes. Provider,
operational, provenance, and safety failures are separate infrastructure or
trust signals and make the run ineligible for diagnostic interpretation.

Trusted scoring ignores model prose. It snapshots candidate source, rebuilds
it in a verifier sandbox, and combines public black-box behavior, formatting,
Clippy, locked tests, candidate-regression replay against the pristine
baseline, diff-scope, and safety gates. Persisted change evidence contains only
bounded counts and hashes—never candidate-controlled paths or contents. Each
artifact also binds a versioned verifier-composition digest over the exact
repo-owned case mapping, assertion orchestration, scorer, public verifier,
isolation helpers, and locked fixture bytes that determine those gates.
Benchmark input/runtime provenance is bound separately by the exact input,
runtime, workspace, and test-case hashes that the assertion and results checker
validate.

Skill activation evidence is derived from the structured Codex turn, not from
Promptfoo's host-path heuristic. The trusted sanitizer accepts only successful
command records that name an exact, installed `SKILL.md` path from that row's
immutable runtime surface. It publishes sorted qualified skill names and the
constant evidence method
`codex-turn-successful-command-path-references`; this is a bounded path-reference
heuristic, not proof that a shell command read the file. Raw commands, outputs,
and transcripts are discarded. Missing or malformed turn telemetry is a
provenance failure, while a valid empty trace records an observed zero. Public provenance
also pins the Codex CLI and SDK, Node, Promptfoo, package lock, boundary, and
toolchain composition by version and/or SHA-256 as applicable.
