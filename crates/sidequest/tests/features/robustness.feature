Feature: Surfacing corrupted control-plane state

  A side-quest's config and registry live on disk. If either cannot be read for
  any reason other than being absent, the control plane must surface an error
  rather than silently fall back to defaults — silently resetting the registry
  would orphan running side-quests.

  Scenario: an unreadable config is not silently treated as default
    Given a git repository
    And the config path is a directory
    When a harness tries to launch a side-quest with the goal "do the thing"
    Then the launch is rejected

  Scenario: an unreadable registry is not silently treated as empty
    Given a git repository
    And the registry path is a directory
    When the harness tries to list the side-quests
    Then listing the side-quests fails
