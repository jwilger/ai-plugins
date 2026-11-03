---
name: tasks
description: Compatibility entry point for Development System task coordination through the independently installed Tiber plugin.
---

# Tiber integration

Require `[features].tiber = true` in `.development-system.toml` for Development
System task integration. Invoke the installed `tiber:tasks` skill for task
operations and `tiber:new-task` for admission. Those skills own board policy,
publication recovery, signing, and CI-recovery authority.

If the companion is missing, explain how to install `tiber@ai-plugins`; do not
silently bypass task tracking, CI-recovery holds, or review receipts. Disabling
Development System integration does not disable standalone Tiber, which uses
`.tiber.toml` without requiring Development System configuration.
