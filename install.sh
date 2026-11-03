#!/bin/sh
# Install Development System and Tiber from main with optional companions.
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
run codex plugin add "tiber@$marketplace"
if [ "$dry_run" = false ]; then
  installed_plugins="$(codex plugin list --json)"
  plugin_data="$(codex -C /tmp mcp list --json | python3 -c '
import json,os,sys
installed={p.get("pluginId"):p for p in json.loads(sys.argv[1]).get("installed", [])}
servers={server.get("name"):server for server in json.load(sys.stdin)}
data={}
for server,plugin in (("development-discipline","development-system"),("tiber","tiber")):
 version=installed.get(plugin+"@ai-plugins",{}).get("version","")
 env=servers.get(server,{}).get("transport",{}).get("env",{})
 if not version or (plugin != "tiber" and not env.get("PLUGIN_ROOT", "").endswith("/"+plugin+"/"+version)):
  sys.exit(f"install: {server} is not provided by the installed {plugin} plugin")
 directory=env.get("PLUGIN_DATA", "")
 if not os.path.isabs(directory): sys.exit(f"install: {plugin} has no absolute PLUGIN_DATA")
 data[plugin]=directory
if data["development-system"] == data["tiber"]:
 sys.exit("install: companion plugins must have separate data directories")
print(data["tiber"])
' "$installed_plugins")"
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

# Codex filters SSH_AUTH_SOCK from plugin transports and portable MCP has no
# env_vars field. Use one complete user-level replacement, never per-project
# entries or a persisted session socket. Reapply after every plugin upgrade.
if [ "$dry_run" = true ]; then
  printf '%s\n' 'install: configure the installed Tiber transport with env_vars = ["SSH_AUTH_SOCK"]'
else
  tiber_root="$(printf '%s' "$installed_plugins" | python3 -c '
import json,os,re,sys
version=next(p["version"] for p in json.load(sys.stdin)["installed"] if p.get("pluginId")=="tiber@ai-plugins")
if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+(?:[-+][A-Za-z0-9.+-]+)?", version):
 sys.exit("install: invalid installed Tiber version")
home=os.environ.get("CODEX_HOME", os.path.expanduser("~/.codex"))
if not os.path.isabs(home): sys.exit("install: CODEX_HOME must be absolute")
print(os.path.join(home, "plugins/cache/ai-plugins/tiber", version))
')"
  run python3 "$tiber_root/scripts/configure-codex-mcp.py" --plugin-data "$plugin_data" --apply
fi
printf 'install: restart Codex from a shell with the current SSH agent; repeat after agent rotation\n'
