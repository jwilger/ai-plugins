# Development System

The single personal development plugin for Codex, also packaged for pi.

## pi package (experimental)

This directory is a pi package as well as a Codex plugin
([ADR-0018](../../docs/adr/0018-pi-package-delivery.md)). Install it in place:

```shell
pi install ./plugins/development-system
```

The package loads the skills in `skills/` and the adapter in
`pi/extension.ts`. The adapter:

- registers both MCP servers from `mcp.json` using the same plugin-root
  launchers, so binaries are repaired the same way as under Codex;
- runs `bin/development-system session-start --harness pi` in the background at
  session start and shows its warnings as notifications;
- adds `/development-system doctor` to rerun that check on demand.

The check warns when a project or user `mcp.json` entry shadows
`development-discipline` or `tiber`, and when pi settings disable built-in MCP
support. Codex subagents are not yet available in pi, and some skills still
describe Codex-specific setup.

## Host-local binaries

After installing or upgrading this plugin, download and verify its Rust
executables once from the marketplace checkout:

```shell
just install-development-system-binaries
```

On Linux x86_64, the command downloads the exact plugin-version bundle from the
repository's GitHub Release, verifies its SHA-256 sidecar and fixed archive
layout, and atomically installs `tiber` and
`development-discipline-mcp` in
`$PLUGIN_DATA/ai-plugins/development-system/<plugin-version>/<host>/`
under Codex. Direct CLI use falls back to `$XDG_DATA_HOME`, then
`~/.local/share`. Re-running the command
is safe; a newer plugin version is installed alongside previous versions.
The plugin root `mcp.json` declares both servers with plugin-relative
launchers. Each launcher repairs a missing matching binary before starting, so
consuming projects do not configure MCP servers. Project setup removes only a
previously managed Development System MCP block from `.codex/config.toml`.
Signed Tiber operations use the plugin-owned
`$PLUGIN_DATA/signing-agent-socket` file. Outside a portable harness,
the launcher uses `$XDG_CONFIG_HOME/ai-plugins/development-system/` (or
`~/.config`). The root installer captures a valid `SSH_AUTH_SOCK`; update
the plugin-owned file with an absolute socket path if your agent moves.
The release bundle contains statically linked Linux x86_64 executables, the
applicable license, and the exact source tag and commit. To force a locked
source build for diagnosis:

```shell
just install-development-system-binaries --from-source
```

The repository's Nix devshell is optional. If `just` is unavailable, run
`scripts/install-development-system-binaries.sh` from the marketplace checkout.

The installed `SessionStart` hook and both MCP launchers verify the binaries
and atomic installation marker against the plugin manifest version. It repairs
missing or stale installations automatically: Linux x86_64 uses the verified
release bundle, while hosts without a prebuilt bundle use the locked Cargo
build. The `setup` skill performs the same automatic check before configuring a
repository. Manual use is only needed for diagnosis or an intentional source
build.

It is inert outside a Git repository or without a valid schema-3
`.development-system.toml`. Read-only repository inspection remains available
in every state. Start configuration through the structured `setup.preview`
tool. It detects the project's manifests, lockfiles, conventional
source/test/docs and build paths, Nix devshell, and stack-native test commands,
then produces a schema-validated project-specific configuration instead of a
generic template. Review its exact scopes and named command catalog, then
review the generated project `lefthook.yml` and its separate pre-commit and
pre-push command selections, then explicitly confirm `setup.apply`. Setup
installs both Git hooks with Lefthook but never stages or commits configuration.

Normal Development System work relies on those Git operations for repository
verification: `git commit` triggers the fast pre-commit checks and `git push`
triggers the pre-push checks. Agents must not run the same gate commands
separately unless the user explicitly requests a diagnostic run.

## Recover a failed checkpoint gate

