# Development System Pi Extension — Product Requirements Document

- **Status:** Proposed
- **Primary product surface:** Pi
- **Secondary product surface:** Claude Code
- **Tertiary product surface:** Codex
- **Last updated:** 2026-07-27
- **Target repository area:** `plugins/development-system/`

## 1. Summary

The development-system plugin currently provides one consolidated development
workflow for Claude Code and Codex. It combines shared skills with setup,
worktree, task, review, delivery, and agentic-system components. This project
will make Pi the primary supported product surface while retaining Claude Code
and Codex support from the same canonical implementation and content.

Pi support will be delivered as a Pi package containing the existing public
skills and a first-party TypeScript extension. The extension will expose native
Pi tools, lifecycle integration, trusted user interactions, and deterministic
guardrails. It will reuse mature Rust components such as Tiber and the
development-discipline review coordinator through narrow, supervised protocol
adapters rather than rewriting them solely to obtain in-process calls.

New orchestration and lightweight tooling will prefer TypeScript. Shared
TypeScript capability definitions will be adaptable to Pi tools, MCP tools, and
CLI commands without independently implementing each surface. Existing
substantial Rust systems will remain authoritative until a separate,
evidence-backed reason justifies migration.

The evaluation system will add Pi as a first-class harness. It will measure the
shared skill behavior, Pi package loading, deterministic extension behavior,
and provider-backed outcomes using Pi with OpenAI subscription authentication.
The documentation, default workflows, eval presentation, and release criteria
will reflect the support order of Pi first, Claude Code second, and Codex third.

## 2. Background and current state

The current development-system plugin lives at
`plugins/development-system/`. Its public plugin root contains eight
consolidated skills:

- `agentic-systems`
- `delivery`
- `development-workflow`
- `engineering-standards`
- `eval-case-reporting`
- `setup`
- `tasks`
- `worktrees`

The plugin also bundles component implementations and more detailed specialist
material, including:

- the development-discipline final-review coordinator;
- Tiber task and CI-recovery tooling;
- worktree scripts;
- setup and compatibility diagnostics;
- agentic-system and eval-reporting guidance;
- Claude Code and Codex manifests, hooks, agents, and MCP configuration.

Claude Code and Codex already share the same physical plugin directory, but
some harness-specific metadata and agent representations must be synchronized.
Pi can load Agent Skills directly and can execute TypeScript extensions with
full lifecycle and tool interception support. Pi also supports OpenAI Codex
models through ChatGPT subscription authentication, making it the preferred
interactive harness for the owner. Pi's extension API additionally permits
hard behavior to be implemented deterministically instead of relying only on
model instructions.

A prior experimental Pi SDLC guard implementation explored event-sourced
workflow enforcement, signed process history, and broad delivery-command
interception. It was intentionally reverted because its implementation and
operating cost were disproportionate before user value had been established.
This project may reuse narrow lessons and test cases from that history, but it
must not restore the previous system wholesale.

## 3. Problem statement

The development system needs a Pi-native experience without creating a third,
independently maintained copy of its skills, tools, policies, or agent prompts.
At the same time, the design should take advantage of Pi capabilities that do
not exist, or are less directly accessible, in the other harnesses.

The principal problems are:

1. **No Pi product package exists.** Pi cannot currently install the
   development system as a supported package with an extension and an explicit
   resource set.
2. **Important rules remain stochastic.** Skills can guide a model, but they
   cannot reliably block a write in the wrong checkout, mediate a trusted
   approval, or prevent an unauthorized delivery operation.
3. **Component boundaries are harness-oriented.** Some behavior is exposed as
   MCP or CLI commands even when Pi could invoke a library or native extension
   tool more directly.
4. **A wholesale language rewrite would be wasteful.** Tiber and
   development-discipline have substantial Rust implementations and test
   coverage. Rewriting them in TypeScript only to avoid local protocol calls
   would add risk without clear product value.
5. **Eval coverage omits Pi.** Existing provider-backed marketplace evals cover
   Claude Code and Codex, so they cannot establish Pi package loading,
   extension effectiveness, or deterministic guard behavior.
6. **Support priority is not represented.** Documentation, release gates, and
   dashboard presentation do not yet identify Pi as primary, Claude Code as
   secondary, and Codex as tertiary.

## 4. Product vision

A user installs one development-system package and receives the strongest
experience available in the current harness:

- In **Pi**, skills supply progressive-disclosure guidance while a native
  extension supplies tools, orchestration, status, trusted interactions, and
  deterministic guardrails.
- In **Claude Code**, the same skills and shared implementations are exposed
  through the plugin, hook, MCP, and agent surfaces supported by Claude Code.
- In **Codex**, the same skills and shared implementations are exposed through
  Codex plugin, hook, MCP, and agent surfaces.

The underlying business rules and capability behavior are authored once.
Harness adapters may differ because harness APIs differ, but they must not
become independent semantic implementations.

Pi should provide the best-supported experience, not a behaviorally incompatible
fork. Pi-only enhancements are acceptable when they strengthen deterministic
execution or usability. Shared semantic policy must remain consistent across
all supported harnesses, and documentation must identify any enforcement that
a secondary harness cannot technically provide.

### 4.1 Support-order rationale

- **Pi is primary** because it combines first-class extension events and native
  TypeScript tools with access to OpenAI Codex models through the owner's
  ChatGPT subscription. It can therefore provide stronger deterministic
  behavior without giving up subscription-backed OpenAI usage.
- **Claude Code is secondary** because it remains the mature direct Anthropic
  harness and an important compatibility surface. Using Anthropic subscription
  authentication from a third-party harness such as Pi may draw from separately
  billed extra usage rather than ordinary Claude plan limits, so Pi is not
  intended to replace the direct Claude Code option for Anthropic work.
- **Codex is tertiary** because much of its OpenAI subscription-backed use can be
  performed through Pi while benefiting from the Pi extension. Codex remains
  supported for compatibility, native-provider comparison, and workflows where
  its own harness behavior is specifically desired.

Support order controls defaults, documentation emphasis, eval presentation, and
where new harness-specific UX is designed first. It does not lower the release
quality bar for a secondary or tertiary surface that the package still claims
to support.

## 5. Goals

### 5.1 Product goals

1. Make the development system installable and usable as a first-party Pi
   package.
2. Make Pi the default and best-documented harness for the development system.
3. Preserve Claude Code and Codex compatibility from the same source tree.
4. Eliminate manual maintenance of semantically duplicate skills, policies,
   tool schemas, and agent prompts.
5. Use deterministic Pi extension behavior for high-value guardrails and trusted
   user decisions.
6. Prefer TypeScript for new Pi-facing orchestration and lightweight
   capabilities.
7. Preserve mature Rust implementations where rewriting would not provide
   proportionate value.
8. Add Pi to package canaries, behavioral evals, deterministic guard tests, and
   reporting.
9. Establish an incremental path in which each guard is justified by a concrete
   failure mode and measurable behavior.

### 5.2 Engineering goals

1. Follow a functional-core/imperative-shell architecture.
2. Parse boundary inputs into semantic domain types and keep invalid states out
   of the core model.
3. Return typed, machine-readable errors across library, MCP, CLI, and Pi
   boundaries.
4. Keep model-visible context to the minimum that remains effective.
5. Support local installation without requiring globally installed project
   dependencies outside the repository's prescribed environment.
6. Keep extension and subprocess lifecycles bounded, cancellable, and
   observable.
7. Maintain an explicit, proportional threat model for a local, single-owner
   development tool.

## 6. Non-goals

The initial product does not aim to:

