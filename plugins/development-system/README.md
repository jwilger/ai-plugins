# Development System

Development System is a configurable engineering workflow for **Codex
(primary)** and **Claude Code (supported)**. It combines shared skills, Beads
and Dolt task coordination, worktree guidance, final-review support, and
optional agentic-system and eval-reporting capabilities without creating a
separate task tracker or goal runtime.

In shell examples below, `development-system` means the plugin executable. If a
harness does not expose it on `PATH`, use
`<plugin-root>/bin/development-system` instead.

## Install

### Codex

Install from the GitHub marketplace source with:

```shell
codex plugin marketplace add jwilger/ai-plugins
codex plugin add development-system@ai-plugins
```

For a local checkout, use its marketplace root instead, for example
`codex plugin marketplace add ./ai-plugins`. Codex reads this plugin's
`.codex-plugin/plugin.json`, hooks, MCP configuration, and skills. Start a new
thread after installation so those resources load for the target repository.
Then open `/hooks`, inspect, and explicitly trust the current Development
System hook definition. Repeat that review after a hook-definition or plugin
update. Do not use `--dangerously-bypass-hook-trust` merely to skip the review.

### Claude Code

From Claude Code, add the marketplace and install the plugin:

```text
/plugin marketplace add jwilger/ai-plugins
/plugin install development-system@ai-plugins
```

For a local checkout, replace the marketplace source with the local path. See
the repository [README](../../README.md) for the shared installation and
integration overview.

## Configure a project

Setup is an explicit repository mutation. Start from the primary checkout and
request a preview before approval:

```shell
development-system setup --project <repo> --preset personal-trunk --dry-run
```

After the owner reviews the preview, repeat with `--apply --yes` to write the
policy and initialize enabled capabilities. The default is direct-to-trunk
delivery with `worktrees` and `beads` enabled. Configuration lives in
`.development-system.toml`.

```shell
development-system setup --project <repo> --preset personal-trunk \
  --delivery direct-to-trunk \
  --enable worktrees --enable beads
```

Available delivery modes are `direct-to-trunk`, `pull-request`, and
`local-only`. Optional capabilities are `agentic-systems` and
`eval-case-reporting`.

When Beads is enabled, setup handles the plugin's supported `bd` dependency
only after explicit approval. It uses Beads' embedded Dolt store by default, so
a standalone `dolt` CLI is unnecessary unless the owner deliberately chooses
server-mode or direct database administration.

## Work types, tickets, goals, and worktrees

Questions and read-only investigation require neither a ticket nor a worktree.
Do not create workflow state just to answer, inspect, explain, or review.

For planned mutable delivery work, Beads is the sole task and workflow
authority:

1. Find ready work deterministically and claim it atomically.
2. Pour and follow the delivery formula selected by `[beads].workflow`.
3. Classify each slice as behavior, documentation, CI-workflow, or
   validation-only, and collect the causal evidence for that slice.
4. Record durable delivery evidence in Beads before closing the relevant work.

Goals are native to the active harness. Development System does not add an
independent goal command, session store, or automatic continuation loop. Use
the harness' own goal handling and create Beads work only when the task needs
durable mutable-delivery coordination.

One mutable ticket may use its current checkout. Create a linked worktree only
when separate mutable tickets are active concurrently across sessions, agents,
or subagents. Do not create a worktree for ordinary read-only work, and do not
create nested worktrees.

The active harness owns worktree creation, switching, and cleanup. This
repository still supplies the worktree bootstrap and teardown boundary: its
post-checkout hook bootstraps a linked checkout, and a clean, no-longer-needed
checkout can be prepared with `just worktree-teardown <path>` before the owner
or harness removes it. Cleanup is housekeeping, not a delivery condition; never
force removal of a dirty, detached, identity-changed, or valuable worktree.

## Optional integrations

Use the integration guide before discussing any user-scoped installation:

```shell
development-system integrations --harness <all|codex|claude>
```

The command is read-only. It reports the integration contract and owner
handoff; it does not execute installers, modify harness settings, or read or
write credentials.

### Beads

Beads hooks are plugin-owned and run only when the target project explicitly
enables Beads. Do not add duplicate user hooks. Claude Code loads Beads context
at session start, while Codex uses the plugin-owned lifecycle hooks to preserve
and refresh it. After Codex installation or an update, inspect and explicitly
trust the current plugin hook definition through `/hooks`.

### Context7

