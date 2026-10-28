# Repository map and invariants

- Installable marketplace plugins live under `plugins/`; `plugins/development-system/components/` contains internal Development System components, not separately installable marketplace plugins.
- `.agents/plugins/marketplace.json` and each root `plugin.json` are the portable publication authority. Public skills wrap retained component contracts.
- `plugins/development-system/components/tiber/` contains the installed Tiber issue tracker.
- Repository-local delivery policy and current user direction govern delivery. A PR request does not authorize merging.
- Architecture and quality rules: `docs/adr/`, `docs/rules/`; retain unrelated changes and exact-revision evidence.
- Environment and dependency boundaries: `mem:tech_stack`. Canonical execution recipes: `mem:suggested_commands`. Design and change conventions: `mem:conventions`. Completion gates and delivery evidence: `mem:task_completion`.
- Development System component and binary boundaries: `mem:development_system/core`. Installed Tiber issue tracking and shared authority: `mem:tiber/core`.
