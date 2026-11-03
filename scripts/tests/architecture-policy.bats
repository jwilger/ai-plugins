#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  checker="$ROOT/plugins/development-system/bin/development-system"
  repo="$BATS_TEST_TMPDIR/project"
  export GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
  git init -q --initial-branch=main "$repo"
  git -C "$repo" config user.name Test
  git -C "$repo" config user.email test@example.invalid
  git -C "$repo" config commit.gpgsign false
  git -C "$repo" config core.hooksPath "$repo/.git/hooks"
  printf '# Architecture\n\nCurrent boundary: one application.\n' >"$repo/ARCHITECTURE.md"
  git -C "$repo" add .
  git -C "$repo" commit -qm baseline
  baseline=$(git -C "$repo" rev-parse HEAD)
}

@test "architecture checker reports a valid paired addition without mutating the project" {
  mkdir -p "$repo/docs/adr"
  printf '# Durable boundaries\n\nStatus: Accepted\n\nDecision: isolate storage.\n' >"$repo/docs/adr/0001-storage.md"
  printf '\nStorage is owned by a separate persistence boundary.\n' >>"$repo/ARCHITECTURE.md"
  git -C "$repo" add .
  git -C "$repo" commit -qm 'feat: isolate storage'
  candidate=$(git -C "$repo" rev-parse HEAD)
  before=$(git -C "$repo" status --porcelain=v1)

  run "$checker" architecture-check --rules pairing,one-adr,immutable-history,supersession-only --project "$repo" \
    --base "$baseline" --head "$candidate" --trunk "$baseline" --delivery preserve

  [ "$status" -eq 0 ]
  jq -e --arg candidate "$candidate" --arg baseline "$baseline" '
    .schema_version == 1 and .head == $candidate and .trunk == $baseline
    and .checked_commits == [$candidate] and .violations == []
    and (.review_required | index("substantive-correspondence") != null)
    and (.review_required | index("additional-semantic-review-only-if-adopted") != null)
  ' <<<"$output"
  [ "$(git -C "$repo" rev-parse HEAD)" = "$candidate" ]
  [ "$(git -C "$repo" status --porcelain=v1)" = "$before" ]
}

@test "architecture checker rejects an ADR addition without its same-commit architecture update" {
  mkdir -p "$repo/docs/adr"
  printf '# Storage\n\nStatus: Accepted\n\nDecision: isolate storage.\n' >"$repo/docs/adr/0001-storage.md"
  git -C "$repo" add .
  git -C "$repo" commit -qm 'feat: isolate storage without updating current architecture'
  candidate=$(git -C "$repo" rev-parse HEAD)

  run "$checker" architecture-check --rules pairing,one-adr,immutable-history,supersession-only --project "$repo" \
    --base "$baseline" --head "$candidate" --trunk "$baseline" --delivery preserve

  [ "$status" -eq 1 ]
  jq -e --arg candidate "$candidate" '
    .violations | any(.code == "architecture-update-required" and .commit == $candidate)
  ' <<<"$output"
}

@test "architecture checker rejects multiple new ADRs in one otherwise paired commit" {
  mkdir -p "$repo/docs/adr"
  printf '# Storage\n\nStatus: Accepted\n' >"$repo/docs/adr/0001-storage.md"
  printf '# Transport\n\nStatus: Accepted\n' >"$repo/docs/adr/0002-transport.md"
  printf '\nStorage and transport have distinct ownership boundaries.\n' >>"$repo/ARCHITECTURE.md"
  git -C "$repo" add .
  git -C "$repo" commit -qm 'feat: establish two decisions at once'
  candidate=$(git -C "$repo" rev-parse HEAD)

  run "$checker" architecture-check --rules pairing,one-adr,immutable-history,supersession-only --project "$repo" \
    --base "$baseline" --head "$candidate" --trunk "$baseline" --delivery preserve

  [ "$status" -eq 1 ]
  jq -e --arg candidate "$candidate" '
    .violations | any(.code == "multiple-new-adrs" and .commit == $candidate
      and (.paths | length) == 2)
  ' <<<"$output"
}

