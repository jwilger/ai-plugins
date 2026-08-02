#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  TMPROOT="$(mktemp -d)"
  mkdir -p "$TMPROOT/bin" "$TMPROOT/out"
  chmod 700 "$TMPROOT/out"
  printf '{"results":{"results":[]}}\n' >"$TMPROOT/out/results.json"
  chmod 600 "$TMPROOT/out/results.json"
  node "$ROOT/scripts/evals/artifact-scan-receipt.mjs" write \
    "$TMPROOT/out/artifact-scan-receipt.json" \
    "$TMPROOT/out/results.json"
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

  run env EVAL_OUT_DIR="$TMPROOT/out" PROMPTFOO_BIN="$TMPROOT/bin/promptfoo" PROMPTFOO_ARGS_FILE="$TMPROOT/promptfoo-args" \
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

  run env EVAL_OUT_DIR="$TMPROOT/out" PROMPTFOO_BIN="$TMPROOT/bin/promptfoo" "$ROOT/scripts/evals/share.sh"

  [ "$status" -eq 1 ]
  [[ "$output" == *"promptfoo share did not print a URL"* ]]
}

@test "share wrapper refuses a secret-bearing artifact before invoking promptfoo" {
  secret="codex-fixture-secret-that-must-not-escape"
  printf '{"response":"%s"}\n' "$secret" >"$TMPROOT/out/results.json"
  node "$ROOT/scripts/evals/artifact-scan-receipt.mjs" write \
    "$TMPROOT/out/artifact-scan-receipt.json" \
    "$TMPROOT/out/results.json"
  cat >"$TMPROOT/bin/promptfoo" <<'SH'
#!/usr/bin/env bash
touch "$PROMPTFOO_INVOKED"
SH
  chmod +x "$TMPROOT/bin/promptfoo"

  run env EVAL_OUT_DIR="$TMPROOT/out" CODEX_API_KEY="$secret" \
    PROMPTFOO_BIN="$TMPROOT/bin/promptfoo" PROMPTFOO_INVOKED="$TMPROOT/invoked" \
    "$ROOT/scripts/evals/share.sh"

  [ "$status" -eq 86 ]
  [ ! -e "$TMPROOT/invoked" ]
  [[ "$output" == *"provider eval artifacts failed secret scanning"* ]]
  [[ "$output" != *"$secret"* ]]
}

@test "share wrapper refuses an artifact containing a dynamic host path" {
  host_workspace="$TMPROOT/private-provider-workspace"
  printf '{"testCase":{"vars":{"context":"%s/private"}}}\n' \
    "$host_workspace" >"$TMPROOT/out/results.json"
  node "$ROOT/scripts/evals/artifact-scan-receipt.mjs" write \
    "$TMPROOT/out/artifact-scan-receipt.json" \
    "$TMPROOT/out/results.json"
  cat >"$TMPROOT/bin/promptfoo" <<'SH'
#!/usr/bin/env bash
touch "$PROMPTFOO_INVOKED"
SH
  chmod +x "$TMPROOT/bin/promptfoo"

  run env EVAL_OUT_DIR="$TMPROOT/out" \
    EVAL_PROVIDER_WORKSPACE="$host_workspace" \
    PROMPTFOO_BIN="$TMPROOT/bin/promptfoo" \
    PROMPTFOO_INVOKED="$TMPROOT/invoked" \
    "$ROOT/scripts/evals/share.sh"

  [ "$status" -eq 86 ]
  [ ! -e "$TMPROOT/invoked" ]
  [[ "$output" == *"provider eval artifacts failed secret scanning"* ]]
  [[ "$output" != *"$host_workspace"* ]]
}

@test "share wrapper refuses artifacts changed after their successful scan receipt" {
  prior_seeded_secret="opaque-prior-seeded-codex-credential"
  printf '{"response":"%s"}\n' "$prior_seeded_secret" \
    >"$TMPROOT/out/results.json"
  cat >"$TMPROOT/bin/promptfoo" <<'SH'
#!/usr/bin/env bash
touch "$PROMPTFOO_INVOKED"
SH
  chmod +x "$TMPROOT/bin/promptfoo"

  run env EVAL_OUT_DIR="$TMPROOT/out" \
    PROMPTFOO_BIN="$TMPROOT/bin/promptfoo" \
    PROMPTFOO_INVOKED="$TMPROOT/invoked" \
    "$ROOT/scripts/evals/share.sh"

  [ "$status" -eq 2 ]
  [ ! -e "$TMPROOT/invoked" ]
  [[ "$output" == *"provider eval artifact scan receipt is invalid"* ]]
  [[ "$output" != *"$prior_seeded_secret"* ]]
}

@test "share wrapper buffers and rejects credential-bearing share output" {
  secret="codex-share-output-that-must-not-escape"
  cat >"$TMPROOT/bin/promptfoo" <<'SH'
#!/usr/bin/env bash
printf 'sharing with %s\n' "$CODEX_API_KEY"
printf 'https://promptfoo.example/private\n'
SH
  chmod +x "$TMPROOT/bin/promptfoo"

  run env EVAL_OUT_DIR="$TMPROOT/out" \
    CODEX_API_KEY="$secret" \
    PROMPTFOO_BIN="$TMPROOT/bin/promptfoo" \
    "$ROOT/scripts/evals/share.sh"

  [ "$status" -eq 86 ]
  [[ "$output" != *"$secret"* ]]
  [[ "$output" != *"https://promptfoo.example/private"* ]]
}

@test "share wrapper buffers and rejects host paths in share output" {
  host_workspace="$TMPROOT/private-share-workspace"
  cat >"$TMPROOT/bin/promptfoo" <<'SH'
#!/usr/bin/env bash
printf 'sharing from %s/private\n' "$EVAL_PROVIDER_WORKSPACE"
SH
  chmod +x "$TMPROOT/bin/promptfoo"

  run env EVAL_OUT_DIR="$TMPROOT/out" \
    EVAL_PROVIDER_WORKSPACE="$host_workspace" \
    PROMPTFOO_BIN="$TMPROOT/bin/promptfoo" \
    "$ROOT/scripts/evals/share.sh"

  [ "$status" -eq 86 ]
  [[ "$output" != *"$host_workspace"* ]]
}

@test "share wrapper refuses non-private artifact paths before invoking promptfoo" {
  chmod 755 "$TMPROOT/out"
  cat >"$TMPROOT/bin/promptfoo" <<'SH'
#!/usr/bin/env bash
touch "$PROMPTFOO_INVOKED"
SH
  chmod +x "$TMPROOT/bin/promptfoo"

  run env EVAL_OUT_DIR="$TMPROOT/out" PROMPTFOO_BIN="$TMPROOT/bin/promptfoo" \
    PROMPTFOO_INVOKED="$TMPROOT/invoked" "$ROOT/scripts/evals/share.sh"

  [ "$status" -eq 2 ]
  [ ! -e "$TMPROOT/invoked" ]
  [[ "$output" == *"provider eval artifact path is not private"* ]]
}
