Feature: Connecting to the control plane

  A harness (Claude Code or Codex) reaches sidequest by connecting to its MCP
  server. If that connection or handshake fails, nothing else works — so the
  control plane must be reachable as an MCP server and must identify itself.

  Scenario: a harness connects and reads the control plane's identity
    When a harness connects to the sidequest control plane over MCP
    Then the control plane identifies itself as "sidequest"
