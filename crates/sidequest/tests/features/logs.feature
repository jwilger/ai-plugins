Feature: Reading a side-quest's session log

  A side-quest's goal session output is captured live as it runs, so its
  progress is inspectable while it runs or after it finishes -- there must be
  a way to watch what a backgrounded side-quest is actually doing.

  Scenario: a side-quest's session output can be read back via the logs tool
    Given a git repository
    And a session runner that prints "hello from the session"
    When a harness launches a side-quest with the goal "do the thing"
    Then the side-quest "side-quest/do-the-thing"'s log contains "hello from the session"

  Scenario: the logs tool returns nothing for a side-quest that was never launched
    Given a git repository
    Then the side-quest "side-quest/never-launched"'s log is empty

  Scenario: an unreadable log surfaces an error rather than pretending it is empty
    Given a git repository
    And the log path for "side-quest/broken-log" is a directory
    Then reading the logs for "side-quest/broken-log" fails

  Scenario: the logs tool never returns more than a bounded tail of a very large log
    Given a git repository
    And a session runner that writes a very large log
    When a harness launches a side-quest with the goal "do the thing"
    Then the side-quest "side-quest/do-the-thing"'s log is at most 300000 characters
