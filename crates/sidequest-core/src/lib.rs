//! Pure functional core for the sidequest control plane.
//!
//! This crate is **side-effect-free by construction**: its `Cargo.toml`
//! declares no I/O dependencies, so it can only *describe* effects (via the
//! effect / step types) for the imperative shell to interpret — it can never
//! perform them. See `docs/adr/0001` and the workspace `AGENTS.md`.
//!
//! Domain types, effects, and state machines are added outside-in as the
//! behavioral slices require them.

pub mod config;
pub mod harness;
pub mod launch;
pub mod side_quest;