Version 6.9.5 supports recording a failed pre-commit attempt or a lightweight
review that requires remediation without pretending a commit or passing gate
exists. Both become canonical `failing` checkpoints; only the recorded causal
repair and immediate fresh testing may follow. Fresh lightweight review and a
new normal hook-backed commit are then required before exact verification and
authorized delivery. Full terminal review and exact-SHA CI requirements remain
unchanged.

For a session stranded by an older plugin, update and reinstall:

```shell
codex plugin marketplace upgrade ai-plugins
codex plugin add development-system@ai-plugins
codex plugin list --json
```

Verify the installed version is 7.0.0 or newer for protocol-v3 terminal review. Restart the harness and resume
the work in a fresh thread with its original request, immutable baseline, and
checkpoint ID. The startup hook repairs version-matched host binaries; verify
both MCP servers use the new plugin root and call `workspace-reader.status`
through the connected Development Discipline MCP. Its final-review attestation
must report `contract_version >= 3`, `minimum_clean_iterations: 1`,
`scoped_coverage: true`, `explicit_policy_migration: true`, and
`durable_pending_assignment_recovery: true` before terminal review.
If the release is not yet available, build from the updated marketplace
checkout into the installed MCP's exact data directory:

```shell
plugin_data=$(codex -C /tmp mcp list --json | jq -er '.[] | select(.name == "development-discipline") | .transport.env.PLUGIN_DATA | select(type == "string" and startswith("/"))')
PLUGIN_DATA="$plugin_data" nix develop -c just install-development-system-binaries --from-source
```

Verify the checkout version matches the installed plugin version, and verify
the runtime after restart. A stale explicitly configured project MCP
binding requires the separately previewed and approved setup migration.

For new checkpoint transitions, use the bundled
[`transition-local-checkpoint.sh` operation API](skills/development-workflow/references/checkpoint-operations.md).
It derives successors under the authoritative writer lock and returns durable
receipts keyed by stable operation IDs. Initialization, focused test outcomes,
review and gate outcomes, actual commits, exact verification and retry, local
or pushed delivery, CI observations, and terminal outcomes have explicit
operations. The recovery command below remains supported for older sessions.

Before changing source, reconcile the retained failed command/review evidence
with Git. HEAD must still equal the checkpoint's HEAD: if a commit exists,
recover its normal committed transition instead. Keep the real failure
evidence in a readable, nonempty file outside the worktree and retain that file
for handoffs. Include the failed command or review identity, actual failed
checks/findings, exit status when available, and causal diagnosis; avoid secrets.
The helper records a bounded path and SHA-256 reference, not raw log content.
If evidence is missing, recover it from the original session, or retry the
still-pending normal commit with captured output and record the actual result.

Run from the target repository; resolve the installed root rather than using
the old session's cached path:

```shell
plugin_root=$(codex -C /tmp mcp list --json | jq -er '.[] | select(.name == "development-discipline") | .transport.env.PLUGIN_ROOT')
checkpoint_id=YOUR_CHECKPOINT_ID
checkpoint="$(git rev-parse --path-format=absolute --git-common-dir)/development-system/checkpoints/$checkpoint_id.latest"
generation=$(sed -n 's/^checkpoint-v1 //p' "$checkpoint" | jq -er '.generation + 1')
predecessor=$(sha256sum "$checkpoint" | cut -d ' ' -f 1)
"$plugin_root/scripts/record-checkpoint-failure.sh" \
  "$checkpoint_id" "$generation" "$predecessor" pre-commit-hook \
  'THE ACTUAL FAILED GIT COMMAND' /absolute/path/to/retained-failure-evidence \
  'THE SPECIFIC CAUSAL REPAIR'
cat "$checkpoint"
```

For a failed review, substitute `lightweight-review` and the actual review
identity/evidence. The helper uses the validated writer's lock and
generation/predecessor compare-and-swap. It recomputes the current snapshot,
retains the immutable baseline and CI history, clears gate receipts and
delivery, and records `causal-edit: <repair>`. It rejects stale callers,
incompatible pending actions, changed HEAD, and missing/empty/in-worktree
evidence. It never edits source, stages, unstages, commits, pushes, or replaces
the checkpoint directly.

