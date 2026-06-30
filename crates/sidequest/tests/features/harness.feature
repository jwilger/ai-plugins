Feature: Cross-harness spawning

  A side-quest may run in a harness other than the project default, but only when
  the project opts in.

  Scenario: cross-harness spawn is allowed when the project opts in
    Given a git repository
    And a project that allows cross-harness spawning
    When a harness launches a side-quest with the goal "port the ui" targeting "codex"
    And the harness lists the side-quests
    Then the side-quest "side-quest/port-the-ui" targets harness "codex"

  Scenario: cross-harness spawn is rejected by default
    Given a git repository
    When a harness tries to launch a side-quest with the goal "port the ui" targeting "codex"
    Then the launch is rejected
