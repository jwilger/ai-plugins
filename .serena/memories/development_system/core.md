# Development System boundaries

- `plugins/development-system/` is the public portable plugin; root `mcp.json`, `plugin.json`, `bin/`, and public skills are synchronized publication surfaces.
- `components/development-discipline/` owns native workflow/review coordination and retained lifecycle contracts; public workflow/delivery skills delegate to them.
- `components/tiber/` owns bundled Git-backed task services; `components/worktrees/` owns reusable worktree scripts/templates. Agentic-system components provide routing/eval standards.
- Host-local binary installer is versioned and uses `PLUGIN_DATA`; distinguish checkout, built binaries, installed runtime and live harness uptake. Do not silently modify installed plugins during source development.
- Checkpoint publication must use the validated writer, immutable baseline, snapshot binding, locking and optimistic concurrency. Preserve real failure evidence and durable operation receipts.
