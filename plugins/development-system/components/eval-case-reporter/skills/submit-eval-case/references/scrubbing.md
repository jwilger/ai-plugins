# Sanitization And De-identification

Before submitting an eval-case issue, convert the observed behavior into a
portable scenario.

## Remove

- API keys, tokens, cookies, session ids, passwords, private keys, and auth
  headers.
- Direct identifiers and personal data that are not necessary to reproduce the
  behavior.
- Quasi-identifiers whose combination could identify a person, customer, or
  private project, including precise dates, locations, rare roles, commit hashes,
  internal paths, and artifact metadata.
- Client names, account ids, internal hostnames, private repository names, and
  contract-specific details.
- Raw proprietary source excerpts or private knowledge-base passages.
- Full logs when a short sanitized excerpt is enough.

## Replace

- Real names with roles such as `domain expert`, `reviewer`, or `customer`.
- Real organizations with `ExampleCo`.
- Real endpoints with `https://example.invalid/...`.
- Real identifiers with stable placeholders such as `CASE-123`, `ACCOUNT-456`,
  or `USER-789`. This is pseudonymization for scenario consistency, not proof of
  anonymity; remove or generalize linkable context as well.
- Private data values with realistic but synthetic equivalents.

## Preserve

- The user intent.
- The minimum input needed to trigger the behavior.
- The actual behavior.
- The expected behavior.
- Safe reproduction provenance: harness and model family, plugin or skill
  version, relevant tool availability, and the minimum triggering context.
- The deterministic assertion or judging rubric that distinguishes actual from
  expected behavior.
- The observed result: `pass`, `fail`, `partial`, or `uncertain`.
- The independent coverage kind: `regression`, `boundary`, `adversarial`, or
  `other`.

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

## Observed result

<pass | fail | partial | uncertain>

## Coverage kind

<regression | boundary | adversarial | other>

## Suggested assertion or rubric

<deterministic assertion or judging rubric>

## Supporting artifacts

<safe links or notes>

## Safety check

- [x] Secrets, authentication material, direct identifiers, linkable
      quasi-identifiers, private client data, private repository details,
      internal hosts or paths, raw proprietary excerpts, and unsafe artifact
      metadata were removed, generalized, or replaced with synthetic values.
````
