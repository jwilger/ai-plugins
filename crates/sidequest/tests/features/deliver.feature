Feature: Delivering to an origin integration branch

  A side-quest can deliver its work by pushing to the project's origin
  integration branch, not just merging locally.

  Scenario: a side-quest pushes its work to the origin integration branch
    Given a git repository
    And a bare origin remote
    And a project configured for push-origin delivery
    And a session runner that commits "feature.txt" with "done"
    When a harness launches a side-quest with the goal "add the feature"
    Then the origin integration branch contains "feature.txt" with "done"
    And the side-quest "side-quest/add-the-feature" is delivered

  Scenario: PR-mode delivery pushes the side-quest's feature branch to origin
    Given a git repository
    And a bare origin remote
    And a project configured for PR delivery
    And a session runner that commits "feature.txt" with "done"
    When a harness launches a side-quest with the goal "add the feature"
    Then the origin branch "side-quest/add-the-feature" contains "feature.txt" with "done"
    And the side-quest "side-quest/add-the-feature" is delivered
