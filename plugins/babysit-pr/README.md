# babysit-pr

Babysit a pull/merge request to completion: watch CI, respond to review
feedback, and merge once green and approved — across **GitHub, Forgejo, and
GitLab**, with no forge preferred over the others.

Usable standalone (it only needs the forge's tooling), or as the PR phase of a
side-quest's delivery.

## What it provides

- The `babysit-pr` skill (`skills/babysit-pr/`) — the forge-agnostic babysitting
  loop.
- `scripts/detect-forge.sh` — classify a repository's forge from its `origin`
  remote (github / gitlab / forgejo), used to pick the right tooling.

## Tooling per forge

| Forge   | Tooling                                            |
| ------- | -------------------------------------------------- |
| GitHub  | `gh`                                               |
| Forgejo | Forgejo MCP tools (`forgejo_*`); `tea` as fallback |
| GitLab  | `glab`                                             |

## Harnesses

Harness-agnostic — the skill and script are consumed identically by Claude Code
and Codex.