@test "architecture checker rejects whitespace-only architecture pairing" {
  mkdir -p "$repo/docs/adr"
  printf '# Storage\n\nStatus: Accepted\n' >"$repo/docs/adr/0001-storage.md"
  printf '\n\n' >>"$repo/ARCHITECTURE.md"
  git -C "$repo" add .
  git -C "$repo" commit -qm 'feat: pair storage decision with cosmetic whitespace'
  candidate=$(git -C "$repo" rev-parse HEAD)

  run "$checker" architecture-check --rules pairing,one-adr,immutable-history,supersession-only --project "$repo" \
    --base "$baseline" --head "$candidate" --trunk "$baseline" --delivery preserve

  [ "$status" -eq 1 ]
  jq -e --arg candidate "$candidate" '
    .violations | any(.code == "architecture-update-required" and .commit == $candidate)
  ' <<<"$output"
}

@test "architecture checker rejects a paired edit to a merged ADR" {
  mkdir -p "$repo/docs/adr"
  printf '# Storage\n\n## Status\n\nAccepted\n\n## Context\n\nOriginal rationale.\n' >"$repo/docs/adr/0001-storage.md"
  printf '\nStorage has an explicit persistence boundary.\n' >>"$repo/ARCHITECTURE.md"
  git -C "$repo" add .
  git -C "$repo" commit -qm 'feat: establish merged storage decision'
  trunk=$(git -C "$repo" rev-parse HEAD)
  printf '\nRetrospective replacement of original rationale.\n' >>"$repo/docs/adr/0001-storage.md"
  printf '\nStorage rationale is now rewritten.\n' >>"$repo/ARCHITECTURE.md"
  git -C "$repo" add .
  git -C "$repo" commit -qm 'docs: rewrite merged rationale'
  candidate=$(git -C "$repo" rev-parse HEAD)

  run "$checker" architecture-check --rules pairing,one-adr,immutable-history,supersession-only --project "$repo" \
    --base "$trunk" --head "$candidate" --trunk "$trunk" --delivery preserve

  [ "$status" -eq 1 ]
  jq -e --arg candidate "$candidate" '
    .violations | any(.code == "merged-adr-immutable" and .commit == $candidate
      and .path == "docs/adr/0001-storage.md")
  ' <<<"$output"
}

@test "architecture checker protects an ADR first introduced in a historical merge result" {
  git -C "$repo" switch -qc feature
  printf 'feature\n' >"$repo/feature"
  git -C "$repo" add .
  git -C "$repo" commit -qm 'feat: feature work'
  git -C "$repo" switch -q main
  printf 'main\n' >"$repo/main"
  git -C "$repo" add .
  git -C "$repo" commit -qm 'feat: main work'
  git -C "$repo" merge --no-commit --no-ff feature
  mkdir -p "$repo/docs/adr"
  printf '# Storage\n\n## Status\n\nAccepted\n\n## Context\n\nOriginal rationale.\n' >"$repo/docs/adr/0001-storage.md"
  printf '\nStorage has an explicit persistence boundary.\n' >>"$repo/ARCHITECTURE.md"
  git -C "$repo" add .
  git -C "$repo" commit -qm 'merge: establish storage decision in merge result'
  trunk=$(git -C "$repo" rev-parse HEAD)
  printf '\nRewrite of merged historical rationale.\n' >>"$repo/docs/adr/0001-storage.md"
  printf '\nStorage rationale is rewritten.\n' >>"$repo/ARCHITECTURE.md"
  git -C "$repo" add .
  git -C "$repo" commit -qm 'docs: rewrite merge-introduced decision'
  candidate=$(git -C "$repo" rev-parse HEAD)

  run "$checker" architecture-check --rules pairing,one-adr,immutable-history,supersession-only --project "$repo" \
    --base "$trunk" --head "$candidate" --trunk "$trunk" --delivery preserve

  [ "$status" -eq 1 ]
  jq -e --arg candidate "$candidate" '
    .violations | any(.code == "merged-adr-immutable" and .commit == $candidate
      and .path == "docs/adr/0001-storage.md")
  ' <<<"$output"
}

