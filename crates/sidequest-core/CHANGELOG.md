# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://git.johnwilger.com/Slipstream/ai-plugins/releases/tag/sidequest-core-v0.1.0) - 2026-06-30

### Added

- *(harness)* opt-in cross-harness spawn gate (slice 8)
- *(steer)* ask/answer hive-mind protocol (slice 4)
- *(list)* registry + list tool (slice 3)
- *(core)* sidequest.toml config parser
- *(core)* Goal and BranchName semantic types for launching

### Other

- close workspace mutation gaps in config, registry, and tracing
- *(sidequest-core)* cover BranchName ref-safety, killing is_ref_safe mutants
- scaffold the sidequest two-crate workspace
