//! The sidequest control plane — the imperative shell.
//!
//! This crate owns all side effects (process supervision, git / worktree
//! operations, the flock'd registry, the forge port, notifications) and the
//! trampoline interpreter that runs the pure state machines from the
//! `sidequest-core` crate. Its two binaries are the MCP stdio server
//! (`sidequest-mcp`, the primary surface) and the `sidequest` CLI (a secondary
//! scripting / hook / debug surface).
//!
//! Behavior is added outside-in as the BDD slices require it.

pub mod config;
pub mod deliver;
pub mod quest;
pub mod registry;
pub mod server;
pub mod session;
pub mod worktree;