@test "architecture checker accepts status-only supersession by one new paired ADR" {
  mkdir -p "$repo/docs/adr"
  printf '# Storage\n\n## Status\n\nAccepted\n\n## Context\n\nOriginal rationale.\n' >"$repo/docs/adr/0001-storage.md"
  printf '\nStorage is local.\n' >>"$repo/ARCHITECTURE.md"
  git -C "$repo" add .
  git -C "$repo" commit -qm 'feat: establish local storage decision'
  trunk=$(git -C "$repo" rev-parse HEAD)
  printf '# Storage\n\n## Status\n\nSuperseded by [ADR-0002](0002-remote-storage.md)\n\n## Context\n\nOriginal rationale.\n' >"$repo/docs/adr/0001-storage.md"
  printf '# Remote storage\n\n## Status\n\nAccepted\n\n## Context\n\nChanged durability requirements.\n' >"$repo/docs/adr/0002-remote-storage.md"
  printf '\nStorage now uses the remote durable persistence boundary.\n' >>"$repo/ARCHITECTURE.md"
  git -C "$repo" add .
  git -C "$repo" commit -qm 'feat: supersede local storage decision'
  candidate=$(git -C "$repo" rev-parse HEAD)

  run "$checker" architecture-check --rules pairing,one-adr,immutable-history,supersession-only --project "$repo" \
    --base "$trunk" --head "$candidate" --trunk "$trunk" --delivery preserve

  [ "$status" -eq 0 ]
  jq -e '.violations == []' <<<"$output"
}

@test "architecture checker rejects an absolute supersession link that aliases a new relative ADR" {
  mkdir -p "$repo/docs/adr"
  printf '# Storage\n\n## Status\n\nAccepted\n\n## Context\n\nOriginal rationale.\n' >"$repo/docs/adr/0001-storage.md"
  printf '\nStorage is local.\n' >>"$repo/ARCHITECTURE.md"
  git -C "$repo" add .
  git -C "$repo" commit -qm 'feat: establish local storage decision'
  trunk=$(git -C "$repo" rev-parse HEAD)
  printf '# Storage\n\n## Status\n\nSuperseded by [ADR-0002](/0002-remote-storage.md)\n\n## Context\n\nOriginal rationale.\n' >"$repo/docs/adr/0001-storage.md"
  printf '# Remote storage\n\n## Status\n\nAccepted\n' >"$repo/docs/adr/0002-remote-storage.md"
  printf '\nStorage now uses a remote persistence boundary.\n' >>"$repo/ARCHITECTURE.md"
  git -C "$repo" add .
  git -C "$repo" commit -qm 'feat: use misleading supersession reference'
  candidate=$(git -C "$repo" rev-parse HEAD)

  run "$checker" architecture-check --rules pairing,one-adr,immutable-history,supersession-only --project "$repo" \
    --base "$trunk" --head "$candidate" --trunk "$trunk" --delivery preserve

  [ "$status" -eq 1 ]
  jq -e '.violations | any(.code == "merged-adr-immutable"
    and .path == "docs/adr/0001-storage.md")' <<<"$output"
}

