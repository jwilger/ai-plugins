# Error handling — railway-oriented, thiserror

Errors are values. Use **railway-oriented programming** (Scott Wlaschin): functions
return `Result`, errors propagate via `?`, and typed error enums convert upward via
`#[from]` / `From`.

- Derive `thiserror::Error` on every error enum; **never** hand-write `Display`.
- Error messages are **kebab-case, machine-readable identifiers** (e.g.
  `invalid-credentials`), not prose.
- Never `.to_string()` an error (it discards the source chain); convert with `From`
  and `Box::new(e)` to preserve the chain.
- No blanket `Result<T>` alias — write the explicit `Result<T, E>`.