1. Rewrite Tiber in TypeScript.
2. Rewrite the development-discipline coordinator in TypeScript.
3. Restore the reverted event-sourced SDLC system or require a new workflow
   history branch.
4. Guarantee identical enforcement mechanisms in harnesses with different hook
   APIs.
5. Add Pi support to every marketplace plugin as part of this project.
6. Build a general-purpose MCP framework for arbitrary untrusted servers.
7. Implement an operating-system sandbox or defend against intentional bypass
   by the local owner or another malicious same-UID process.
8. Infer permission for destructive Git, publication, merge, or deployment
   operations.
9. Choose final package names, registry publication mechanics, every tool name,
   or every internal module boundary in this PRD.
10. Replace eval-based product decisions with an exhaustive up-front guardrail
    design.

## 7. Users and primary use cases

### 7.1 Primary user

The primary user is a single developer who uses Pi for day-to-day development,
uses OpenAI models through subscription authentication, and wants a consistent,
high-discipline workflow across repositories.

Primary needs include:

- installing the development system once;
- configuring a repository safely;
- routing work through the correct lifecycle phase;
- working in linked worktrees when configured;
- maintaining a bounded Tiber backlog;
- receiving deterministic protection around mistakes with high recovery cost;
- performing fresh-context, risk-proportional final review;
- selecting the configured delivery mode without inventing authorization;
- running and interpreting stochastic evals;
- retaining the ability to work in Claude Code or Codex when appropriate.

### 7.2 Maintainer

The maintainer needs to:

- change a skill or policy in one place;
- expose a capability through Pi, MCP, and CLI without reimplementing it;
- know which files are canonical and which are generated adapters;
- run focused deterministic tests before provider-backed evals;
- compare Pi behavior with Claude Code and Codex behavior;
- publish or install a package whose declared resources are explicit;
- avoid rebuilding mature subsystems merely because the primary harness
  changed.

### 7.3 Secondary-harness user

A Claude Code or Codex user needs the shared workflow to continue working with
clear documentation about which safeguards are instructional, hook-enforced, or
unavailable in that harness.

## 8. Product principles

### 8.1 One semantic source, multiple adapters

A semantic rule, prompt, schema, or operation must have one canonical source.
Harness-specific files may be generated or may adapt that source, but maintainers
must not edit equivalent behavior independently in several places.

Generated files may be committed when required by a harness or marketplace.
They must carry provenance, be reproducible, and be checked for drift in CI.
Generated duplication is a packaging artifact, not a second source of truth.

### 8.2 Determinism at consequential boundaries

Use the model for classification, synthesis, review judgment, and other
inherently stochastic work. Use deterministic code for:

- parsing configuration;
- resolving repository and worktree identity;
- enforcing path and mode boundaries;
- checking state and evidence freshness;
- mediating trusted user approval;
- invoking tools with bounded inputs and outputs;
- deciding whether a protected operation is mechanically eligible.

### 8.3 Protocol boundaries are acceptable

"Pi native" means that the capability is naturally discoverable and callable
as a Pi tool or command and participates in Pi lifecycle behavior. It does not
require every implementation to execute in the Pi process.

A supervised local MCP or JSON subprocess is acceptable when it preserves a
mature implementation, typed contract, cancellation, and bounded output. The
product should optimize for correctness and maintainability rather than
eliminating negligible local call overhead.

### 8.4 Selective TypeScript adoption

TypeScript is the preferred language for new extension logic, orchestration,
capability definitions, and small utilities because it integrates directly with
Pi and can share JSON Schema contracts with MCP. Language migration of an
existing component requires its own value case, migration plan, and regression
evidence.

### 8.5 Progressive disclosure

The always-loaded surface should contain routing descriptions and concise tool
guidance. Detailed specialist instructions, schemas, diagnostics, and reference
material should load only when needed.

### 8.6 Evidence before enforcement expansion

The first supported release should establish package loading and eval baselines.
The first primary-recommended release should add only a small set of
high-confidence guards. Additional guards should be admitted when a real failure
mode, eval miss, or recurring workaround demonstrates value.

## 9. Scope and priorities

Requirements use the following priorities:

- **P0:** required for the first supported Pi release;
- **P1:** required for Pi to become the primary recommended experience;
- **P2:** valuable follow-up that may ship after the primary transition.

The **primary-support gate** consists of every P1 requirement whose stated
consumer or applicability condition is present:

- packaging and adapters: FR-PKG-4, FR-SRC-3, FR-EXT-4, FR-CFG-4,
  FR-CAP-3, and FR-CAP-4;
- retained components and setup: FR-MCP-2 through FR-MCP-5, FR-MCP-7, and
  FR-SET-1 through FR-SET-4;
- deterministic guards: FR-GRD-1 through FR-GRD-6, FR-DEL-1, FR-DEL-2, and
  FR-DEL-4;
- authoritative workflow and interaction: FR-REV-1 through FR-REV-5 and
  FR-UI-1 through FR-UI-3; and
- executable evidence and support transition: FR-EVAL-7 and FR-DOC-1.

A requirement with an explicit consumer condition is inapplicable only when the
release does not expose that consumer; the release record must say so. No
smaller informal subset such as "the initial guards" may be used to claim that
Pi is the primary recommended experience. P2 requirements do not block that
transition.

Primary-gate evidence is matched to the requirement rather than forced through
one test shape:

- package, source, metadata, compatibility, and documentation requirements use
  reproducible generation/validation, clean-install canaries, cross-harness
  checks, and risk-proportional review;
- extension, configuration, adapter, component, setup, guard, delivery, review,
  and interaction behavior uses the black-box contracts in FR-EVAL-6; and
- model-dependent execution uses FR-EVAL-7 and the configured stochastic value
  gates in addition to deterministic boundary checks.

Documentation validation must not become a test that merely searches committed
Markdown for expected phrases; acceptance follows from the behavior and
artifact surfaces the documentation describes.

## 10. Functional requirements

### 10.1 Package and installation

#### FR-PKG-1 — Pi package manifest (P0)

The development-system plugin directory must be a valid Pi package with an
explicit manifest for extensions and skills. It must not depend on recursive
resource discovery that accidentally exposes internal component skills.

#### FR-PKG-2 — Explicit public skill set (P0)

The Pi package must load the same eight consolidated public skills exposed by
the development-system plugin root. Package validation must detect missing,
duplicate, or colliding skill names.

Promoting an internal specialist skill to the Pi public surface requires an
explicit product decision and corresponding cross-harness and eval updates.

#### FR-PKG-3 — Local installation (P0)

A user must be able to install the package from a local checkout using Pi's
local package mechanism. Installation instructions must identify the trusted
code-execution implications of Pi extensions.

#### FR-PKG-4 — Distribution path (P1)

The design must permit later npm publication without relocating or duplicating
the canonical implementation. The initial release may remain local-checkout
only. Registry name, scope, and public release timing are deferred decisions.

#### FR-PKG-5 — Runtime dependencies (P0)

Every dependency required by an installed Pi package at runtime must be declared
as a runtime dependency or Pi-provided peer dependency according to Pi package
rules. Development-only tooling must not be required in an installed package.

#### FR-PKG-6 — Version synchronization (P0)

Pi package metadata, Claude Code plugin metadata, Codex plugin metadata, and
marketplace entries must resolve to one release version. One canonical release
record must drive generation or strict validation of harness-specific metadata.

#### FR-PKG-7 — Pi compatibility contract (P0)

The package must declare and test a Pi compatibility range. Reproducible local
and CI canaries must pin the Pi version used as release evidence, and release
notes must identify intentional compatibility-range changes.

#### FR-PKG-8 — Clean-checkout bootstrap (P0)

