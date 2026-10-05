# Repository map and invariants

- One installable plugin: `development-system`; `plugins/development-system/components/` contains internal components, not separately installable marketplace plugins.
- `.agents/plugins/marketplace.json` and each root `plugin.json` are the portable publication authority. Public skills wrap retained component contracts.
- `tiber/` is a standalone harness workspace, distinct from the bundled Tiber component.
- Repository-local delivery policy and current user direction govern delivery. A PR request does not authorize merging.
- Architecture and quality rules: `docs/adr/`, `docs/rules/`; retain unrelated changes and exact-revision evidence.
- Environment and dependency boundaries: `mem:tech_stack`. Canonical execution recipes: `mem:suggested_commands`. Design and change conventions: `mem:conventions`. Completion gates and delivery evidence: `mem:task_completion`.
- Development System component and binary boundaries: `mem:development_system/core`. The two Tiber implementations and shared authority: `mem:tiber/core`.
