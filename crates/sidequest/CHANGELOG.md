# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://git.johnwilger.com/Slipstream/ai-plugins/compare/sidequest-v0.0.1...sidequest-v0.1.0) - 2026-06-30

### Added

- *(sidequest)* PR/MR delivery mode + harden delivery verification
- *(harness)* opt-in cross-harness spawn gate (slice 8)
- *(deliver)* push-origin delivery mode (slice 6)
- *(steer)* ask/answer hive-mind protocol (slice 4)
- *(background)* detached worker makes launch non-blocking (slices 4/5)
- *(list)* registry + list tool (slice 3)
- *(config)* drive delivery from sidequest.toml
- *(deliver)* deliver a side-quest's work to local main
- *(session)* run the goal session inside the worktree
- *(launch)* launch tool creates an isolated worktree
- *(mcp)* connectable sidequest MCP stdio server

### Other

- close workspace mutation gaps in config, registry, and tracing
- *(mcp)* acceptance test — a harness connects and reads identity
- scaffold the sidequest two-crate workspace
