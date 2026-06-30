# Functional core, imperative shell

All business logic is **pure** (the functional core): no I/O, no side effects.
All I/O and side effects live in the **imperative shell** at the edges.

Side effects the core needs are expressed with a **Step/Trampoline effect
pattern**: a pure state machine exposes `step()` / `resume(result)` returning
`Yield(effect)` / `WaitForResult` / `Done(outcome)`; a thin shell loop performs
the real I/O and feeds results back via `resume`. The core only ever _describes_
effects — it never performs them.

This is **compiler-enforced**: the pure core lives in `crates/sidequest-core`,
whose `Cargo.toml` declares **no I/O dependencies** (no tokio, rmcp, git, http).
If core code tries to do I/O, it will not compile. Keep it that way — never add
an I/O dependency to `sidequest-core`.
