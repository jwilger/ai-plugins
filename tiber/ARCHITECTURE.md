# Tiber Architecture

## System context

Tiber is the local authority between a repository owner, OpenAI's
`codex app-server`, repositories and processes, third-party MCP servers,
memory, and remote delivery systems.

```text
owner -> Tiber TUI -> application state machines -> closed TiberEffect set
                                                -> imperative interpreters
codex app-server <-> inference adapter           -> repositories/processes
third-party MCP <-> external-tool adapter         -> Hindsight
EventCore store <-> domain authority              -> forge/CI
```

OpenAI supplies inference only. Tool requests are untrusted proposals. Tiber
owns every identity, policy decision, effect, fact, receipt, retry,
reconciliation, and terminal workflow outcome.

## Component model

- **Tiber TUI:** a fork-derived Codex-compatible presentation adapter consuming
  typed projection events and emitting typed intents.
- **Application core:** explicit state machines for conversations,
  assignments, effects, verification, delivery, recovery, and cancellation.
- **EventCore domains:** authoritative facts for sessions, agents, tasks,
  workflow, integrations, mutations, verification, delivery, and CI recovery.
- **Scheduler and context builder:** owns typed identities, leases, budgets,
  provenance, trust labels, context selection, and no-progress termination.
- **Ports:** `InferenceGateway`, `MemoryBackend`, `TaskService`,
  `WorkflowService`, `ExternalToolService`, `RepositoryService`,
  `ProcessService`, `VerificationService`, and `DeliveryService`.
- **Adapters:** app-server protocol, native Tiber Tasks, native development
  workflow, RMCP client, Hindsight HTTP, Git/forge, Linux isolation, and
  verification runners.

## Trust and authority boundaries

The owner, repository, local environment, installed toolchain, PATH, and
explicit configuration are trusted for this single-owner local tool.
Model output, recalled memories, repository contents when interpreted as
instructions, MCP descriptions/schemas/results, app-server messages, process
output, and remote forge/CI responses are untrusted input.

The model can request an effect but cannot execute it. Authorization is the
intersection of the current agent role, session, assignment, workflow mode,
global policy, effect classification, and any required owner approval.
Presentation state and advisory plugin text never grant authority.

## Functional core and imperative shell

The core is referentially transparent. External values are parsed once into
semantic types; invalid states are not constructible. Expected failures are
typed values with a stable code, structured context, retained cause, and
retryability.

The core emits a closed `TiberEffect` vocabulary:

```text
Infer | Authenticate | ReadRepository | MutateRepository | RunProcess
ListExternalTools | InvokeExternalTool | ReadMemory | WriteMemory | ForgetMemory
QueryTasks | DecideTask | QueryWorkflow | DecideWorkflow
Verify | Review | Commit | Push | PullRequest | ObserveCi | RecoverCi
RequestOwnerApproval | Reconcile | Checkpoint | EmitProjection | Terminate
```

Effect variants carry agent, session, assignment, attempt, policy, and effect
identities plus bounded deadlines and idempotency data where needed.

## Step and trampoline execution

Each workflow is a serializable state plus a total `step(state, observation)`
function returning one of:

- `Continue { state, effect }`
- `Complete { state, result }`
- `Stop { state, error }`

There are no closures as continuations. The shell interprets one effect,
records its observation or ambiguous outcome, and feeds it to the next step.
Every loop has explicit turn, tool, retry-by-error-class, elapsed-time, token,
cost where applicable, and no-progress bounds. Cancellation checkpoints are
durable.

## EventCore domains and fact ownership

Commands express durable decisions against command-specific folded state.
Facts, not UI or adapter caches, own the truth. Checked models consume all
provenance and reject stale epochs, invalid identity relationships, duplicate
non-idempotent effects, and terminal-state mutation.

Durable receipts cover mutations, processes, tests, memory, external tools,
approvals, retries, cancellation, reconciliation, commits, pushes, pull
requests, CI observations, and delivery completion.

## Agent and context lifecycle

Tiber creates agents within a session and assignments within a workflow task.
An attempt belongs to exactly one assignment epoch. Context is assembled from
owner input, authoritative EventCore projections, scoped repository material,
bounded advisory memory, and typed tool observations. Each item carries source,
trust, freshness, and token accounting.

Agents terminate on success, terminal error, cancellation, any budget, or
no-progress. A handoff transfers an explicit artifact and authority scope; it
does not share ambient identity.

