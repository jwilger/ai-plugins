---
title: Publish a GitHub release for each marketplace version
blocked_by: []
blocks: []
tags: [release, github-actions, marketplace, semantic-versioning]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Create an automatic GitHub release for every published marketplace version so users can install a stable, named version instead of depending on the latest main-branch state. Matching releases make versions reproducible, easier to understand, and safer to roll back.

## Context / Why

Marketplace metadata can advance without a matching GitHub release and Git tag. A published marketplace version should identify one exact, tested state of the repository so users can pin installations and maintainers can trace or roll back a release. Implementation notes: Add a GitHub Actions workflow that runs only for commits on main. Read the marketplace version from the canonical metadata files and require all applicable Claude Code and Codex marketplace version fields to agree. Compare it with existing releases and tags. When the version is newer and no matching release exists, create a semantic-version tag and GitHub release for that exact commit. Make reruns succeed without duplicates. Release only after required CI succeeds for the exact commit, not merely because metadata changed. Fail clearly on version regression, conflicting metadata, a tag pointing to another commit, or a mismatched existing release. Document a vMAJOR.MINOR.PATCH tag convention and generate useful notes from commits since the prior release.

## Acceptance criteria

## Subtasks

## Notes / Log