Local-path installation must have one documented, reproducible bootstrap or
build command that materializes every runtime dependency and generated artifact
that Pi does not install for a referenced local package. It must follow the
repository's Nix and `.dependencies/` conventions and must not require a global
manual npm installation. The local-install canary must start from a clean
checkout and run after exactly those documented steps.

#### FR-PKG-9 — Pi support inventory (P0)

The repository must have one canonical, machine-readable declaration of which
marketplace packages support Pi and which Pi resources each exposes. Package
installation, full Pi-compatible marketplace eval selection, documentation,
and canaries must derive from that inventory rather than from directory
heuristics or Claude/Codex manifests.

#### FR-PKG-10 — Supported runtime targets (P0)

The initial bundled-component support matrix is:

- x86_64 Linux with the runtime needed by the bundled Tiber GNU target;
- aarch64 Linux with the runtime needed by the bundled Tiber GNU target;
- x86_64 macOS; and
- Apple-silicon macOS.

Tiber and development-discipline may select different Rust target triples
internally, but both required binaries must be present, executable, and verified
for every claimed product target. Windows and other OS/architecture combinations
are unsupported initially and must receive a deterministic unsupported-platform
result before a component is invoked.

Cargo fallback may remain a development convenience, but it must not be needed
to satisfy clean installed-package behavior or a supported-target release
canary.

### 10.2 Shared content and adapter generation

#### FR-SRC-1 — Canonical skills (P0)

All three harnesses must consume the same physical `SKILL.md` content for each
shared public skill. Harness-specific copies of skill prose are prohibited.

#### FR-SRC-2 — Canonical policy (P0)

Rules used by more than one adapter must live in a shared core or shared
component implementation. Pi, hook, MCP, and CLI adapters must not separately
encode equivalent eligibility or transition logic.

#### FR-SRC-3 — Agent definition consolidation (P1)

Where Claude Code and Codex require different agent file formats, agent intent,
system instructions, tool policy, and abstract model role must originate in one
canonical representation. Harness-specific Markdown or TOML may be generated
from that representation.

Generated representations must be deterministic and CI-checked. Harness-only
fields may remain in adapter-specific configuration.

#### FR-SRC-4 — Generated artifact drift detection (P0)

CI must fail when a generated harness artifact does not match its canonical
source. Maintainers must have one documented command for regeneration.

### 10.3 Extension startup and status

#### FR-EXT-1 — Extension loading (P0)

The package must load a first-party TypeScript extension in interactive, print,
JSON, and RPC modes, subject to Pi's trust model and resource settings.

#### FR-EXT-2 — Startup diagnostics (P0)

At session start, the extension must run or reuse the development-system
compatibility diagnostic behavior. It must report material conflicts such as
incompatible plugin settings, unsupported hook posture, or user-managed MCP
configuration without blocking unrelated work unless a concrete requirement is
violated.

#### FR-EXT-3 — Status surfaces (P0)

The extension must provide a user-invoked status command in modes that support
extension commands and a deterministic non-model entry point for headless modes
where command invocation is unavailable or unsuitable. A model-callable status
tool may supplement these surfaces but must not be the only way to obtain
status.

Status must report, at minimum:

- whether a development-system project configuration is present;
- the selected delivery mode when configured;
- enabled optional features;
- primary versus linked checkout identity;
- availability of bundled components;
- active enforcement limitations for the current Pi mode;
- actionable configuration or compatibility errors.

The model should receive only the portion needed for its current task. Detailed
status intended solely for the user should not be injected into model context by
default.

#### FR-EXT-4 — Reload safety (P1)

The extension must tolerate Pi resource reload and session replacement. It must
not retain stale session-bound objects or leave child processes running after
shutdown.

### 10.4 Project configuration

#### FR-CFG-1 — Authoritative project policy (P0)

`.development-system.toml` remains the authoritative project-level feature and
delivery policy unless superseded by an approved future schema migration.

#### FR-CFG-2 — Parse once (P0)

The configuration boundary must parse external TOML into semantic types. Domain
logic must not repeatedly interpret raw strings or independently validate the
same fields.

#### FR-CFG-3 — Missing configuration behavior (P0)

Read-only guidance may explain available modes when configuration is missing.
The system must not guess a delivery mode, optional feature state, or mutation
authority.

A capability that requires configured policy must return a typed, actionable
error or route to the explicit setup workflow.

#### FR-CFG-4 — Compatibility (P1)

Existing valid configuration files must continue to work. A future schema
change must define compatibility, per-repository backfill, rollback, and
recovery before implementation.

### 10.5 Shared TypeScript capability model

#### FR-CAP-1 — Reusable capability definition (P0)

A new lightweight capability needed by more than one surface must have one
semantic implementation and one canonical input, outcome, and error contract.
Its module may carry identity and presentation metadata when that reduces
adapter duplication; a central registry is an implementation option, not a
product requirement.

#### FR-CAP-2 — Pi adapter (P0)

When Pi consumes a capability, a thin adapter must register it as the
appropriate native Pi tool or command while preserving Pi cancellation,
progress, rendering, and mode semantics where applicable.

#### FR-CAP-3 — MCP adapter (P1)

When Claude Code, Codex, or another demonstrated consumer needs the capability
through MCP, a thin adapter must expose the canonical behavior through
`tools/list` and `tools/call` without a separately authored semantic
implementation. A capability does not require an MCP surface merely because an
adapter can be generated.

#### FR-CAP-4 — CLI adapter (P1)

When a capability is needed by shell hooks, users, or non-MCP automation, a thin
CLI adapter must parse boundary input, invoke the shared capability, and render
stable machine-readable results. Capabilities without such a consumer do not
require a CLI.

#### FR-CAP-5 — Contract portability (P0)

A capability shared between Pi and MCP must provide compatible JSON Schema at
both boundaries, either directly or through deterministic adapters generated
from the canonical contract. Provider-specific restrictions, including enum
compatibility, must be covered by contract tests. Capabilities that are not
shared with MCP are not required to adopt an MCP-shaped schema.

### 10.6 Retained Rust components and MCP bridge

#### FR-MCP-1 — Retain mature implementations (P0)

Tiber and development-discipline remain Rust implementations for the initial Pi
release. Their existing state, Git, locking, review, and CI-recovery semantics
must not be reimplemented in the extension.

#### FR-MCP-2 — First-party bridge (P1)

The Pi extension must be able to expose approved tools from the bundled Tiber
and development-discipline MCP servers as Pi tools. The bridge must be limited
to plugin-owned, explicitly configured servers; it is not a general arbitrary
MCP loader.

#### FR-MCP-3 — Dynamic discovery without semantic duplication (P1)

The bridge should obtain tool names, descriptions, and input schemas from the
server's MCP contract rather than copying them into TypeScript. The extension
may add concise Pi-specific routing metadata, but it must not alter tool
semantics.

#### FR-MCP-4 — Feature-aware activation (P1)

Tools must be active only when their corresponding project feature is enabled
or when they are required for configuration/status. Disabled feature tools must
not consume model context or imply availability.

#### FR-MCP-5 — Process supervision (P1)

The bridge must bound startup time, request time, output size, and shutdown. It
must propagate cancellation, close process groups it started, and distinguish a
component failure from a domain-level tool rejection.

#### FR-MCP-6 — Development-discipline library boundary (P2)

The development-discipline binary should be considered for refactoring into a
Rust library plus thin MCP executable. This is an internal maintainability
improvement, not a prerequisite for Pi support and not authorization for a
language rewrite.

#### FR-MCP-7 — Tool admission and collision safety (P1)

