# Final-review benchmark workspace

This disposable workspace intentionally has no development-discipline MCP
server. The benchmark checks that the `final-review` skill fails closed when
enforced-review authority is unavailable, even when the user requests a
one-pass or budget shortcut.

The scenario prompt permits only one workspace mutation:
`final-review-status.json`.