@test "architecture checker preserves history outside an unambiguous status field" {
  baseline=$(git -C "$repo" rev-parse HEAD)
  failures=0
  for structure in fenced historical; do
    git -C "$repo" checkout -q -B "$structure" "$baseline"
    mkdir -p "$repo/docs/adr"
    if [ "$structure" = fenced ]; then
      printf '# Storage\n\n```markdown\n## Status\n\nAccepted\n\n## Context\n\nExample.\n```\n\n## Status\n\nAccepted\n' >"$repo/docs/adr/0001-storage.md"
    else
      printf '# Storage\n\n## Status\n\nAccepted\n\n# Historical context\n\nOriginal rationale.\n' >"$repo/docs/adr/0001-storage.md"
    fi
    git -C "$repo" add .
    git -C "$repo" commit -qm 'feat: record original decision'
    trunk=$(git -C "$repo" rev-parse HEAD)
    if [ "$structure" = fenced ]; then
      printf '# Storage\n\n```markdown\n## Status\n\nSuperseded by [ADR-0002](0002-remote-storage.md)\n\n## Context\n\nExample.\n```\n\n## Status\n\nAccepted\n' >"$repo/docs/adr/0001-storage.md"
    else
      printf '# Storage\n\n## Status\n\nSuperseded by [ADR-0002](0002-remote-storage.md)\n' >"$repo/docs/adr/0001-storage.md"
    fi
    printf '# Remote storage\n' >"$repo/docs/adr/0002-remote-storage.md"
    printf '\nStorage now uses a remote persistence boundary.\n' >>"$repo/ARCHITECTURE.md"
    git -C "$repo" add .
    git -C "$repo" commit -qm 'feat: disguise historical edit as supersession'
    candidate=$(git -C "$repo" rev-parse HEAD)

    run "$checker" architecture-check --rules pairing,one-adr,immutable-history,supersession-only --project "$repo" \
      --base "$trunk" --head "$candidate" --trunk "$trunk" --delivery preserve

    if [ "$status" -ne 1 ] || ! jq -e '.violations | any(.code == "merged-adr-immutable")' <<<"$output"; then
      printf '%s history incorrectly accepted: %s\n' "$structure" "$output"
      failures=$((failures + 1))
    fi
  done
  [ "$failures" -eq 0 ]
}

@test "architecture checker rejects ambiguous status tokens and escaped supersession delimiters" {
  baseline=$(git -C "$repo" rev-parse HEAD)
  failures=0
  for structure in comment fence escaped; do
    git -C "$repo" checkout -q -B "$structure" "$baseline"
    mkdir -p "$repo/docs/adr"
    case "$structure" in
      comment) original='<!-- Accepted'; closing='-->' ;;
      fence) original='```markdown'; closing='```' ;;
      escaped) original='Accepted'; closing='' ;;
    esac
    printf '# Storage\n\n## Status\n\n%s\n\n## Context\n\nOriginal rationale.\n%s\n' "$original" "$closing" >"$repo/docs/adr/0001-storage.md"
    git -C "$repo" add .
    git -C "$repo" commit -qm 'feat: record original decision'
    trunk=$(git -C "$repo" rev-parse HEAD)
    reference='Superseded by [ADR-0002](0002-remote-storage.md)'
    if [ "$structure" = escaped ]; then
      reference='Superseded by [ADR\](0002-remote-storage.md)'
    fi
    printf '# Storage\n\n## Status\n\n%s\n\n## Context\n\nOriginal rationale.\n%s\n' "$reference" "$closing" >"$repo/docs/adr/0001-storage.md"
    printf '# Remote storage\n' >"$repo/docs/adr/0002-remote-storage.md"
    printf '\nStorage now uses a remote persistence boundary.\n' >>"$repo/ARCHITECTURE.md"
    git -C "$repo" add .
    git -C "$repo" commit -qm 'feat: use ambiguous supersession field'
    candidate=$(git -C "$repo" rev-parse HEAD)

    run "$checker" architecture-check --rules pairing,one-adr,immutable-history,supersession-only --project "$repo" \
      --base "$trunk" --head "$candidate" --trunk "$trunk" --delivery preserve

    if [ "$status" -ne 1 ] || ! jq -e '.violations | any(.code == "merged-adr-immutable")' <<<"$output"; then
      printf '%s status incorrectly accepted: %s\n' "$structure" "$output"
      failures=$((failures + 1))
    fi
  done
  [ "$failures" -eq 0 ]
}

