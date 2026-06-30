#!/usr/bin/env bash
# On session end, notice any side-quests still running in the background.
#
# Side-quests run in detached workers, so they survive the session ending; this
# is a reminder, not a blocker.
set -uo pipefail

registry=".git/sidequest/registry.json"
[ -f "$registry" ] || exit 0

count=$(grep -cE '"state": *"(running|awaiting-input)"' "$registry" 2>/dev/null || true)
if [ "${count:-0}" -gt 0 ]; then
  echo "sidequest: ${count} side-quest(s) still running in the background; they continue after this session. Check them with the side-quest list tool." >&2
fi
exit 0
