# Functional core, imperative shell

All business logic is **pure** (the functional core): no I/O, no side effects.
All I/O and side effects live in the **imperative shell** at the edges.

Side effects the core needs are expressed with a **Step/Trampoline effect
pattern**: a pure state machine exposes `step()` / `resume(result)` returning
`Yield(effect)` / `WaitForResult` / `Done(outcome)`; a thin shell loop performs
the real I/O and feeds results back via `resume`. The core only ever _describes_
effects — it never performs them.

Enforce this with the strongest mechanism the stack supports: module boundaries,
package boundaries, dependency rules, tests, or lints. Keep core code free of
direct filesystem, process, network, database, and clock access.