@test "architecture checker forbids rewriting another unmerged ADR in a new ADR commit" {
  trunk=$(git -C "$repo" rev-parse HEAD)
  mkdir -p "$repo/docs/adr"
  printf '# Storage\n\n## Status\n\nProposed\n\n## Context\n\nOriginal rationale.\n' >"$repo/docs/adr/0001-storage.md"
  printf '\nStorage is local.\n' >>"$repo/ARCHITECTURE.md"
  git -C "$repo" add .
  git -C "$repo" commit -qm 'feat: propose local storage'
  printf '# Storage\n\n## Status\n\nProposed\n\n## Context\n\nRewritten rationale.\n' >"$repo/docs/adr/0001-storage.md"
  printf '# Remote storage\n' >"$repo/docs/adr/0002-remote-storage.md"
  printf '\nStorage now uses a remote persistence boundary.\n' >>"$repo/ARCHITECTURE.md"
  git -C "$repo" add .
  git -C "$repo" commit -qm 'feat: add another decision while rewriting history'
  candidate=$(git -C "$repo" rev-parse HEAD)

  run "$checker" architecture-check --rules pairing,one-adr,immutable-history,supersession-only --project "$repo" \
    --base "$trunk" --head "$candidate" --trunk "$trunk" --delivery preserve

  [ "$status" -eq 1 ]
  jq -e --arg commit "$candidate" '.violations | any(
    .code == "other-adr-supersession-only" and .commit == $commit
    and .path == "docs/adr/0001-storage.md")' <<<"$output"
}

@test "architecture checker rejects squashing two separately paired ADR additions" {
  trunk=$(git -C "$repo" rev-parse HEAD)
  mkdir -p "$repo/docs/adr"
  for decision in 0001-storage 0002-transport; do
    printf '# Decision %s\n' "$decision" >"$repo/docs/adr/$decision.md"
    printf '\nCurrent boundary: %s.\n' "$decision" >>"$repo/ARCHITECTURE.md"
    git -C "$repo" add .
    git -C "$repo" commit -qm "feat: establish $decision boundary"
  done
  candidate=$(git -C "$repo" rev-parse HEAD)

  run "$checker" architecture-check --rules pairing,one-adr,immutable-history,supersession-only --project "$repo" \
    --base "$trunk" --head "$candidate" --trunk "$trunk" --delivery preserve
  [ "$status" -eq 0 ]
  jq -e '.violations == [] and (.checked_commits | length == 2)' <<<"$output"

  run "$checker" architecture-check --rules pairing,one-adr,immutable-history,supersession-only --project "$repo" \
    --base "$trunk" --head "$candidate" --trunk "$trunk" --delivery squash

  [ "$status" -eq 1 ]
  jq -e '.violations | any(.code == "squash-multiple-new-adrs"
    and (.paths | length == 2))' <<<"$output"
}

@test "architecture checker rejects squash when later rollback cancels the ADR architecture pairing" {
  trunk=$(git -C "$repo" rev-parse HEAD)
  mkdir -p "$repo/docs/adr"
  printf '# Storage\n' >"$repo/docs/adr/0001-storage.md"
  printf '\nStorage uses a remote persistence boundary.\n' >>"$repo/ARCHITECTURE.md"
  git -C "$repo" add .
  git -C "$repo" commit -qm 'feat: establish storage boundary'
  git -C "$repo" show "$trunk:ARCHITECTURE.md" >"$repo/ARCHITECTURE.md"
  git -C "$repo" add .
  git -C "$repo" commit -qm 'docs: remove current architecture boundary'
  candidate=$(git -C "$repo" rev-parse HEAD)

  run "$checker" architecture-check --rules pairing,one-adr,immutable-history,supersession-only --project "$repo" \
    --base "$trunk" --head "$candidate" --trunk "$trunk" --delivery squash

  [ "$status" -eq 1 ]
  jq -e '.violations | any(.code == "squash-architecture-update-required")' <<<"$output"
}

