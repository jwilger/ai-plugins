# babysit-pr

Bundled implementation reference for [Development System](../../README.md).
Install `development-system` from the marketplace; this component is not a separate public plugin.

Babysit a pull/merge request to completion: watch CI, respond to review
feedback, and merge once green and approved — across **GitHub, Forgejo, and
GitLab**, with no forge preferred over the others.

Used through Development System delivery with the forge's tooling.

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

## Harness

Codex.
