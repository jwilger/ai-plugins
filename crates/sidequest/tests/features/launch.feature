Feature: Launching a side-quest in an isolated worktree

  When the user launches a side-quest, the work must happen in its own git
  worktree on a fresh branch, so it never disturbs the main checkout.

  Scenario: launching a side-quest creates an isolated worktree
    Given a git repository
    When a harness launches a side-quest with the goal "fix the action buttons"
    Then an isolated worktree exists on branch "side-quest/fix-the-action-buttons"

  Scenario: a very long goal still yields a branch git can create
    Given a git repository
    When a harness launches a side-quest with the goal "This is a really unsatisfactory change that does not fit the specification we talked about. After an incomplete answer, ask specific leading Socratic questions targeting exactly the missing piece, allow the learner to answer incrementally across multiple turns instead of demanding a full restatement, and make the tone read like a natural human tutor conversation"
    Then an isolated worktree exists with a branch name short enough for git

  Scenario: a side-quest runs its goal session inside the worktree
    Given a git repository
    And a session runner that records the goal to "goal.txt"
    When a harness launches a side-quest with the goal "fix the action buttons"
    Then the worktree contains "goal.txt" with "fix the action buttons"

  Scenario: a side-quest delivers its work to the local main branch
    Given a git repository
    And a project configured for local-merge delivery
    And a session runner that commits "feature.txt" with "done"
    When a harness launches a side-quest with the goal "add the feature"
    Then the main checkout contains "feature.txt" with "done"
    And the side-quest "side-quest/add-the-feature" is delivered

  Scenario: a side-quest runs in the background and is observable while running
    Given a git repository
    And a project configured for local-merge delivery
    And a session runner that waits for a signal then commits "feature.txt" with "done"
    When a harness launches a side-quest with the goal "add the feature"
    And the harness lists the side-quests
    Then the side-quest "side-quest/add-the-feature" is running
    When the side-quest is signaled to finish
    Then the main checkout contains "feature.txt" with "done"
