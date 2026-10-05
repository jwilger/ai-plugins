# Tiber authority boundaries

- `tiber/` is the standalone harness; bundled services live under `plugins/development-system/components/tiber/`. Do not conflate their Cargo targets or launchers.
- Git/EventCore events are durable authority; projection/cache readback alone does not establish publication success. Preserve locking/CAS and reconcile ambiguous publication before retrying mutations.
- Repository board uses one active task, queued backlog in strict priority order. Causal defects blocking the active task stay within it rather than becoming fragmented tickets.
- Shared repository task/workflow authority remains on the Tiber stream across worktrees. Preserve unrelated task records and source changes.
