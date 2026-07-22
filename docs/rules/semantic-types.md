# Semantic types — parse, don't validate

Zero primitive-obsession. **Only semantic types** flow through the domain;
primitives, built-ins, and structural types appear **only at the I/O boundaries**
of the system. Parse external input into semantic types immediately at the
boundary, and never re-validate downstream — the type is the proof.

A renamed representation is not a semantic type. Raw strings, numbers,
booleans, UUIDs, other built-ins, structural records, and aliases such as
`type UserId = string` carry no proof that a domain invariant holds. Use a named
wrapper whose constructor is private and whose parser or smart constructor is
the only way to obtain a value. Model mutually exclusive domain alternatives
with a closed sum type so invalid combinations are unrepresentable. Parse once
at the I/O boundary; domain functions accept the resulting wrapper or sum and
do not repeat its validation.

For example, parse request text into a privately constructed `UserId` that
guarantees the required identifier format, and represent an exclusive contact
choice as `EmailContact(Email) | PhoneContact(PhoneNumber)`. Do not expose an
unchecked `UserId(string)` constructor or replace the contact choice with a
record containing independently optional `email` and `phone` fields.

Use [`nutype`](https://docs.rs/nutype) for newtypes (`sanitize` / `validate`
predicates; choose inner types like `NonZeroU32` so the type system rejects
invalid states). `serde` may be derived on semantic types for ergonomic
(de)serialization, **but conversions happen only at I/O boundaries.**
