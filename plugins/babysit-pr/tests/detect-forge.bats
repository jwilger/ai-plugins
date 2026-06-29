#!/usr/bin/env bats
# Tests for forge detection.

setup() {
  SCRIPT="$BATS_TEST_DIRNAME/../scripts/detect-forge.sh"
  REPO="$(mktemp -d)"
  git -C "$REPO" init -q
}

teardown() {
  rm -rf "$REPO"
}

detect() {
  git -C "$REPO" remote remove origin 2>/dev/null || true
  git -C "$REPO" remote add origin "$1"
  bash "$SCRIPT" "$REPO"
}

@test "detects github" {
  [ "$(detect 'git@github.com:owner/repo.git')" = "github" ]
  [ "$(detect 'https://github.com/owner/repo.git')" = "github" ]
}

@test "detects gitlab" {
  [ "$(detect 'git@gitlab.com:owner/repo.git')" = "gitlab" ]
  [ "$(detect 'https://gitlab.example.org/owner/repo.git')" = "gitlab" ]
}

@test "defaults self-hosted to forgejo" {
  [ "$(detect 'ssh://forgejo@git.johnwilger.com:2222/owner/repo.git')" = "forgejo" ]
}
