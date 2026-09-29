# ADR-0016: Use portable plugin-owned MCP packaging

## Status

Accepted; supersedes the packaging restriction in ADR-0014 and amends the MCP launch path in ADR-0015

## Date

2026-09-29

## Context

Codex now supports the [Agent Plugins 1.0.0 package format](https://agent-plugins.org/specification), including root `plugin.json` and `mcp.json`. Development System instead registered its two MCPs in each consuming repository. That made plugin upgrades depend on project setup and duplicated server ownership. Codex remains the required client and Linux x86_64 the required binary target.

## Decision

Use root `plugin.json` as the sole plugin version authority and root `mcp.json` for both bundled MCP servers. Keep hooks and presentation under `extensions.com.openai`; other harnesses may use the portable components without a separate compatibility promise. Launchers resolve bundled commands from the plugin root and repair versioned binaries in `PLUGIN_DATA`, with XDG fallback for direct CLI use. Linux x86_64 uses the exact-version, checksum-verified static GitHub Release bundle; other hosts may build the pinned Rust source. Existing managed project MCP blocks are removed during confirmed setup; no new project or global server registrations are written.

Codex filters local MCP process environments, so a plugin-owned file in `PLUGIN_DATA` records the user's SSH agent socket path for signed Tiber operations. No secret is embedded in `mcp.json`. The installer follows marketplace `main`, offers GitHub and CodeRabbit as optional companions, and excludes Superpowers because its workflow overlaps. The installed GitHub plugin declares a Codex app connector, CodeRabbit declares skills and uses its CLI, and Superpowers declares skills and hooks. None of those installed packages has a root `mcp.json` or declares MCP servers in its manifest, so Development System does not inherit or copy an upstream server configuration. Agent Plugins 1.0.0 has no plugin dependency field, so these companions are installer choices, not manifest dependencies.

Root `plugin.json` is authoritative for SemVer. A bump command synchronizes the Codex marketplace and catalog; CI rejects plugin changes without a bump. Successful main CI publishes a new versioned binary release automatically, while unrelated pushes skip an already published version.

This verified-release-and-cache approach follows a pattern used by [mcp-bin](https://github.com/chriswessells/mcp-bin), while the synchronized metadata bump follows [Superpowers' version-bump tooling](https://github.com/obra/superpowers/blob/main/.version-bump.json). Neither becomes a runtime dependency.

## Consequences

Consuming repositories can use both MCPs after plugin installation without server setup. New versions require the matching release asset before Linux x86_64 startup, so release publication remains part of delivery. An available SSH agent is required for signed operations; the installer records its socket path when present. Portable manifests reduce future harness adaptation work without asserting untested support.

## Alternatives considered

Bundling compiled binaries in Git repeats the clone and history cost rejected by ADR-0015. A mandatory local Cargo build adds unnecessary setup cost for Linux x86_64. Depending on the consuming harness to register these servers retains the upgrade and ownership problem. Depending on optional upstream plugins as required dependencies would couple unrelated release cycles and has no portable manifest representation.

## Revisit when

Add another tested binary target, Agent Plugins gains dependency semantics, or a required harness cannot provide plugin-root commands and writable `PLUGIN_DATA`.
