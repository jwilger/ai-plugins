# Semantic types — parse, don't validate

Zero primitive-obsession. **Only semantic types** flow through the domain;
primitives, built-ins, and structural types appear **only at the I/O boundaries**
of the system. Parse external input into semantic types immediately at the
boundary, and never re-validate downstream — the type is the proof.

Use [`nutype`](https://docs.rs/nutype) for newtypes (`sanitize` / `validate`
predicates; choose inner types like `NonZeroU32` so the type system rejects
invalid states). `serde` may be derived on semantic types for ergonomic
(de)serialization, **but conversions happen only at I/O boundaries.**