Every bridged tool must have an approved first-party origin and a deterministic,
Pi-safe public name. The bridge must namespace or otherwise reserve names so a
discovered tool cannot override a Pi built-in or another extension tool.
Duplicate and reserved names must fail closed before activation.

Discovered schemas must be checked or deterministically adapted for Pi and the
selected provider's supported tool-schema subset. A tool with an unsupported or
ambiguous schema must remain inactive with a bounded diagnostic. Canaries must
cover origin, naming, collision, and schema admission.

### 10.7 Setup and compatibility workflow

#### FR-SET-1 — Trusted setup interaction (P1)

Pi must provide a setup command or tool that deterministically:

1. resolves the repository and primary checkout;
2. rejects execution from a linked worktree;
3. produces the dry-run preview;
4. binds the preview to the repository identity and material preconditions that
   could change its effect;
5. displays the complete material preview to the user;
6. obtains explicit trusted confirmation for that preview;
7. rechecks the bound preconditions immediately before mutation and rejects a
   stale confirmation; and
8. applies exactly the confirmed operation.

The binding representation is an implementation decision, but approval must not
float across a changed plan or repository state.

#### FR-SET-2 — Non-TUI setup (P1)

Print and JSON modes must not manufacture approval. RPC responses also must not
be treated as human approval merely because Pi reports `ctx.hasUI`: an RPC
client can synthesize `extension_ui_response` messages. Until a separately
specified, authenticated, and auditable trusted-client contract exists, setup
mutation must require TUI confirmation and every non-TUI mode must stop after
preview with a typed confirmation-required result.

#### FR-SET-3 — Shared implementation (P1)

Pi, Claude Code hooks or commands, Codex hooks or commands, and direct CLI users
must reuse the same setup and doctor core. The existing shell implementation
may be replaced by a TypeScript core and compiled CLI adapter after behavioral
parity tests exist.

#### FR-SET-4 — Atomic failure behavior (P1)

Setup must preserve its existing all-or-recoverable behavior: it must not leave
a partially written configuration or claim success when its initialization
commit fails.

### 10.8 Deterministic worktree and path guardrails

#### FR-GRD-1 — Structured write interception (P1)

When worktrees are enabled, Pi must reject every generic model `write` and
`edit` operation that targets the coordination checkout. The guard must make
this decision from tool identity, canonical path, checkout identity, and
configuration; it must not infer the model's purpose.

A required coordination mutation, such as approved setup, may occur only
through an extension-owned capability with a narrower schema that binds the
exact previewed paths and preconditions. It is not implemented as an intent
exception to the generic tools.

#### FR-GRD-2 — Canonical path resolution (P1)

Path policy must operate on resolved project-relative identities, account for
absolute paths and symlinks, and reject paths outside the intended repository or
worktree boundary. It must not rely on substring matching alone.

#### FR-GRD-3 — Protected data paths (P1)

The extension must block mutation of protected repository metadata and populated
secret material by default. For paths classified as populated secret material,
the supported guarded tool composition must also mediate model-visible reads,
search results, and package-owned tool results without opening the file merely
to classify it.

The exact default path set and project override schema should be chosen during
implementation from concrete use cases and threat analysis. Documentation must
not claim protection from arbitrary disclosure through an unmediated shell or
third-party tool.

#### FR-GRD-4 — Shell-command policy (P1)

When worktrees are enabled, Pi's default `bash` tool must not provide a path for
model-authored coordination-checkout mutation. It must be mediated by an
enforcement boundary that can prove a command is permitted, replaced by a
bounded non-mutating command surface, or disabled in that checkout. A command
whose effect cannot be classified must fail closed; reporting a limitation is
not sufficient for this known built-in tool.

The same protected-operation contract must cover interactive user-bash where
project policy intentionally applies it. Direct RPC `bash` is a separate RPC
command path and must be handled according to FR-GRD-5. Shell policy must use a
tested classifier or conservative command contract rather than ad hoc regular
expressions for arbitrary shell syntax. Ordinary unrelated shell work in an
allowed linked worktree must remain usable.

#### FR-GRD-5 — Harness mode coverage (P1)

The implementation must verify which Pi events fire in TUI, print, JSON, and RPC
modes. Direct RPC `bash`, extension UI responses, and any operation outside the
ordinary model-tool path must be included in that characterization. A mode or
entry point that can bypass a required guard must disable or wrap the protected
operation, or be declared unsupported for guarded execution.

#### FR-GRD-6 — Guarded tool composition (P1)

Before claiming deterministic protection, the extension must account for every
active tool capable of crossing the protected mutation or data boundary. Such a
tool must be mediated, disabled in the guarded profile, or explicitly reported
as a limitation. Unknown third-party tools or newly discovered mutation
capabilities must downgrade the corresponding status claim rather than silently
creating a bypass.

#### FR-GRD-7 — User escape hatch (P2)

Where an override is appropriate, it must be narrow, visible, operation-specific,
and short-lived. General guard disablement and conversational approval are not
acceptable substitutes for an explicit policy mechanism.

### 10.9 Delivery and CI-recovery guardrails

#### FR-DEL-1 — Delivery-mode enforcement (P1)

Protected delivery behavior must respect the configured mode:

- `direct-to-trunk` permits only the configured direct delivery workflow;
- `pull-request` requires the configured branch and PR/MR workflow;
- `local-only` prohibits publication absent current explicit authorization.

Missing configuration must not default to a mode.

#### FR-DEL-2 — Destructive operation approval (P1)

Force pushes, forced refspecs, destructive remote operations, and equivalent
history rewrites require explicit case-by-case approval. Approval for one
operation must not authorize another.

#### FR-DEL-3 — Evidence freshness (P2)

Before enforcing a completion-sensitive delivery gate, the system must bind the
relevant verification and final-review evidence to the current repository
scope. A changed scope must invalidate stale evidence.

The exact evidence representation should reuse authoritative component state
where possible and is deferred until executable workflow scenarios establish
the minimum required contract.

#### FR-DEL-4 — CI-recovery hold (P1)

When Tiber records an active pushed-CI recovery incident, Pi must prevent the
development system from claiming readiness or initiating unrelated guarded
work. The hold ends only when Tiber's authoritative transition records terminal
success.

#### FR-DEL-5 — No MCP-only enforcement (P0)

A model-callable MCP tool is not an enforcement boundary because the model can
omit the call. Pi enforcement must occur in extension lifecycle/tool events;
Claude Code and Codex enforcement must occur in their supported hook or command
boundaries. MCP may supply authoritative state and operations to those guards.

### 10.10 Final review and subagent orchestration

#### FR-REV-1 — Preserve authoritative coordinator (P1)

Development-discipline remains authoritative for final-review planning,
progression, filtering, holds, and completion. The Pi extension must not create
a parallel review state machine.

#### FR-REV-2 — Fresh-context execution (P1)

Pi should orchestrate coordinator-assigned review work using fresh child agent
sessions. Each assignment must use the coordinator's abstract model role,
close after returning its result, and produce the required attestation about
model role, fresh context, and closure.

#### FR-REV-3 — Model routing (P1)

Pi must support project-configured mapping of abstract review roles to Pi
provider/model selections. Concrete model identifiers must remain
harness-specific configuration rather than universal policy.

The initial intended Pi/OpenAI mapping may follow the existing Sol/Terra/Luna
quality-cost strategy, but exact defaults remain a release-time decision based
on available models and eval evidence.

#### FR-REV-4 — Child isolation (P1)

A child reviewer must receive only the assignment, required scope, relevant
repository context, and permitted tools. It must not inherit unrelated parent
conversation history as hidden review state.

