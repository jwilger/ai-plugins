---
name: setup
description: >-
  Use when initializing or reconfiguring a repository with the development-system
  plugin, including delivery mode, worktrees, Beads with Dolt, workflow formulas,
  optional capabilities, and conflict checks. Run initialization only from the
  primary checkout with `<plugin-root>/bin/development-system setup --project
  <repo> --preset personal-trunk --dry-run`; review its preview and obtain explicit
  approval before replacing the dry-run with `--apply --yes`. Also reconcile
  enabled-but-unavailable managed tools even when repository policy is unchanged.
---

# Development system setup

Setup is an explicit repository mutation and must run from the primary Git
checkout. Never require or permit a linked worktree for this operation.

Use the plugin-owned command:

```shell
<plugin-root>/bin/development-system setup --project <repo> --preset personal-trunk --dry-run
```

Show the preview and ask for explicit approval before replacing `--dry-run`
with `--apply --yes`. Ask customization questions one at a time.

**Enabled-but-unavailable Beads:** `development_system.setup_no_changes` means
the project policy needs no rewrite; it does not skip reconciliation of an
enabled `beads: unavailable` dependency. Reopen the explicit managed-tool offer
and keep any tools-only outcome separate from repository configuration or a
repository commit. Follow the complete contract in [Managed tools and no-op
reconciliation](#managed-tools-and-no-op-reconciliation).

The default is `--delivery direct-to-trunk` with `worktrees` and `beads`
enabled. Worktree-capable repositories use `mode = "concurrent-tickets"` in
their generated `[worktrees]` policy. Select features with repeatable options:

```shell
--delivery direct-to-trunk|pull-request|local-only
--enable worktrees|beads|agentic-systems|eval-case-reporting
--disable worktrees|beads|agentic-systems|eval-case-reporting
```

## Harness integrations

Repository setup and harness integrations are separate operations. Before
offering optional, user-scoped integrations, print the read-only contract for
the selected harness:

```shell
<plugin-root>/bin/development-system integrations --harness claude|codex
```

This command only reports the handoff. It never downloads or executes an
installer, changes harness configuration, reads credentials, writes credentials,
or creates a memory store. Get explicit approval for each user-scoped action
before the owner runs it.

Beads hooks are already owned by this plugin and are inert unless the target
project explicitly enables Beads. Do not add duplicate user hooks:

- Claude Code invokes the plugin's gated Beads launcher at `SessionStart`.
- Codex invokes the plugin's gated Beads launcher at `SessionStart`,
  `PreCompact`, `PostCompact`, and `UserPromptSubmit`.

Codex plugin hooks are non-managed. After installation, start or restart Codex,
open `/hooks`, inspect and explicitly trust the current Development System hook
definition, and repeat that review after a hook-definition or plugin update.
If the owner later installs or merges Hindsight hooks, review and trust those
user-hook definitions there too. Do not use `--dangerously-bypass-hook-trust`
merely to skip review.

For Context7, the owner can install the official adjunct plugin after approval:

```shell
# Claude Code
claude plugin marketplace add upstash/context7
claude plugin install context7@context7-marketplace

# Codex
codex plugin marketplace add upstash/context7
codex plugin add context7@context7-marketplace
```

Any Context7 API key is owner-provided as `CONTEXT7_API_KEY` in user environment
or harness configuration. Never add it to repository files or plugin
configuration.

For Hindsight, make retention and authentication choices explicit. Its Claude
Code plugin is installed from the official marketplace:

```shell
claude plugin marketplace add vectorize-io/hindsight
claude plugin install hindsight-memory
```

Tell the owner that Hindsight can retain session material. They must choose an
appropriate memory scope and retention policy, and keep any token in its
private Hindsight configuration rather than in the repository or chat.

Codex Hindsight setup uses an official interactive installer because the owner
must choose Hindsight Cloud or local memory and provide credentials privately.
Upstream publishes this direct-pipeline command:

```shell
curl -fsSL https://hindsight.vectorize.io/get-codex | bash
```

Treat that as upstream reference, not as a command to copy: a direct pipeline
cannot be inspected before execution. This is a manual owner handoff, not an
agent action. The owner must fetch the installer to a temporary file, inspect
it, and only then separately approve its execution in their own terminal:

```shell
installer_path="$(mktemp)"
curl -fsSL https://hindsight.vectorize.io/get-codex -o "$installer_path"
${PAGER:-less} "$installer_path"
# Only after independent review and approval:
bash "$installer_path"
```

Do not run the remote, version-unpinned pipeline from development-system or an
agent shell, and never capture, echo, or persist its credentials.

The Codex installer overwrites `~/.codex/hooks.json`, and its `--uninstall`
mode deletes that file. If a hooks file already exists, the owner must make a
backup before executing the inspected installer. Afterwards, review the
installer-generated replacement and merge Hindsight's entries with backed-up
non-Beads shared hooks into a combined file. Omit legacy `bd`/Beads lifecycle
entries: Development System is their sole owner. Review and explicitly trust
the resulting Hindsight user-hook definitions through `/hooks`. Do not run the
Hindsight uninstaller when that file has Beads or other shared hooks; preserve
the backup and manually remove only Hindsight's entries instead. Development
System does not perform any of those changes.

## Managed tools and no-op reconciliation

Enabled capabilities reconcile every external binary declared for them in
`bin/tool-releases.json`; Git and `tar` remain host prerequisites rather than
managed tools. For Beads, the current managed dependency is `bd >= 1.1.2`.
Beads uses its embedded Dolt engine by default, so do not require or install a
standalone `dolt` CLI unless the user explicitly chooses server mode or direct
database administration.

In setup previews, show each missing, invalid, or outdated
managed tool with its current status, manifest target, `~/.local/bin`
destination, user-global scope, and no-sudo guarantee. A user must
explicitly approve installation or update. Approval downloads only the pinned
HTTPS release, verifies SHA-256, installs atomically, verifies the resulting
version, and preserves a previous working executable on failure. Declining
keeps the capability unavailable; retry with
`development-system setup --enable beads`.

Always reconcile enabled dependencies even when the proposed TOML is bytewise
unchanged. For the `setup_no_changes` plus `beads: unavailable` regression,
explain the complete observable contract, not only the command:

- confirmation lists each tool's missing/outdated current state and manifest
  target version, `~/.local/bin`, user-global scope, and no-sudo requirement;
- approval installs the pinned, checksum-verified executable atomically;
- a tools-only result reports unchanged configuration separately and creates no
  repository commit;
- decline keeps Beads unavailable, and the same setup command reopens the offer;
- embedded mode needs no standalone Dolt CLI.

Make the user-global directory available immediately to extension child
processes. If it was absent from the inherited `PATH`, report the exact
`export PATH="$HOME/.local/bin:$PATH"` action and shell-restart requirement
rather than claiming complete shell integration. Setup then initializes
embedded Dolt storage without competing Git or harness hooks, sets Dolt
auto-commit, and installs formulas under `.beads/formulas/`.
The delivery mode selects `development-change-direct`,
`development-change-pr`, or `development-change-local` in project policy.