@test "architecture checker rejects a base that skips pending trunk commits" {
  trunk=$(git -C "$repo" rev-parse HEAD)
  mkdir -p "$repo/docs/adr"
  printf '# Unpaired storage decision\n' >"$repo/docs/adr/0001-storage.md"
  git -C "$repo" add .
  git -C "$repo" commit -qm 'feat: add unpaired decision'
  candidate=$(git -C "$repo" rev-parse HEAD)

  run "$checker" architecture-check --rules pairing,one-adr,immutable-history,supersession-only --project "$repo" \
    --base "$candidate" --head "$candidate" --trunk "$trunk" --delivery preserve

  [ "$status" -eq 2 ]
  [[ "$output" == *"base must be the unique merge base of trunk and head"* ]]
}

@test "architecture checker requires architecture pairing for each premerge ADR edit" {
  trunk=$(git -C "$repo" rev-parse HEAD)
  mkdir -p "$repo/docs/adr"
  printf '# Storage\n\nOriginal rationale.\n' >"$repo/docs/adr/0001-storage.md"
  printf '\nStorage has a local persistence boundary.\n' >>"$repo/ARCHITECTURE.md"
  git -C "$repo" add .
  git -C "$repo" commit -qm 'feat: propose storage boundary'
  proposed=$(git -C "$repo" rev-parse HEAD)
  for pairing in paired unpaired; do
    git -C "$repo" checkout -q -B "$pairing" "$proposed"
    printf '# Storage\n\nRevised rationale for remote persistence.\n' >"$repo/docs/adr/0001-storage.md"
    if [ "$pairing" = paired ]; then
      printf '\nThe revised boundary uses remote persistence.\n' >>"$repo/ARCHITECTURE.md"
    fi
    git -C "$repo" add .
    git -C "$repo" commit -qm 'docs: refine unmerged storage decision'
    candidate=$(git -C "$repo" rev-parse HEAD)

    run "$checker" architecture-check --rules pairing,one-adr,immutable-history,supersession-only --project "$repo" \
      --base "$trunk" --head "$candidate" --trunk "$trunk" --delivery preserve

    if [ "$pairing" = paired ]; then
      [ "$status" -eq 0 ]
      jq -e '.violations == [] and (.checked_commits | length == 2)' <<<"$output"
    else
      [ "$status" -eq 1 ]
      jq -e --arg commit "$candidate" '.violations | any(
        .code == "architecture-update-required" and .commit == $commit)' <<<"$output"
    fi
  done
}

@test "architecture checker rejects squash supersession whose replacement disappears" {
  mkdir -p "$repo/docs/adr"
  printf '# Storage\n\n## Status\n\nAccepted\n\n## Context\n\nOriginal rationale.\n' >"$repo/docs/adr/0001-storage.md"
  printf '\nStorage is local.\n' >>"$repo/ARCHITECTURE.md"
  git -C "$repo" add .
  git -C "$repo" commit -qm 'feat: establish local storage decision'
  trunk=$(git -C "$repo" rev-parse HEAD)
  printf '# Storage\n\n## Status\n\nSuperseded by [ADR-0002](0002-remote-storage.md)\n\n## Context\n\nOriginal rationale.\n' >"$repo/docs/adr/0001-storage.md"
  printf '# Remote storage\n\n## Status\n\nAccepted\n\n## Context\n\nChanged durability requirements.\n' >"$repo/docs/adr/0002-remote-storage.md"
  printf '\nStorage now uses the remote durable persistence boundary.\n' >>"$repo/ARCHITECTURE.md"
  git -C "$repo" add .
  git -C "$repo" commit -qm 'feat: supersede local storage decision'
  git -C "$repo" rm -q docs/adr/0002-remote-storage.md
  printf '\nRemote replacement was withdrawn.\n' >>"$repo/ARCHITECTURE.md"
  git -C "$repo" add .
  git -C "$repo" commit -qm 'docs: withdraw unmerged replacement'
  candidate=$(git -C "$repo" rev-parse HEAD)

  run "$checker" architecture-check --rules pairing,one-adr,immutable-history,supersession-only --project "$repo" \
    --base "$trunk" --head "$candidate" --trunk "$trunk" --delivery squash

  [ "$status" -eq 1 ]
  jq -e '.violations | any(.code == "squash-merged-adr-immutable"
    and .path == "docs/adr/0001-storage.md")' <<<"$output"
}

