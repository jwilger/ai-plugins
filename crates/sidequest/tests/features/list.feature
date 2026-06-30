Feature: Listing side-quests

  The user must be able to see which side-quests exist for a project and on which
  branches, so they can monitor what is running.

  Scenario: launched side-quests appear in the list
    Given a git repository
    When a harness launches a side-quest with the goal "fix the action buttons"
    And the harness lists the side-quests
    Then the list includes a side-quest on branch "side-quest/fix-the-action-buttons"