#### FR-REV-5 — Failure propagation (P1)

Timeout, cancellation, provider error, malformed result, or failed attestation
must remain an unresolved assignment. The extension must not synthesize a pass
or silently downgrade the requested role.

### 10.11 User interaction and state

#### FR-UI-1 — Trusted decisions (P1)

Consequential approvals must be selected and confirmed through the local Pi TUI,
not supplied as model-generated tool arguments. Print and JSON modes have no
trusted UI, and an unauthenticated RPC client response proves only client input,
not human approval. Those modes must fail closed unless a future trusted-client
contract authenticates the approving principal and binds approval to the exact
operation.

#### FR-UI-2 — Clear guard feedback (P1)

A blocked operation must explain:

- the stable machine-readable reason;
- the protected operation or boundary;
- the evidence or authorization that is missing; and
- the safe next action.

Messages must avoid exposing secrets, raw credentials, or unnecessary private
repository content.

#### FR-UI-3 — Durable versus session state (P1)

Authoritative workflow state must remain in its owning component or project
artifact. Extension-local session state may track ephemeral UI and invocation
context but must not become a second authority for Tiber, final review, delivery
mode, or repository identity.

### 10.12 Claude Code and Codex compatibility

#### FR-HAR-1 — Shared behavior compatibility (P0)

A change to shared skills or semantic policy must be evaluated against all
supported harnesses affected by that change.

#### FR-HAR-2 — Honest capability reporting (P0)

Documentation and status output must distinguish:

- shared instructional behavior;
- deterministic Pi enforcement;
- Claude Code hook enforcement;
- Codex hook enforcement; and
- behavior unavailable in a given harness.

The product must not claim parity where a harness lacks an interception or
trusted-UI capability.

#### FR-HAR-3 — Thin harness adapters (P0)

Claude Code and Codex manifests, hooks, and launchers must remain thin. They may
perform harness-specific input/output conversion but must delegate semantic
behavior to the canonical core or component.

#### FR-HAR-4 — Secondary surfaces remain release-gated (P0)

Making Pi primary does not permit known regressions in Claude Code or Codex.
Relevant compatibility checks remain blocking for a release that claims support
for those harnesses.

### 10.13 Evaluation support

#### FR-EVAL-1 — Pi provider integration (P0)

The repository eval runner must support Pi as a tested harness. If Promptfoo has
no suitable native Pi provider, the repository must provide a custom local
provider that invokes Pi through its documented JSON or SDK interface.

#### FR-EVAL-2 — Subscription model execution (P0)

The default Pi eval variant should use an OpenAI Codex model through Pi's
ChatGPT subscription authentication. Model and reasoning level must be
overridable without changing fixture semantics.

#### FR-EVAL-3 — Isolated eval home and auth ownership (P0)

Live Pi evals must use disposable, git-ignored configuration and session
locations through `PI_CODING_AGENT_DIR` or an equivalent SDK boundary. The auth
strategy must be validated against Pi credential refresh behavior before live
use. It must define one writer or serialized ownership for each mutable auth
store, preserve the source login unchanged and usable, and reconcile refreshed
credentials only through an explicit safe mechanism.

If authentication material is copied, the copy must be the minimum required,
use restrictive permissions, have a bounded lifetime, and never enter generated
artifacts or uploads. Secret-leak checks must cover setup, execution, retention,
and cleanup.

#### FR-EVAL-4 — Package modes (P0)

Pi behavior evals must support:

- a no-package baseline;
- a targeted development-system package mode; and
- a full Pi-compatible marketplace mode.

The full mode must report the packages actually loaded. Until other marketplace
plugins declare Pi support, it must not imply that Claude Code- or Codex-only
resources were installed.

#### FR-EVAL-5 — Shared behavior fixtures (P0)

Where semantics are shared, Pi must run the same natural behavior cases used by
Claude Code and Codex. Provider and harness metadata must remain separate so
results can be compared without conflating model and harness effects.

#### FR-EVAL-6 — Deterministic contract tests (P0)

Every extension behavior included in a release must have provider-free
black-box tests at its public extension, command, tool, or process boundary. The
P0 release must cover package and skill loading, startup diagnostics,
mode-appropriate status access, and reload/shutdown behavior.

As each P1 or P2 capability ships, its release must add the corresponding setup
preview and confirmation, coordination-checkout rejection, path
canonicalization, guarded tool composition, non-TUI fail-closed behavior,
child-process cleanup, RPC/direct-bash characterization, and delivery-mode
scenarios. A deferred guard does not create a P0 implementation obligation, but
it cannot ship later without its deterministic cases.

#### FR-EVAL-7 — Executable agent scenarios (P1)

Provider-backed evals for deterministic guards must use disposable repositories
and verify the actual outcome of attempted operations, not merely whether the
assistant says it would obey the rule.

#### FR-EVAL-8 — Canary (P0)

A provider-free canary must establish that the extension loads, exactly the
intended public skills are discoverable, no skill-name collision exists, and
required bundled component entry points can be resolved.

A provider-backed canary must establish that the selected Pi model can discover
and use representative package capabilities.

#### FR-EVAL-9 — Reporting priority (P0)

Generated dashboards and summaries must present Pi first, Claude Code second,
and Codex third while retaining explicit provider/model labels. Ordering must
not alter threshold calculation.

#### FR-EVAL-10 — Value gates (P0)

Pi package behavior must be compared with its no-package baseline. Deterministic
safety requirements must pass as hard guards; stochastic behavior must meet
case-specific pass-rate and value-lift thresholds.

#### FR-EVAL-11 — Live-eval authorization (P0)

Before the first provider-backed Pi execution, repository policy must explicitly
authorize use of Pi with the owner's OpenAI/ChatGPT subscription, or the runner
must obtain fresh approval. Existing authorization that names only Claude Code
and Codex must not be assumed to include Pi.

#### FR-EVAL-12 — Noninteractive trust and resource provenance (P0)

The eval runner must make project trust explicit. It may grant one-run trust only
to a repository-owned disposable fixture whose contents and package composition
are part of the eval contract, or it may load explicit package resources through
a path that does not depend on project trust. It must not rely on Pi's default
noninteractive `ask` posture.

The canary must prove that the expected extension code executed and record its
source provenance, not merely discover a similarly named skill or tool.
No-package mode must prove that the package extension and skills did not load.

### 10.14 Documentation and support posture

#### FR-DOC-1 — Primary-support catalog transition (P1)

Before the primary-support gate is met, documentation may identify Pi as the
primary target or preview but must not recommend it as the primary experience.
After every applicable P1 requirement enumerated in Section 9 has its required
evidence, the repository README and development-system README must list Pi first
and identify it as the primary recommended surface. Claude Code must be
described as secondary and Codex as tertiary.

#### FR-DOC-2 — Installation and trust (P0)

Documentation must explain local Pi installation, project trust, package update
behavior, extension code-execution privileges, configuration setup, and
uninstallation.

#### FR-DOC-3 — Capability matrix (P0)

Documentation must contain a maintained matrix showing which skills, tools,
agents, MCP components, hooks, and deterministic guards are available in each
harness.

#### FR-DOC-4 — Canonical-source guidance (P0)

Contributor documentation must identify canonical sources, generated artifacts,
regeneration commands, and validation commands.

## 11. Technical architecture

### 11.1 System context

The product consists of one package with several adapter boundaries:

