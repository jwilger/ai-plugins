#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd -P)"
  FIXTURE_ROOT="$(mktemp -d)"
  mkdir -p "$FIXTURE_ROOT/scripts/evals" "$FIXTURE_ROOT/tooling/evals/node_modules"
  cp "$ROOT/scripts/evals/ensure-node-deps.sh" "$FIXTURE_ROOT/scripts/evals/ensure-node-deps.sh"
  chmod +x "$FIXTURE_ROOT/scripts/evals/ensure-node-deps.sh"

  cat >"$FIXTURE_ROOT/tooling/evals/package-lock.json" <<'JSON'
{"name":"fixture","lockfileVersion":3}
JSON

  make_eval_dependencies "$FIXTURE_ROOT/tooling/evals/node_modules" stale
  printf 'outdated-lock-fingerprint\n' >"$FIXTURE_ROOT/tooling/evals/node_modules/.ai-plugins-eval-lock-fingerprint"

  mkdir -p "$FIXTURE_ROOT/bin"
  cat >"$FIXTURE_ROOT/bin/npm" <<'SH'
#!/usr/bin/env bash
set -euo pipefail

printf '%s\n' "$*" >>"$NPM_LOG"
prefix="$2"
modules="$prefix/node_modules"
rm -rf -- "$modules"
mkdir -p "$modules/.bin" "$modules/@openai/codex-sdk" "$modules/@anthropic-ai/claude-agent-sdk" "$modules/promptfoo"
printf '#!/usr/bin/env bash\n' >"$modules/.bin/promptfoo"
chmod +x "$modules/.bin/promptfoo"
printf '{"version":"0.144.5"}\n' >"$modules/@openai/codex-sdk/package.json"
printf '{"version":"0.3.201"}\n' >"$modules/@anthropic-ai/claude-agent-sdk/package.json"
printf '{"version":"0.121.19"}\n' >"$modules/promptfoo/package.json"
SH
  chmod +x "$FIXTURE_ROOT/bin/npm"
}

teardown() {
  rm -rf "$FIXTURE_ROOT"
}

make_eval_dependencies() {
  local modules="$1"
  local version="$2"
  mkdir -p "$modules/.bin" "$modules/@openai/codex-sdk" "$modules/@anthropic-ai/claude-agent-sdk" "$modules/promptfoo"
  printf '#!/usr/bin/env bash\n' >"$modules/.bin/promptfoo"
  chmod +x "$modules/.bin/promptfoo"
  printf '{"version":"%s"}\n' "$version" >"$modules/@openai/codex-sdk/package.json"
  printf '{"version":"%s"}\n' "$version" >"$modules/@anthropic-ai/claude-agent-sdk/package.json"
  printf '{"version":"%s"}\n' "$version" >"$modules/promptfoo/package.json"
}

@test "eval dependency bootstrap replaces a stale pinned installation once" {
  npm_log="$FIXTURE_ROOT/npm.log"

  run env \
    PATH="$FIXTURE_ROOT/bin:$PATH" \
    NPM_LOG="$npm_log" \
    "$FIXTURE_ROOT/scripts/evals/ensure-node-deps.sh"

  [ "$status" -eq 0 ]
  [ "$(cat "$npm_log")" = "--prefix $FIXTURE_ROOT/tooling/evals ci --ignore-scripts --no-audit --no-fund" ]
  [ "$(cat "$FIXTURE_ROOT/tooling/evals/node_modules/.ai-plugins-eval-lock-fingerprint")" = "$(sha256sum "$FIXTURE_ROOT/tooling/evals/package-lock.json" | awk '{print $1}')" ]
  [ "$(jq -r '.version' "$FIXTURE_ROOT/tooling/evals/node_modules/promptfoo/package.json")" = "0.121.19" ]
  [ -L "$FIXTURE_ROOT/node_modules" ]
  [ "$(readlink "$FIXTURE_ROOT/node_modules")" = "tooling/evals/node_modules" ]

  run env \
    PATH="$FIXTURE_ROOT/bin:$PATH" \
    NPM_LOG="$npm_log" \
    "$FIXTURE_ROOT/scripts/evals/ensure-node-deps.sh"

  [ "$status" -eq 0 ]
  [ "$(wc -l <"$npm_log")" -eq 1 ]
}
