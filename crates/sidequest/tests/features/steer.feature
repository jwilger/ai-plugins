Feature: Steering a running side-quest

  A side-quest must be able to ask the operator a question and continue once
  answered, so the operator can steer work in flight.

  Scenario: a side-quest asks the operator for input and continues once answered
    Given a git repository
    And a session runner that asks "ship it?" and records the answer to "answer.txt"
    When a harness launches a side-quest with the goal "risky change"
    Then the side-quest "side-quest/risky-change" is awaiting input with question "ship it?"
    When the operator answers "yes" to "side-quest/risky-change"
    Then the worktree contains "answer.txt" with "yes"
