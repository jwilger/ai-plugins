#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  TMPROOT="$(mktemp -d)"
  mkdir -p "$TMPROOT/bin"
}

teardown() {
  rm -rf "$TMPROOT"
}

@test "share wrapper invokes promptfoo share and prints the final url" {
  cat >"$TMPROOT/bin/promptfoo" <<'SH'
#!/usr/bin/env bash
printf '%s\n' "$*" > "$PROMPTFOO_ARGS_FILE"
echo "View your eval at https://promptfoo.example/eval/abc123"
SH
  chmod +x "$TMPROOT/bin/promptfoo"

  run env PROMPTFOO_BIN="$TMPROOT/bin/promptfoo" PROMPTFOO_ARGS_FILE="$TMPROOT/promptfoo-args" \
    "$ROOT/scripts/evals/share.sh"

  [ "$status" -eq 0 ]
  [ "$(cat "$TMPROOT/promptfoo-args")" = "share" ]
  [[ "$output" == *"View your eval at https://promptfoo.example/eval/abc123"* ]]
  [[ "$output" == *"Promptfoo share URL: https://promptfoo.example/eval/abc123"* ]]
}

@test "share wrapper fails when promptfoo share does not return a url" {
  cat >"$TMPROOT/bin/promptfoo" <<'SH'
#!/usr/bin/env bash
echo "shared, but no url"
SH
  chmod +x "$TMPROOT/bin/promptfoo"

  run env PROMPTFOO_BIN="$TMPROOT/bin/promptfoo" "$ROOT/scripts/evals/share.sh"

  [ "$status" -eq 1 ]
  [[ "$output" == *"promptfoo share did not print a URL"* ]]
}