@test "architecture checker rejects shallow history instead of claiming merged ADR protection" {
  shallow="$BATS_TEST_TMPDIR/shallow"
  git clone -q --depth=1 "file://$repo" "$shallow"
  printf '\nApplication now isolates durable storage.\n' >>"$shallow/ARCHITECTURE.md"
  mkdir -p "$shallow/docs/adr"
  printf '# Storage\n\nDecision: isolate persistence.\n' >"$shallow/docs/adr/0001-storage.md"
  git -C "$shallow" -c user.name=Test -c user.email=test@example.invalid add .
  git -C "$shallow" -c user.name=Test -c user.email=test@example.invalid commit -qm 'feat: isolate persistence'
  candidate=$(git -C "$shallow" rev-parse HEAD)

  run "$checker" architecture-check --rules pairing,one-adr,immutable-history,supersession-only --project "$shallow" \
    --base "$baseline" --head "$candidate" --trunk "$baseline" --delivery preserve

  [ "$status" -eq 2 ]
  [[ "$output" == *"complete history is required; shallow repositories are unsupported"* ]]
}

@test "architecture checker skips unadopted recommendations without inspecting Git history" {
  run "$checker" architecture-check --project "$BATS_TEST_TMPDIR/no-repository"
  [ "$status" -eq 0 ]
  jq -e '.status == "skipped" and .reason == "no-adopted-rules" and .checks == [] and .violations == []' <<<"$output"
}

@test "architecture checker enforces only explicitly selected project rules" {
  mkdir -p "$repo/docs/adr"
  printf '# First\n' >"$repo/docs/adr/0001-first.md"
  printf '# Second\n' >"$repo/docs/adr/0002-second.md"
  git -C "$repo" add .
  git -C "$repo" commit -qm 'docs: project permits batched unpaired decisions'
  candidate=$(git -C "$repo" rev-parse HEAD)
  run "$checker" architecture-check --rules immutable-history --project "$repo" \
    --base "$baseline" --head "$candidate" --trunk "$baseline" --delivery squash
  [ "$status" -eq 0 ]
  jq -e '.rules == ["immutable-history"] and .violations == [] and (.checks | index("one-new-adr-per-commit") == null) and (.review_required | index("sequential-authoring") == null)' <<<"$output"
  run "$checker" architecture-check --rules pairing --project "$repo" \
    --base "$baseline" --head "$candidate" --trunk "$baseline" --delivery preserve
  [ "$status" -eq 1 ]
  jq -e '.violations | all(.code == "architecture-update-required")' <<<"$output"
}

@test "architecture checker rejects unknown selected rules instead of silently skipping" {
  run "$checker" architecture-check --rules typo --project "$repo"
  [ "$status" -eq 2 ]
  [[ "$output" == *"unknown or duplicate rule"* ]]
}

@test "supersession-only selection reports its own rule for a merged edit" {
  mkdir -p "$repo/docs/adr"
  printf '# First decision\n\n## Status\n\nAccepted\n' >"$repo/docs/adr/0001-first.md"
  git -C "$repo" add .
  git -C "$repo" commit -qm 'Establish history'
  trunk=$(git -C "$repo" rev-parse HEAD)
  printf '\nChanged historical rationale.\n' >>"$repo/docs/adr/0001-first.md"
  printf '# Second decision\n' >"$repo/docs/adr/0002-second.md"
  git -C "$repo" add .
  git -C "$repo" commit -qm 'Add followup with unrelated history edit'
  for delivery in preserve squash; do
    run "$checker" architecture-check --project "$repo" --base "$trunk" --head HEAD --trunk "$trunk" --delivery "$delivery" --rules supersession-only
    [ "$status" -eq 1 ]
    [[ "$output" == *other-adr-supersession-only* ]]
    [[ "$output" != *merged-adr-immutable* ]]
  done
}
