Feature: Launching a side-quest in an isolated worktree

  When the user launches a side-quest, the work must happen in its own git
  worktree on a fresh branch, so it never disturbs the main checkout.

  Scenario: launching a side-quest creates an isolated worktree
    Given a git repository
    When a harness launches a side-quest with the goal "fix the action buttons"
    Then an isolated worktree exists on branch "side-quest/fix-the-action-buttons"