```text
                         Canonical skills
                               │
              ┌────────────────┼────────────────┐
              │                │                │
          Pi package      Claude plugin     Codex plugin
              │                │                │
       Pi extension        hooks/MCP          hooks/MCP
              │                │                │
              └──────────── shared policy ─────┘
                               │
                 ┌─────────────┴─────────────┐
                 │                           │
        TypeScript capabilities      Retained Rust components
        setup/doctor/orchestration    Tiber/development-discipline
                 │                           │
        Pi / MCP / CLI adapters        MCP / CLI subprocesses
```

The architecture separates three concerns:

1. **Guidance:** shared skills and references consumed by models.
2. **Capabilities:** operations callable through Pi tools, MCP, or CLI.
3. **Enforcement:** harness lifecycle boundaries that can block or mediate an
   operation regardless of whether the model voluntarily invokes a capability.

### 11.2 Conceptual package layout

The implementation should converge on a shape similar to the following without
requiring these exact directory names:

```text
plugins/development-system/
├── package.json                  # Pi package and canonical release metadata
├── skills/                       # Canonical public Agent Skills
├── extensions/
│   └── development-system/
│       ├── index.ts              # Pi adapter composition root
│       ├── core/                 # Pure policy and semantic types
│       ├── capabilities/         # Shared TypeScript capability definitions
│       └── adapters/             # Pi, MCP, CLI and process adapters
├── components/
│   ├── development-discipline/   # Retained Rust coordinator and assets
│   ├── tiber/                    # Retained Rust task system
│   └── ...
├── agents/ or agent-sources/     # Canonical agent definitions
├── hooks/                        # Claude Code and Codex adapters
├── bin/                          # Thin portable launchers
├── .claude-plugin/
└── .codex-plugin/
```

The final layout may preserve existing paths to reduce migration risk. The
required property is the dependency direction: adapters depend on canonical
cores; canonical cores do not depend on harness APIs.

### 11.3 Functional core and imperative shells

Shared domain behavior must be pure. Filesystem access, Git execution, process
management, clock access, UI, network calls, and provider calls belong in
imperative adapters.

Where a domain decision requires external evidence, it must use the
repository's Step/Trampoline effect contract: a pure state machine exposes
`step()` and `resume(result)`, yields typed effects or waits for typed results,
and reaches a typed outcome. A thin imperative interpreter performs each effect
and resumes the core. This permits the same domain flow to run under Pi, MCP,
CLI, and tests without embedding I/O in business logic.

Illustrative effects include:

- resolve repository identity;
- read project configuration;
- inspect Git status;
- obtain final-review status;
- request trusted confirmation;
- execute an approved command.

The supporting library and internal representation are development decisions;
the Step/Trampoline `step()`/`resume()` boundary and pure/effectful dependency
direction are requirements.

### 11.4 Semantic types and errors

Boundary adapters parse raw Pi arguments, MCP JSON, CLI arguments, TOML values,
paths, Git output, and process responses into semantic types before invoking the
core.

Likely domain concepts include repository identity, checkout kind, project
root, delivery mode, enabled feature, protected operation, approval, evidence
scope, and harness mode. The exact type inventory should emerge from vertical
slices rather than be designed exhaustively in advance.

Errors are values with stable machine-readable identifiers and preserved source
chains. Adapters may add concise user guidance, but they must not convert every
failure into unstructured prose or report a policy rejection as an
infrastructure crash.

### 11.5 Shared TypeScript capability pattern

For a new lightweight operation with multiple consumers, one capability module
should own the semantic input, outcome, errors, and behavior. It may also carry
a stable identity, canonical schema, and presentation-neutral metadata when
those fields reduce duplication. Implementations may compose these modules in a
registry, but no central registry is required.

Only demonstrated consumers receive adapters:

- a Pi adapter registers native tools or commands;
- an MCP adapter presents discovery and calls when an MCP consumer exists;
- a CLI adapter maps argv/stdin and exit status when hooks, users, or automation
  need it;
- tests exercise public boundaries with controlled effect interpreters.

Pi-specific prompt snippets and UI rendering may live in the Pi adapter. They
must not be copied into the semantic implementation. The repository's
functional-core/imperative-shell and Step/Trampoline rules govern business
logic. The capability-specific semantic states and yielded effect types should
emerge from each vertical slice rather than from a speculative universal model.

### 11.6 Rust component integration

Tiber already has a library-oriented Rust workspace with CLI, MCP, Git, core,
and server crates. It remains behind its existing supported interfaces.

Development-discipline currently has a larger binary-oriented implementation.
The initial Pi integration should invoke its MCP server. A later refactor may
extract a Rust library to improve internal testability and adapter reuse, but
must preserve the external MCP contract during migration.

The Pi bridge acts as a client, not a semantic proxy. It is responsible for:

- locating the source-bound bundled executable;
- starting it only when needed;
- performing MCP initialization and tool discovery;
- registering an approved subset of tools;
- converting Pi cancellation and errors to protocol behavior;
- bounding output and cleaning up the process.

It is not responsible for duplicating Tiber or review state.

### 11.7 Pi lifecycle integration

The extension is expected to use Pi events according to responsibility:

- `session_start`: initialize bounded session resources and diagnostics;
- `resources_discover`: contribute explicit resource paths when package metadata
  alone is insufficient;
- `before_agent_start`: inject only current, task-relevant status or guidance;
- `tool_call`: inspect, mutate when safe, or block protected calls;
- `tool_result`: attach bounded structured evidence when required;
- `user_bash`: apply policy to user-entered shell execution where appropriate;
- `session_shutdown`: terminate resources and clear session state.

Commands should own workflows requiring session replacement, reload, or
interactive user control. Tools should expose model-callable operations with
strict schemas. A trusted human decision must never be represented as a freeform
model-selected enum merely because tools support enum schemas. In the initial
product, only the local TUI is a trusted approval channel; RPC extension UI is a
client protocol and is not human provenance by itself.

### 11.8 Tool activation and context budget

The extension may register all package-owned tools while initially activating
only those relevant to enabled features. Dynamic tool loading may be used where
it improves context and provider cache behavior.

Tool descriptions must state intent and critical boundaries without embedding
full workflow documentation. Skills and references remain the source for
detailed process guidance. Context size and trigger effectiveness must be
measured across Pi, Claude Code, and Codex.

### 11.9 Build and release architecture

TypeScript source may be loaded directly by Pi, but non-Pi adapters cannot
assume Pi's TypeScript loader. The release process may therefore compile
TypeScript for CLI or MCP execution.

The implementation must choose a reproducible strategy that:

- preserves TypeScript as the canonical source;
- does not require globally installed compilers at runtime;
- distinguishes source from generated JavaScript;
- checks generated output or builds it during packaging;
- works within the repository's Nix and local dependency conventions.

Whether compiled JavaScript is committed, built into release artifacts, or
produced during package installation is deliberately deferred until packaging
constraints are tested.

### 11.10 Metadata generation

One canonical package/release record should supply common fields such as name,
version, description, author, license, and keywords. Generators then emit or
validate:

- Pi package metadata;
- Claude Code plugin metadata;
- Codex plugin metadata;
- marketplace entries;
- README catalog versions where versions remain displayed.

Harness-specific fields stay in harness adapter templates or overlays. The
system must not erase fields required by only one harness.

### 11.11 Eval provider architecture

The likely Promptfoo Pi provider will execute Pi in JSON mode for process
isolation and stable event capture. The provider should:

1. create or receive an isolated Pi agent directory;
2. select a provider, model, and reasoning level;
3. derive the package composition from the canonical Pi support inventory;
4. establish explicit one-run trust for the repository-owned disposable fixture
   or load resources independently of project trust;
5. run without persistent sessions;
6. stream and parse JSON events;
7. capture the final assistant output, usage, tool trajectory, extension
   execution provenance, and diagnostics;
