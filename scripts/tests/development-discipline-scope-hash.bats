#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  HASH_SCRIPT="$ROOT/plugins/development-discipline/scripts/final-review-scope-hash.sh"
  REPO="$BATS_TEST_TMPDIR/repo"

  git init -q "$REPO"
  git -C "$REPO" config user.email test@example.com
  git -C "$REPO" config user.name Test
  git -C "$REPO" config commit.gpgsign false
  printf '%s\n' base >"$REPO/tracked.txt"
  git -C "$REPO" add tracked.txt
  git -C "$REPO" commit -qm base
}

scope_hash() {
  local inventory="$BATS_TEST_TMPDIR/changed-files.inventory"

  printf '%s\0' "$@" >"$inventory"
  "$HASH_SCRIPT" \
    --project-root "$REPO" \
    --scope base \
    --base HEAD \
    --changed-files-from "$inventory"
}

@test "final-review scope hash covers index worktree and untracked content deterministically" {
  local baseline
  local staged
  local unstaged
  local untracked
  local reordered
  local changed_untracked

  baseline="$(scope_hash tracked.txt)"

  printf '%s\n' staged >"$REPO/tracked.txt"
  git -C "$REPO" add tracked.txt
  staged="$(scope_hash tracked.txt)"
  [ "$staged" != "$baseline" ]

  printf '%s\n' unstaged >"$REPO/tracked.txt"
  unstaged="$(scope_hash tracked.txt)"
  [ "$unstaged" != "$staged" ]

  printf '%s\n' first >"$REPO/untracked.txt"
  untracked="$(scope_hash tracked.txt untracked.txt)"
  [ "$untracked" != "$unstaged" ]

  reordered="$(scope_hash untracked.txt tracked.txt)"
  [ "$reordered" = "$untracked" ]

  printf '%s\n' second >"$REPO/untracked.txt"
  changed_untracked="$(scope_hash tracked.txt untracked.txt)"
  [ "$changed_untracked" != "$untracked" ]
}

@test "final-review scope hash changes when a tracked path is deleted" {
  local baseline
  local deleted

  baseline="$(scope_hash tracked.txt)"
  rm "$REPO/tracked.txt"
  deleted="$(scope_hash tracked.txt)"

  [ "$deleted" != "$baseline" ]
}

@test "final-review scope hash includes the exact symlink target" {
  local first
  local second

  ln -s tracked.txt "$REPO/link.txt"
  first="$(scope_hash link.txt)"
  rm "$REPO/link.txt"
  ln -s missing.txt "$REPO/link.txt"
  second="$(scope_hash link.txt)"

  [ "$second" != "$first" ]
}

@test "final-review scope hash covers staged and unstaged gitlink pointer changes" {
  local submodule="$BATS_TEST_TMPDIR/submodule"
  local first_commit
  local second_commit
  local baseline
  local unstaged
  local staged

  git init -q "$submodule"
  git -C "$submodule" config user.email test@example.com
  git -C "$submodule" config user.name Test
  git -C "$submodule" config commit.gpgsign false
  printf '%s\n' first >"$submodule/content.txt"
  git -C "$submodule" add content.txt
  git -C "$submodule" commit -qm first
  first_commit="$(git -C "$submodule" rev-parse HEAD)"
  printf '%s\n' second >"$submodule/content.txt"
  git -C "$submodule" commit -qam second
  second_commit="$(git -C "$submodule" rev-parse HEAD)"

  git -c protocol.file.allow=always -C "$REPO" submodule add -q "$submodule" vendor/component
  git -C "$REPO/vendor/component" checkout -q "$first_commit"
  git -C "$REPO" add .gitmodules vendor/component
  git -C "$REPO" commit -qm 'add submodule'
  baseline="$(scope_hash vendor/component)"

  git -C "$REPO/vendor/component" checkout -q "$second_commit"
  unstaged="$(scope_hash vendor/component)"
  git -C "$REPO" add vendor/component
  staged="$(scope_hash vendor/component)"

  [ "$unstaged" != "$baseline" ]
  [ "$staged" != "$baseline" ]
}

@test "final-review scope hash preserves an untracked path containing a newline" {
  local path=$'untracked\nname.txt'
  local first
  local second

  printf '%s\n' first >"$REPO/$path"
  first="$(scope_hash "$path")"
  printf '%s\n' second >"$REPO/$path"
  second="$(scope_hash "$path")"

  [ "$second" != "$first" ]
}

@test "final-review scope hash disables repository-configured fsmonitor commands" {
  local hook="$BATS_TEST_TMPDIR/fsmonitor-hook"
  local marker="$BATS_TEST_TMPDIR/fsmonitor-ran"

  cat >"$hook" <<EOF
#!/usr/bin/env bash
: >"$marker"
printf '\0'
EOF
  chmod +x "$hook"
  git -C "$REPO" config core.fsmonitor "$hook"

  run scope_hash tracked.txt

  [ "$status" -eq 0 ]
  [ ! -e "$marker" ]
}

@test "final-review scope hash rejects an unterminated inventory" {
  local inventory="$BATS_TEST_TMPDIR/unterminated.inventory"

  printf '%s' tracked.txt >"$inventory"

  run "$HASH_SCRIPT" \
    --project-root "$REPO" \
    --scope base \
    --base HEAD \
    --changed-files-from "$inventory"

  [ "$status" -eq 2 ]
  [[ "$output" == *"inventory must be NUL-delimited"* ]]
}

@test "final-review scope hash handles the full inventory bound without argv expansion" {
  local inventory="$BATS_TEST_TMPDIR/large.inventory"
  local path

  {
    for ((i = 19999; i >= 0; i--)); do
      printf -v path 'generated/component-%05d/file-with-realistic-name.txt' "$i"
      printf '%s\0' "$path"
    done
  } >"$inventory"

  run timeout 60s "$HASH_SCRIPT" \
    --project-root "$REPO" \
    --scope base \
    --base HEAD \
    --changed-files-from "$inventory"

  [ "$status" -eq 0 ]
  [[ "$output" =~ ^[0-9a-f]+$ ]]
}
