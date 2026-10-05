#!/usr/bin/env bats

setup() {
  repo="$BATS_TEST_TMPDIR/project"
  tools="$BATS_TEST_TMPDIR/tools"
  mkdir -p "$repo/scripts" "$tools" "$BATS_TEST_TMPDIR/shadow"
  cp "$BATS_TEST_DIRNAME/../serena-rust-analyzer.sh" "$repo/scripts/"
  export PINNED_ANALYZER="$BATS_TEST_TMPDIR/pinned-analyzer"
  export SHADOW_DIRECTORY="$BATS_TEST_TMPDIR/shadow"
  cat > "$PINNED_ANALYZER" <<'SH'
#!/usr/bin/env bash
printf 'PINNED %s\n' "$*"
SH
  printf '#!/usr/bin/env bash\nprintf "SHADOW\\n"\n' > "$SHADOW_DIRECTORY/rust-analyzer"
  cat > "$tools/nix" <<'SH'
#!/usr/bin/env bash
[[ "$*" == print-dev-env ]] || exit 43
[[ "${NIX_FAIL_STATUS:-0}" == 0 ]] || exit "$NIX_FAIL_STATUS"
printf 'export AI_PLUGINS_RUST_ANALYZER_BIN=%q\n' "$PINNED_ANALYZER"
printf 'export PATH=%q:$PATH\n' "$SHADOW_DIRECTORY"
printf 'printf "shell-hook diagnostics\\n"\n'
SH
  chmod +x "$tools/nix" "$PINNED_ANALYZER" "$SHADOW_DIRECTORY/rust-analyzer"
}

@test "analyzer uses the selected absolute executable and keeps hook output off protocol stdout" {
  run env PATH="$tools:$PATH" AI_PLUGINS_RUST_ANALYZER_BIN="$SHADOW_DIRECTORY/rust-analyzer" \
    bash -c 'bash "$1" --version > "$2" 2> "$3"' _ "$repo/scripts/serena-rust-analyzer.sh" "$BATS_TEST_TMPDIR/stdout" "$BATS_TEST_TMPDIR/stderr"
  [ "$status" -eq 0 ]
  [ "$(cat "$BATS_TEST_TMPDIR/stdout")" = 'PINNED --version' ]
  [ "$(cat "$BATS_TEST_TMPDIR/stderr")" = 'shell-hook diagnostics' ]
}

@test "failed environment loading does not fall back to a shadow analyzer" {
  run env PATH="$tools:$SHADOW_DIRECTORY:$PATH" NIX_FAIL_STATUS=47 bash "$repo/scripts/serena-rust-analyzer.sh" --version
  [ "$status" -eq 47 ]
  [[ "$output" != *SHADOW* ]]
}