Staged work can remain staged, including newly staged previously untracked
files. Staging changes snapshot partitioning; reconcile source identity rather
than discarding work to reproduce old hashes. Include identified hook-made
changes in the actual failing snapshot. Unexplained source changes or a
malformed predecessor remain a recovery hold. Read back and reconcile the
published failing record before performing its causal repair. Never manually
replace `.latest`, fabricate evidence, or use the helper to skip testing,
review, commit hooks, or delivery gates.

The plugin-wide Development Discipline MCP surface provides bounded repository
inspection, deterministic setup, and multi-agent final review. Those services
and the Codex hooks are advisory: they
do not establish agent identity, isolate project tools, execute project
mutations, or deny ordinary host capabilities. `setup.apply` writes only the
previewed repository-local configuration and installs its Git hook launchers.
It does not generate privileged agents or
profiles and never changes global Codex, marketplace, MCP, shell, or
SSH settings.

Within that advisory boundary, final-review state fails closed. Protocol v3 requires one complete independently risk-selected coverage set, with justified additional samples for consequential risks. Repairs renew affected behavior and dependencies, retaining proven unchanged evidence. Whole-scope impact or unisolatable provenance requires full renewal; malformed assignments are replaced without counting invalid evidence. Model/document responsibilities and rendered behavior are assessed separately from production-code responsibilities. Exact-source binding, current regression verification, security scrutiny, actual lifecycle attestations, CI, CodeRabbit, and signed delivery remain required. V2 sessions retain three complete rounds unless explicitly migrated.

Tiber can additionally enforce receipts at the task boundary through `[final_review].minimum_clean_reviews`. Values one and two are valid explicit policies; larger requirements remain enforced. Task completion and trailer-driven delivery still require current independent evidence. See [the proportional policy](components/development-discipline/skills/final-review/references/proportional-review-policy.md).

The editor, runner, repository, and diagnostic services are retained as
unexposed reusable components for the standalone Tiber harness. Ordinary Codex
remains able to inspect, edit, verify, commit, and push while Tiber is being
bootstrapped. Tiber, rather than this plugin, will own authoritative identity,
isolation, workflow, memory, verification, and delivery.

Codex loads the plugin-root `mcp.json` and plugin-relative launchers. Project
MCP entries and global compatibility overrides are unnecessary. If a signed
append cannot reach the agent, update the plugin-owned signing socket path.

The core workflow requires only this plugin. The root installer offers GitHub
and CodeRabbit as optional companions. GitHub provides a connector mapping and
CodeRabbit provides skills backed by its CLI; neither requires a Development
System MCP dependency. Superpowers is omitted because its workflow overlaps.
The plugin owns its bundled MCP integrations; user-added MCPs are warned about
for compatibility review, not automatically rejected.

The plugin root is the active public surface: its manifests, hooks, launchers,
and `skills/` directory define the installed `development-system` plugin and
the `development-system:<skill>` routing names. Directories under `components/`
retain independently authored source, tests, manifests, and runtime assets that
the public plugin owns or wraps. Those component directory and manifest names
remain valid internal identities; they are not separate marketplace install
targets or public routing labels.

The top-level plugin manifest starts the advisory Development Discipline
inspection, setup, and multi-agent final-review surface plus the independent
Tiber MCP server. The advisory surface stores final-review transitions on a
separate local-only Git-backed EventCore authority and never publishes them to
`development-workflow`; the standalone workflow service retains that remote
authority. Development Discipline reads Tiber's unresolved CI hold when
evaluating delivery readiness. Tiber is the sole CI-incident and receipt
authority on `tiber`, in addition to publishing task events there.
Promptfoo remains an optional MCP owned by the
agentic-systems-engineering component and is intentionally excluded from the
top-level manifest because projects may disable that capability and must supply
the pinned Promptfoo runtime separately. Configure it explicitly when needed;
do not install the retained component as a separate marketplace plugin.

