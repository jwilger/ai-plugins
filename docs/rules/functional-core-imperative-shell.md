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

## EventCore modeling

For EventCore-based domain behavior, model the domain transition—not a generic
event envelope or persistence operation.

Do not model or describe EventCore in aggregate terms. Authority is expressed
by domain-intent commands whose own decision state is folded from their
declared streams.

- A `ModelCommand` names one domain intent, such as issuing an assignment,
  accepting RED evidence, recording a review result, or resolving a CI hold.
- That command's `ModelState` contains only the facts needed to decide that
  intent. Do not use an event log, whole-workflow snapshot, or unrelated
  projection as command state merely because it is convenient to replay.
- `evolve` folds every relevant event into that minimal state data. It must not
  perform I/O or reconstruct state through an external projection.
- `decide` validates the intent against that folded state and emits typed events
  that describe facts resulting from the successful state change—not commands,
  requests, mutable snapshots, or generic “state published” payloads.
- Persist a domain transition through EventCore `execute()`. Do not first
  mutate a projection, diff it into events, manually validate a caller-built
  event, or use a generic append/authority command as an alternate write path.
- Register every input, command, event mapping, and state field with EventCore's
  checked-model features, and run the complete model checker for every command
  lane. A passing checker for one command does not prove another command's
  state or event semantics.
- Treat `CheckStatus::Verified` as necessary but not sufficient. Default,
  absence, and constant state recipes are provenance roots, so a model can be
  technically complete while none of those origins or emitted event fields is
  consumed. The repository gate must reject every unconsumed-origin and
  unconsumed-event diagnostic and must connect command inputs, folded state,
  emitted facts, and typed output/view boundaries with executable mappings.
- The checked model proves registered data provenance; it does not prove the
  truth of handwritten branching inside `decide` or `evolve`. Cover every
  allowed and rejected domain transition with behavior tests executed through
  `eventcore::execute`, including concurrency/version-conflict behavior at the
  real EventStore boundary. Never describe a warning-free provenance graph as
  a formal proof of arbitrary Rust control flow.
- Legacy snapshots may be decoded only by an explicit, one-way import/replay
  boundary. A live command must never emit a legacy snapshot event, and a
  rebuildable projection, receipt cache, or crash journal must be documented as
  non-authoritative rather than mistaken for domain state.
