# Advisor

Advisor is a component of the Codex-only Development System plugin. Its public `advisor` skill and read-only agent live at the plugin root for marketplace installation. The coordinator selects the task-local model and reasoning effort using Development Discipline's `model-routing` skill. Unavailable routes are reported visibly.

Use it when planning work is still fuzzy: product/design/engineering tradeoffs, scope cuts, spec shaping, ticket planning, and "challenge this" requests. The skill delegates that work to a read-only advisor subagent so the main Codex thread can stay focused on the user's next decision or implementation step.

This component is part of the Codex-only marketplace.