8. terminate the complete child process on timeout or cancellation; and
9. return a standard Promptfoo provider response.

The Pi SDK is an alternative if it materially improves reliability or
instrumentation. The choice should be made through a small canary rather than
assumed in advance.

Eval metadata must separate:

- harness (`pi`, `claude-code`, or `codex`);
- Pi/provider API identity;
- model identity;
- package mode;
- loaded package composition;
- deterministic guard outcome.

This prevents a model comparison from being misreported as a harness
comparison.

## 12. Security and trust model

### 12.1 Intended boundary

This is a local, single-owner development system. It trusts:

- the owner;
- the installed development-system package;
- the local repository and project configuration after Pi project trust;
- the local toolchain, `PATH`, and environment;
- bundled, checksum-verified component binaries;
- authenticated harness sessions managed by the owner.

### 12.2 In-scope failures

The product should protect against:

- ordinary model or user mistakes;
- stale configuration or evidence;
- writes in the wrong checkout;
- ambiguous repository scope;
- accidental secret-file mutation or model-visible disclosure through the
  explicitly mediated guarded tool composition;
- omitted workflow calls;
- cooperative concurrent worktrees or processes;
- interruption, timeout, and child-process leakage;
- malformed protocol responses;
- unavailable components;
- unauthorized or incorrectly scoped delivery operations;
- accidental mutation of source authentication state during evals.

### 12.3 Out-of-scope adversaries

Unless a future deployment declares a stronger boundary, blocking design does
not need to defeat:

- intentional owner bypass outside Pi;
- malicious root or same-UID processes;
- a compromised local runtime, shell, Git executable, or package installation;
- adversarial replacement of trusted bundled binaries after installation;
- a user intentionally disabling or modifying the extension.

These assumptions do not permit weak handling of ordinary failures or secrets.

### 12.4 Secret handling

The extension and eval provider must not log or return raw credentials. Populated
secret files must not be read merely to classify them. Claims about disclosure
prevention are limited to the guarded tool composition characterized by
FR-GRD-6; arbitrary unmediated shell or third-party execution is outside that
claim.

Eval-home preparation must follow the validated auth ownership and refresh
contract in FR-EVAL-3, use restrictive permissions and ignored storage, and
exclude every auth store from artifacts and uploads. Cleanup or bounded
retention must be explicit and testable.

## 13. Quality strategy

### 13.1 Test layers

1. **Pure domain tests:** policy decisions and state transitions with no I/O.
2. **Adapter contract tests:** Pi, MCP, CLI, configuration, and subprocess
   boundary behavior.
3. **Black-box component tests:** public commands and tools in temporary
   repositories.
4. **Package canaries:** resource loading, collisions, executable resolution,
   and mode behavior.
5. **Provider-backed evals:** natural task behavior and executable guard
   scenarios.
6. **Cross-harness compatibility:** shared semantics in Pi, Claude Code, and
   Codex.

Tests should assert user-observable behavior and effects rather than inspect
committed documentation or copy expected policy text from source files.

### 13.2 Required validation categories

A release must include fresh evidence for all changed surfaces, including as
applicable:

- TypeScript formatting, linting, type checking, and tests;
- Rust tests for changed Rust components;
- shell/Bats tests for retained launchers and hooks;
- manifest and generated-artifact synchronization;
- Pi package canary;
- relevant provider-backed behavior evals;
- secret scans around live provider execution;
- full repository CI.

### 13.3 Eval measurement

Population quality should be improved primarily by adding distinct,
representative cases. Repeated samples should be used only for a named metric
such as per-input reliability, pass-at-k capability, pass-to-the-k reliability,
judge variance, or close A/B comparison.

Hard deterministic guard failures must not be averaged away by stochastic pass
rates.

## 14. Success metrics

### 14.1 Adoption and usability

- A clean local checkout can install and load the Pi package using documented
  steps.
- Package startup reports no unexpected skill collisions or extension errors.
- The user can determine configuration, checkout, feature, and component status
  through one documented, mode-appropriate deterministic status surface.
- The user can complete setup through preview and trusted confirmation without
  manually constructing an apply command.

### 14.2 Maintainability

- Shared public skills have one physical source each.
- Shared TypeScript capabilities have one implementation and one input contract
  across Pi and MCP.
- Agent adapter files are generated or otherwise proven to derive from one
  canonical source.
- CI detects metadata and generated-artifact drift.
- No initial-release work rewrites Tiber or development-discipline behavior.

### 14.3 Effectiveness

- Pi targeted-package behavior meets each fixture's configured pass-rate and
  value gate relative to the Pi no-package baseline.
- Every released deterministic guard passes executable success, rejection,
  ambiguity, and noninteractive scenarios.
- Pi package canaries pass without a provider call, and provider-backed canaries
  prove representative capability use.
- Claude Code and Codex retain passing relevant behavior and compatibility
  checks.

### 14.4 Context and performance

- Always-loaded skill and tool context does not increase without measured,
  proportionate effectiveness.
- Local adapter overhead remains negligible relative to model operations and
  does not create noticeable interactive latency for ordinary commands.
- Child processes terminate within defined operational bounds after completion,
  cancellation, or session shutdown.

Exact numeric latency and context budgets should be established from the first
working canary rather than guessed in this PRD.

## 15. Rollout plan

### Phase 0 — Characterization and architecture record

- Record the accepted package, adapter, language, and canonical-source decisions
  in an ADR.
- Characterize current Claude Code and Codex behavior.
- Build a Pi resource-loading spike that records skill collisions and package
  diagnostics.
- Establish a no-package Pi behavior baseline.

**Exit condition:** the package boundary and public resource set are proven, and
no implementation depends on accidental recursive discovery.

### Phase 1 — Minimal Pi package

- Add canonical Pi package metadata and the machine-readable Pi support
  inventory.
- Add the clean-checkout bootstrap and supported-target checks.
- Load the eight public skills.
- Add a minimal extension with startup diagnostics and mode-appropriate status.
- Add provider-free package canaries and documentation.

**Exit condition:** local installation is supported on the declared target
matrix after exactly the documented bootstrap, and it does not alter Claude Code
or Codex behavior.

### Phase 2 — Pi eval integration

- Add the Pi Promptfoo provider or SDK integration.
- Add isolated eval homes and auth handling.
- Add Pi variants and package modes to config generation, thresholds, metadata,
  and dashboards.
- Obtain explicit live-eval authorization before provider-backed execution.

**Exit condition:** Pi baseline and targeted-package behavior are visible beside
Claude Code and Codex results.

### Phase 3 — Shared TypeScript setup capabilities

- Introduce the TypeScript capability/core pattern.
- Implement status, doctor, and setup through shared definitions.
- Keep or replace shell entry points with thin compatible launchers.
- Add Pi trusted confirmation and noninteractive fail-closed behavior.

**Exit condition:** setup behavior is shared across adapters and executable
black-box tests prove parity.

### Phase 4 — Retained component bridge

- Add the supervised first-party MCP bridge and its tool-admission contract.
- Expose enabled Tiber and development-discipline tools to Pi.
- Consolidate existing Claude Code and Codex agent definitions into one
  canonical source with generated or validated adapters.
- Validate process lifecycle, errors, cancellation, collision handling, schema
  admission, and context footprint.

**Exit condition:** Pi can use authoritative task and final-review components
without duplicated schemas or state machines, and existing agent definitions no
longer require semantic double maintenance.

### Phase 5 — High-value deterministic guards and review execution

- Add structured write/worktree/path protections, including closure over the
  default `bash` mutation path.
- Add delivery-mode, destructive-operation, and CI-hold guards only as their
  command contracts and executable cases are proven.