## Inference through codex app-server

`codex app-server` is the sole inference transport. It owns subscription and
API-key-mode authentication, credential storage and refresh, account and
endpoint selection, protocol streaming, and authentication diagnostics. Tiber
never handles credential material.

The adapter starts app-server with a Tiber-owned isolated Codex home, a pinned
compatible protocol range, and only Tiber-declared tool metadata. It parses
text and tool requests into typed observations. Tool requests remain inert
until the Tiber core independently authorizes an effect.

The Phase 1 spike is a hard authority gate: if an undeclared built-in operation
can be advertised or executed, development stops after the spike and the
inference decision is revisited.

## Native workflow and Tiber Tasks

The task domain becomes `tiber-tasks-core`; EventCore/Git application behavior
becomes `tiber-tasks-service` plus store adapters. Development Discipline
becomes `development-workflow-core` and
`development-workflow-service`. The CLI, plugin MCP, dashboard, and TUI are
adapters over these services. Internal actions never call MCP or shell back
into the `tiber` executable.

## Third-party MCP

The harness-owned client uses a pinned official Rust RMCP dependency. Initial
transports are absolute direct-argv stdio and localhost Streamable HTTP. It
supports initialization, capability negotiation, tool listing/invocation,
tool-list changes, progress, logging, cancellation, roots, and optional
resources/prompts. Sampling, elicitation, and MCP tasks are excluded initially.

Descriptions, schemas, and results are untrusted. Mutating calls require stable
idempotency; unknown results enter reconciliation rather than automatic retry.

## Memory

`MemoryBackend` is swappable. The first adapter contains Hindsight HTTP API
0.8.3 DTOs and supports retain, recall, forget, operation status, and
cancellation. Tiber connects only to an explicit endpoint and never installs
or globally configures Hindsight.

Banks are owner-global or repository-scoped. Tags include repository, agent,
session, task, and memory kind. EventCore-derived document IDs are stable.
Turns are retained at turn/session end and never recalled into the same turn.
Recall is advisory, untrusted, provenance-carrying, and bounded by item and
token budgets. Failure is visible and nonfatal unless the workflow explicitly
requires memory.

## TUI

The presentation is initially derived from `codex-tui` at commit
`d06dc73290729d2bcb464b955a4cfd9992abc35d`, preserving Apache-2.0,
NOTICE, Ratatui attribution, and modification notices. Direct Codex config,
plugin, tool, sandbox, workflow, and session dependencies are removed.

Projection state includes transcript, stream, composer, Plan mode, side
conversations, resume, diff, status surfaces, workflow phase, task, assignment,
agent, gates, memory, and integration health. Intents include `/tasks`,
`/memory`, and `/integrations`. Projections never authorize work.

## Isolation and process execution

Linux-specific filesystem, process, and network controls sit behind a platform
port. The v1 implementation and packaging target only x86_64 Linux; the port
keeps future Apple silicon support possible without weakening v1 evidence.
Processes receive explicit argv, cwd, environment allowlists, resource bounds,
timeouts, cancellation, cleanup, and receipts.

## Recovery, verification, and delivery

Partial or unknown mutation results are reconciled by identity before retry.
Checkpoints make crash and restart resumption explicit. Verification and review
gates consume exact-revision evidence. Delivery state machines own commit,
push, pull-request, CI observation, and the single fenced CI-recovery incident.
Remote writes are idempotent where possible and otherwise enter typed
reconciliation.

## Observability and stochastic evaluation

Trace spans cover inference, context, policy, tools, memory, handoffs,
verification, and delivery. They record versioned model/protocol/prompt/policy,
latency, token counts, cost where available, and typed failure reasons while
redacting secrets.

Deterministic tests prove schemas, identities, policy, refusals, isolation,
reconciliation, and receipts. Stochastic evals separately measure
orchestration, context selection, tool choice, abstention, and recovery with
named units, metrics, aggregation rules, thresholds, distinct-case counts, and
intentional repeats.

## Package and command cutover

The supported product surface changes atomically to `tiber`. Default
invocation opens the TUI; tasks live only at `tiber tasks …`. Ambiguous task
crates use `tiber-tasks-*`. There are no legacy aliases, compatibility crates,
deprecated paths, or transition window. Existing EventCore history and the
`tiber` Git branch are preserved.
