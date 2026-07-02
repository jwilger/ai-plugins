Feature: Resolving a side-quest's harness command

  A side-quest's goal session is driven by a real harness invocation. That
  command is resolved with a clear precedence: an explicit override, then the
  project's `sidequest.toml`, then the harness's built-in default -- and if
  none of those resolve, the side-quest must fail honestly rather than
  silently reporting success having done nothing.

  Scenario: the project's configured harness command overrides the built-in default
    Given a git repository
    And no session command is configured anywhere
    And a project configured with harness command "printf '%s' 'config-override' > config-ran.txt"
    When a harness launches a side-quest with the goal "do the thing" targeting "widget-bot"
    Then the worktree contains "config-ran.txt" with "config-override"

  Scenario: a side-quest fails with a clear reason when no session command can be resolved
    Given a git repository
    And no session command is configured anywhere
    And a project that allows cross-harness spawning
    When a harness launches a side-quest with the goal "do the thing" targeting "widget-bot"
    Then the side-quest "side-quest/do-the-thing" has state "failed"
    And the side-quest "side-quest/do-the-thing" has a detail containing "widget-bot"

  Scenario: a side-quest whose session fails is marked failed with its reason
    Given a git repository
    And a session runner that fails with "boom"
    When a harness launches a side-quest with the goal "do the thing"
    Then the side-quest "side-quest/do-the-thing" has state "failed"

  Scenario: a side-quest whose session makes no commits has nothing to deliver
    Given a git repository
    And a project configured for local-merge delivery
    When a harness launches a side-quest with the goal "do the thing"
    Then the side-quest "side-quest/do-the-thing" has state "done-no-changes"