- Add the minimum fresh-session executor needed to run every authoritative
  final-review assignment with model-role attestation, child isolation, and
  fail-closed result handling.
- Validate every supported Pi mode for bypasses.

**Exit condition:** each shipped guard has an observed failure mode, a public
behavior test, and provider-backed executable evidence where model behavior is
involved; Pi can complete the authoritative final-review workflow without
retaining reviewer context or inventing a pass.

### Phase 6 — Primary-support transition

- Verify every applicable P1 requirement enumerated in Section 9 and its
  FR-EVAL-6 scenarios.
- Put Pi first in user and contributor documentation.
- Publish the capability matrix.
- Decide whether local-only or npm distribution is ready.
- Review support burden and prune low-value compatibility or extension surface.

**Exit condition:** Pi is the documented primary recommended experience, every
applicable P1 requirement has executable evidence, and all three claimed harness
surfaces meet their release gates.

### Phase 7 — Post-primary review experience improvements

- Improve progress rendering, assignment batching, and reviewer-result
  inspection without changing coordinator authority.
- Optimize model routing or child-session startup only when eval and latency
  evidence supports the change.

**Exit condition:** optional UX or performance improvements preserve the P1
fresh-context, isolation, attestation, and fail-closed contracts. This phase does
not delay the primary-support transition.

## 16. Risks and mitigations

### 16.1 Overengineering guardrails before proving value

**Risk:** The project repeats the reverted SDLC experiment's high implementation
and operating cost.

**Mitigation:** Start with package/eval characterization, admit guards one
vertical behavior at a time, and require a concrete failure mode and value test.
Do not introduce a new event history or comprehensive shell policy as an
initial foundation.

### 16.2 TypeScript rewrite regressions

**Risk:** Language enthusiasm causes mature Rust behavior to be rewritten and
subtly weakened.

**Mitigation:** Adopt TypeScript by default only for new orchestration and small
components. Require a separate decision and parity evidence for each migration.
Bridge retained components through their protocols.

### 16.3 False cross-harness parity

**Risk:** Documentation implies that skills or MCP alone enforce behavior in
Claude Code and Codex as strongly as Pi events do.

**Mitigation:** Maintain an explicit capability matrix and classify every rule
as instructional, tool-mediated, hook-enforced, or unavailable per harness.

### 16.4 Tool and skill context growth

**Risk:** Bridging all component tools degrades model performance and prompt
cache behavior.

**Mitigation:** Activate only feature-relevant tools, use concise descriptions,
progressively disclose specialist guidance, and measure context against eval
lift.

### 16.5 Shell-classification bypasses

**Risk:** A simplistic parser misses equivalent Git or deployment commands or
blocks ordinary work.

**Mitigation:** Prefer structured tool interception, define a deliberately
bounded protected command contract, test ambiguity and alternate forms, and
fail closed only after a command has been classified as potentially protected.

### 16.6 Authentication leakage in evals

**Risk:** Disposable Pi homes copy or expose OAuth state incorrectly.

**Mitigation:** validate Pi refresh semantics first; isolate each run through
`PI_CODING_AGENT_DIR`; serialize mutable auth ownership; copy only minimum auth
state when copying is necessary; prove the source login remains unchanged and
usable; scan artifacts; and never upload auth-bearing directories.

### 16.7 Adapter drift

**Risk:** Claude Code, Codex, Pi, generated agents, or manifests diverge.

**Mitigation:** canonical source declarations, reproducible generation,
cross-harness contract tests, and CI drift checks.

### 16.8 Pi API evolution

**Risk:** Extension event or package behavior changes in a newer Pi release.

**Mitigation:** declare a tested Pi compatibility range, use documented APIs,
run package canaries against the supported version, and isolate Pi-specific code
in its adapter.

## 17. Dependencies and constraints

- Pi package, skill, extension, JSON mode or SDK, and project-trust APIs.
- Node.js and the repository Nix development environment.
- TypeBox/JSON Schema compatibility for shared TypeScript tools.
- Existing development-discipline and Tiber executables and MCP contracts.
- Existing Claude Code and Codex marketplace/plugin formats.
- Promptfoo custom-provider support unless a native Pi provider becomes
  available.
- Existing authenticated harness sessions for approved live evals.
- Repository conventions that global npm-style dependencies live under the
  ignored `.dependencies/` prefix.

## 18. Open questions and deferred decisions

The following should be answered with implementation evidence rather than fixed
prematurely:

1. Should the Pi package initially be represented by a nested plugin
   `package.json`, a root package manifest that points into the plugin, or both
   for different distribution modes?
2. What npm package name and release channel should be used if public
   distribution is approved?
3. Should compiled TypeScript be committed, built during release, or produced
   during package installation?
4. Is Pi JSON mode or the Pi SDK the more reliable Promptfoo provider boundary?
5. Which exact bundled MCP tools should be initially exposed, and which should
   remain lazily activated?
6. What authenticated RPC-client and approval-binding contract, if any, would
   justify enabling consequential human approvals outside the local TUI?
7. What is the smallest robust shell-command classifier needed for the first
   delivery guard?
8. Which populated-secret path rules should be default versus project-defined?
9. Which project configuration section should hold Pi model-role mappings?
10. Which existing Claude Markdown or Codex TOML agent format should serve as
    the initial canonical source, or should a neutral schema generate both?
11. What concrete context and latency budgets are justified by the first Pi
    baseline?
12. Which guards create sufficient measured value to progress from warning to
    blocking behavior?
13. When, if ever, does the maintenance cost of a retained Rust component
    justify a language migration?

Each resolved architectural question that materially constrains future work
must be recorded in an ADR.

## 19. Release acceptance criteria

The first supported Pi release is acceptable when all of the following are true:

1. `plugins/development-system/` installs as a trusted local Pi package from a
   clean checkout after exactly the documented bootstrap.
2. The extension and exactly eight public skills load without unexpected
   diagnostics or name collisions.
3. Startup diagnostics and a documented deterministic status surface work in
   every claimed Pi mode; no mode relies on stochastic model invocation to
   obtain status.
4. Claude Code and Codex consume unchanged canonical public skill files.
5. One canonical release version is validated across all package and
   marketplace metadata.
6. The Pi support inventory identifies the exact package resources, and
   provider-free canaries prove extension execution provenance, project-trust
   posture, package loading, and component resolution.
7. Every claimed Linux and macOS target has verified Tiber and
   development-discipline artifacts; unsupported targets fail deterministically
   without relying on Cargo fallback.
8. Promptfoo can execute Pi no-package and targeted-package variants in an
   isolated eval environment.
9. The eval dashboard reports Pi first and records the actual package
   composition, provider, and model.
10. Relevant Pi behavior meets configured thresholds and hard guards.
11. Existing relevant Claude Code and Codex validation remains green.
12. Documentation explains installation, trust, target support priority,
    capability differences, and canonical-source maintenance without calling Pi
    the primary recommended experience before its P1 gate.
13. The package declares a tested Pi compatibility range, and canary evidence
    records the exact Pi version used for release.
14. No mature Rust component has been rewritten merely to satisfy the initial
    release.

Pi becomes the primary recommended experience only after every applicable P1
requirement enumerated in Section 9 has the evidence class specified there. This
includes FR-EVAL-6 deterministic contracts, FR-EVAL-7 provider-backed execution
where model behavior is involved, and FR-EVAL-10 stochastic value gates where
applicable. The P1 fresh-context review executor, isolation, model-role
attestation, and fail-closed behavior cannot be deferred; only the optional P2
review UX and performance improvements in Phase 7 may follow the primary
transition.
