# Change conventions

- Functional core/imperative shell; semantic types and parse-don't-validate; explicit railway-style errors. Stack-specific lint policy lives in checked-in configuration; suppressions need documented policy reasons.
- Black-box behavior tests. RED applies to changed first-party behavior when no existing failing test proves it. Do not add tests that assert committed documentation/configuration text.
- One observable behavior step through passing fast gate and commit; expensive acceptance/mutation/browser/shell gates belong to CI except causal diagnosis.
- Conventional Commit subject plus rationale-bearing body; no `Co-Authored-By` or other AI-attribution trailers.
- Kebab-case plugin/component names. JSON uses two-space indentation; format changed JSON/Markdown with Prettier.
- Plugin behavior/docs/metadata changes need semver bump in the same PR, synchronized with marketplace via the official script.
- Preserve independent review and configured delivery gates. Use validated checkpoint publication rather than manually replacing authoritative records.
