# Scrubbing And Anonymization

Before submitting an eval-case issue, convert the observed behavior into a
portable scenario.

## Remove

- API keys, tokens, cookies, session ids, passwords, private keys, and auth
  headers.
- Personal data that is not necessary to reproduce the behavior.
- Client names, account ids, internal hostnames, private repository names, and
  contract-specific details.
- Raw proprietary source excerpts or private knowledge-base passages.
- Full logs when a short sanitized excerpt is enough.

## Replace

- Real names with roles such as `domain expert`, `reviewer`, or `customer`.
- Real organizations with `ExampleCo`.
- Real endpoints with `https://example.invalid/...`.
- Real identifiers with stable placeholders such as `CASE-123`,
  `ACCOUNT-456`, or `USER-789`.
- Private data values with realistic but synthetic equivalents.

## Preserve

- The user intent.
- The minimum input needed to trigger the behavior.
- The actual behavior.
- The expected behavior.
- The assertion or rubric that would distinguish the two.
- The expected outcome category: `pass`, `fail`, `partial`, `adversarial`, or
  `unsure`.

## Preview Format

Use this body shape:

````markdown
## Issue title

[eval-case]: <sanitized short title>

## Plugin or repo area

<plugin, skill, command, workflow, or repo area>

## Scenario

<sanitized scenario>

## Sanitized input

```text
<sanitized input>
```

## Actual behavior

<what happened>

## Expected behavior

<what should happen>

## Expected eval outcome

<pass | fail | partial | adversarial | unsure>

## Suggested assertion or rubric

<deterministic assertion or judging rubric>

## Supporting artifacts

<safe links or notes>

## Safety check

- [x] Secrets, credentials, auth headers, cookies, session ids, private keys,
      private client data, private repository names, internal hostnames, and raw
      proprietary excerpts were removed or replaced.
````