Install Context7 only through the official marketplace commands for the
selected harness, after the owner reviews and approves the adjunct plugin. No
Context7 key is bundled or written by Development System. If the chosen Context7
configuration requires a key, the owner supplies `CONTEXT7_API_KEY` privately
through their environment or the official configuration flow.

### Hindsight

For Claude Code, the owner may install Hindsight through its official
marketplace listing. For Codex, Hindsight requires its official interactive
installer or manual configuration because the owner must choose the storage
mode and configure credentials. Upstream publishes it as
`curl -fsSL https://hindsight.vectorize.io/get-codex | bash`, but do not execute
that direct pipeline: fetch it to a temporary file, inspect it, and separately
approve its execution. An agent must not run the installer on the owner's
behalf.

The Codex installer overwrites `~/.codex/hooks.json`, and its `--uninstall`
mode deletes that file. Before installing it, back up any existing hooks file,
then review the generated replacement and merge the Hindsight hooks with
backed-up non-Beads shared hooks into a combined file. Omit legacy `bd`/Beads
lifecycle entries: Development System is their sole owner. Review and
explicitly trust the resulting Hindsight user-hook definitions through
`/hooks`. Do not run Hindsight's uninstaller when the file contains Beads or
other shared hooks; preserve the backup and manually remove only the Hindsight
entries instead.

Before installing either Hindsight integration, make an explicit decision about
what session material may be retained, the memory-bank scope, retention policy,
and any cloud-versus-local storage choice. Development System embeds no
Hindsight key, token, configuration, retention setting, or memory bank.

## Diagnostics

The plugin's compatibility check is scoped to the selected harness:

```shell
development-system doctor --project <repo> --harness <all|codex|claude>
```

It can identify conflicting enabled plugins, incompatible hook settings, and
user-managed MCP configuration that needs review. It does not silently alter
those settings.

## Advisor, content authoring, and capability routing

Advisor provides read-only challenge and planning support for fuzzy tradeoffs,
scope, specifications, and ticket plans. It also runs proactively before a plan
with two or more dependent implementation steps is finalized, unless an
existing executable BDD-style scenario already covers both observable behavior
and its material failure boundary.

Codex selects Advisor's model at dispatch time from authoritative
harness-advertised eligibility and capability or upgrade metadata. The agent
file deliberately omits a fixed model identifier, while retaining read-only
execution and `xhigh` reasoning. If the harness cannot rank eligible models,
select the highest-capability route explicitly, or launch it, Advisor fails
visibly instead of guessing or falling back.

Claude Code uses its moving `opus` alias with the highest supported effort. In
both harnesses, stronger model selection changes capability only; it grants no
additional authority.

Substantive human-consumable prose, documentation, instructions, imagery, and
UI/UX route through the public `content-authoring` skill. It delegates artifact
creation to the highest-capability eligible writable agent at high effort,
selected explicitly from authoritative current-harness capability or upgrade
metadata rather than a pinned model or guesses based on names, list order,
price, or date. Routine status updates, ticket metadata, commit messages, and
mechanical summaries are excluded. The author may use an authorized specialized
image tool when appropriate, but model and tool selection grant no additional
authority. If the required ranking, selection, high-effort writable launch, or
specialized tool is unavailable, the route blocks visibly instead of silently
falling back.

## Plan sharpening

`sharpen-plan` improves exactly one load-bearing assumption or plan-level
specification per pass without pre-deciding implementation details. It asks one
harness-native question only for a genuine user decision, applies the answer to
the active plan state, and re-presents the complete plan through the native
approval flow. Another pass begins only after approval; the skill stops plainly
at diminishing returns. In read-only Plan Mode it updates conversational plan
state without writing files, and it never invokes Advisor recursively.

## Included surfaces

The shared public skills are:

- `advisor`
- `agentic-systems`
- `beads`
- `content-authoring`
- `delivery`
- `development-workflow`
- `engineering-standards`
- `eval-case-reporting`
- `sharpen-plan`
- `setup`
- `worktrees`

The plugin also carries the development-discipline review component,
agentic-systems-engineering, eval-case reporting, delivery formulas, and the
Codex and Claude Code adapter metadata needed to expose those surfaces.

## Contributor sources and validation

The Codex plugin manifest is the canonical version source. Run
`node scripts/sync-development-system-metadata.mjs --write` after a required
version bump to synchronize the Claude manifest, marketplace entries, cache
launchers, and catalog row. Validate the resulting marketplace and Markdown
before delivery:

```shell
node scripts/sync-development-system-metadata.mjs --check
prettier --check "**/*.{json,md}"
just ci
```
