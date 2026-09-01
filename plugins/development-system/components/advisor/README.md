# Advisor

Advisor is a Codex-only plugin that packages the local `$advisor` skill and its read-only `advisor` agent for marketplace installation. The agent pins `gpt-5.6-sol` with high reasoning; if that model or custom agent is unavailable, delegation fails visibly instead of falling back to a weaker route.

Use it when planning work is still fuzzy: product/design/engineering tradeoffs, scope cuts, spec shaping, ticket planning, and "challenge this" requests. The skill delegates that work to a read-only advisor subagent so the main Codex thread can stay focused on the user's next decision or implementation step.

This component is part of the Codex-only marketplace.