Skill descriptions are narrow routing indexes. Detailed workflow context is
loaded only after a matching skill routes.

## Model routing

The coordinator assigns bounded, standard, or strong capability according to the
task's eligibility and risk, then selects a concrete model and supported
reasoning setting from current runtime evidence. The plugin does not prescribe
model generations or provider-specific effort labels. Availability alone does
not establish fitness, and mappings cannot grant tool authority.

Optional string-valued `[model_routing]` entries in
`.development-system.toml` supply operator preferences.
`DEVELOPMENT_SYSTEM_MODEL_ROUTING_FILE` can select an absolute path to a
machine-specific TOML mapping instead. These are coordinator-read preferences,
not native spawn configuration; explicitly selected invalid mappings and
unconfirmed routes are reported visibly. See the [routing
skill](skills/model-routing/SKILL.md) and [mapping
contract](skills/model-routing/references/runtime-mappings.md) for keys, precedence,
examples, and fallback rules.

## Continue required review after a timed checkpoint

A medium-risk review's 75-minute boundary is a progress assessment. When
required repairs or fresh independent review remain and no human decision is
needed, call `final_review.continue_review` with the returned `state_ref`, a
stable `operation_id`, and a concrete `rationale`. The coordinator records that
assessment, retains the original start time and every blocker, and opens another
75-minute window. Preserve the recorded coverage and sample obligations: v3
requires complete scoped independent coverage, and retained v2 sessions keep
their original three-round contract. No budget decision bypasses these gates. Retry the exact request after a lost response; conflicting reuse of
an operation ID fails. Recovery of a legacy escalation hold additionally needs
`recovery_reference` explaining why its recorded dependency is resolved.

After source changes to a completed session, use `final_review.reopen` with its
authoritative reference, stable operation ID, reason, current diff identity,
complete changed-file inventory, and fresh shared test evidence. Reopening
preserves the pinned baseline and history and suspends completion until an
independent delta assessment. V3 renews affected behavior/dependencies and retains
proven unchanged receipts and sample floors; whole renewal needs material
whole-scope impact or unisolatable provenance. V2 retains its recorded full-reset
contract unless explicitly migrated.
A replayed historical reopen receipt explicitly identifies stale assignments;
resume current state before launching more reviewers.

For native persistence failures, use the version-matched
[one-operation replay helper](scripts/replay-review-operation.md) through the
host's supported approval mechanism. Its fixed writable paths cover the Git
objects, advisory refs and review replica, the exact per-binding SQLite report
directory, snapshot objects and private scratch. Source, unrelated refs and
configuration stay read-only; IP networking is denied. Signing, publication
locks and concurrency checks still apply. The historical narrow reproduction
shows that these writes were necessary, without proving every host's original
permission failure had the same cause.

Independent rejection evidence now travels with subsequent reviewer packets.
The coordinator uses exact stable identity and actual checked source dependencies,
so an unrelated documentation change does not discard a valid source-bound
resolution. A reviewer may challenge it with explicit new evidence and an
explanation of why the prior resolution fails; reopening requires independent
adjudication. Unsupported repeats do not create renewed escalation work and
still prevent a finding-free round.

Read `final_review.yield_report` for actual round counts and separate raw versus
adjudicated outcomes; use `final_review.evidence` to inspect the associated
records. Source changes compare immutable captured Git trees, with snapshot and
tree identities available in the round evidence. Scope-hash or staging changes
alone do not prove source changed. Historical hash-derived source claims and
missing snapshot evidence report `source_changed: null`. Missing legacy evidence
is reported as unavailable. Reports never infer
that a clean round was useless or reduce review requirements. The existing
serialized-state size limit remains a limitation for evidence-heavy sessions.
