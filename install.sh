#!/bin/sh
# Install this marketplace's Development System plugin from main with optional companions.
set -eu

repo='https://github.com/jwilger/ai-plugins.git'
marketplace='ai-plugins'
dry_run=false
selection='prompt'
while [ "$#" -gt 0 ]; do
  case "$1" in
    --dry-run) dry_run=true ;;
    --with)
      shift
      [ "$#" -gt 0 ] || { echo 'install: --with requires github,coderabbit or none' >&2; exit 2; }
      selection="$1"
      ;;
    --help)
      echo 'Usage: install.sh [--dry-run] [--with github,coderabbit|none]'
      exit 0
      ;;
    *) echo "install: unknown argument: $1" >&2; exit 2 ;;
  esac
  shift
done

command -v codex >/dev/null 2>&1 || { echo 'install: Codex CLI is required' >&2; exit 1; }
command -v python3 >/dev/null 2>&1 || { echo 'install: Python 3 is required for Codex inventory' >&2; exit 1; }
marketplaces="$(codex plugin marketplace list --json)"
plugins="$(codex plugin list --json)"
marketplace_state="$(printf '%s' "$marketplaces" | python3 -c '
import json,sys
for item in json.load(sys.stdin)["marketplaces"]:
 if item["name"] == "ai-plugins":
  source = item.get("marketplaceSource", {})
  print(source.get("sourceType", "local") + ":" + source.get("source", ""))
  break
')"
case "$marketplace_state" in
  '') marketplace_action='add' ;;
  "git:$repo"|'git:https://github.com/jwilger/ai-plugins') marketplace_action='upgrade' ;;
  *) echo "install: ai-plugins marketplace points elsewhere: $marketplace_state" >&2; exit 1 ;;
esac
if [ "$marketplace_action" = upgrade ]; then
  codex_config="${CODEX_HOME:-${HOME:-}/.codex}/config.toml"
  if [ -f "$codex_config" ]; then
    configured_ref="$(python3 -c '
import re,sys
section=False
for line in open(sys.argv[1], encoding="utf-8"):
 if re.match(r"^\[marketplaces\.ai-plugins\]\s*$", line): section=True; continue
 if section and line.startswith("["): break
 if section:
  match=re.match(r"^\s*ref\s*=\s*[\"\x27]([^\"\x27]+)[\"\x27]", line)
  if match: print(match.group(1)); break
' "$codex_config")"
    case "$configured_ref" in
      ''|main) ;;
      *) echo "install: ai-plugins tracks ref $configured_ref; configure main before updating" >&2; exit 1 ;;
    esac
  fi
fi

if [ "$selection" = prompt ]; then
  if (: </dev/tty) 2>/dev/null; then
    printf 'Optional companions (already installed items are skipped):\n  1. GitHub\n  2. CodeRabbit\nSelect numbers separated by spaces, or press Enter for neither: ' >/dev/tty
    IFS= read -r answer </dev/tty || answer=''
    selection='none'
    for choice in $answer; do
      case "$choice" in
        1)
          if [ "$selection" = coderabbit ]; then selection='github,coderabbit'; else selection='github'; fi
          ;;
        2)
          if [ "$selection" = none ]; then selection='coderabbit'; else selection='github,coderabbit'; fi
          ;;
        *) echo "install: invalid checklist selection: $choice" >&2; exit 2 ;;
      esac
    done
  else
    selection='none'
  fi
fi
case "$selection" in
  none|github|coderabbit|github,coderabbit|coderabbit,github) ;;
  *) echo "install: invalid --with selection: $selection" >&2; exit 2 ;;
esac

run() {
  printf '+ '
  printf '%s ' "$@"
  printf '\n'
  if [ "$dry_run" = false ]; then "$@"; fi
}
if [ "$marketplace_action" = add ]; then
  run codex plugin marketplace add jwilger/ai-plugins --ref main
else
  run codex plugin marketplace upgrade "$marketplace"
fi
run codex plugin add "development-system@$marketplace"
if [ "$dry_run" = false ]; then
  installed_version="$(codex plugin list --json | python3 -c '
import json,sys
for plugin in json.load(sys.stdin).get("installed", []):
 if plugin.get("pluginId") == "development-system@ai-plugins":
  print(plugin.get("version", ""))
  break
')"
  [ -n "$installed_version" ] || { echo 'install: Development System is not installed' >&2; exit 1; }
  plugin_data="$(codex -C /tmp mcp list --json | python3 -c '
import json,sys
servers={server.get("name"):server for server in json.load(sys.stdin)}
data=[]
for name in ("development-discipline", "tiber"):
 env=servers.get(name,{}).get("transport",{}).get("env",{})
 if not env.get("PLUGIN_ROOT", "").endswith("/development-system/" + sys.argv[1]):
  sys.exit(f"install: {name} is not provided by the installed portable plugin")
 data.append(env.get("PLUGIN_DATA", ""))
if not data[0] or data[0] != data[1]: sys.exit("install: plugin MCP data directories disagree")
print(data[0])
' "$installed_version")"
  [ "${plugin_data#/}" != "$plugin_data" ] || { echo 'install: Codex did not expose Development System PLUGIN_DATA' >&2; exit 1; }
fi

for name in github coderabbit; do
  case ",$selection," in *",$name,"*) ;; *) continue ;; esac
  state="$(printf '%s' "$plugins" | python3 -c '
import json,sys
name=sys.argv[1]
items=json.load(sys.stdin)
for item in items.get("installed",[]):
 if item.get("name")==name:
  print("installed")
  break
else:
 for item in items.get("available",[]):
  if item.get("name")==name:
   print(item.get("pluginId", ""))
   break
' "$name")"
  if [ "$state" = installed ]; then
    printf 'install: %s already installed\n' "$name"
  elif [ -n "$state" ]; then
    run codex plugin add "$state"
  else
    echo "install: $name was not found in the configured Codex catalogs" >&2
    exit 1
  fi
done

if [ -n "${SSH_AUTH_SOCK:-}" ] && [ -S "$SSH_AUTH_SOCK" ]; then
  if [ "$dry_run" = true ]; then
    socket_file='${PLUGIN_DATA}/signing-agent-socket'
  else
    socket_file="$plugin_data/signing-agent-socket"
  fi
  printf 'install: signing agent socket %s\n' "$socket_file"
  if [ "$dry_run" = false ]; then
    (
      umask 077
      mkdir -p "${socket_file%/*}"
      printf '%s\n' "$SSH_AUTH_SOCK" >"$socket_file"
      chmod 600 "$socket_file"
    )
  fi
fi
printf 'install: restart Codex to load the updated plugin and its MCP servers\n'
