#!/usr/bin/env bash
set -euo pipefail

plugin_root="${1:-"$(cd "$(dirname "${BASH_SOURCE[0]}")/../plugins/development-system" && pwd)"}"

python3 - "$plugin_root" <<'PY'
import json
import pathlib
import sys
import tomllib

root = pathlib.Path(sys.argv[1])
expected = {
    "bounded-helper": "read-only",
    "substantive-worker": "read-only",
    "strong-reviewer": "read-only",
    "strong-worker": "read-only",
    "advisor": "read-only",
}

agents = {}
for name, sandbox in expected.items():
    path = root / "agents" / f"{name}.toml"
    try:
        agent = tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, tomllib.TOMLDecodeError) as error:
        raise SystemExit(f"model-routing-config: invalid-agent: {path}: {error}")
    for key in ("model", "model_reasoning_effort", "reasoning_effort"):
        if key in agent:
            raise SystemExit(f"model-routing-config: hardcoded-route: {path}:{key}")
    if agent.get("sandbox_mode") != sandbox:
        raise SystemExit(f"model-routing-config: invalid-sandbox: {path}")
    if agent.get("name") != ("advisor" if name == "advisor" else f"model-routing-{name}"):
        raise SystemExit(f"model-routing-config: invalid-name: {path}")
    agents[name] = {"sandbox": sandbox}

print(json.dumps({"codex": agents}, separators=(",", ":")))
PY
