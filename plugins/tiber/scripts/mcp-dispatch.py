#!/usr/bin/env python3
"""Stable installed Tiber MCP entry point; never search caller PATH for Codex."""
import json
import os
from pathlib import Path
import re
import subprocess
import sys


def installed_root(codex, home):
    if not Path(codex).is_absolute() or not os.access(codex, os.X_OK):
        raise ValueError('codex_executable_unavailable')
    environment = dict(os.environ)
    environment.pop('SSH_AUTH_SOCK', None)
    environment['CODEX_HOME'] = str(home)
    result = subprocess.run([codex, '-C', '/tmp', 'plugin', 'list', '--json'],
                            cwd='/tmp', env=environment, capture_output=True, text=True, timeout=30)
    if result.returncode:
        raise ValueError('inventory_unavailable')
    matches = [item for item in json.loads(result.stdout)['installed']
               if item.get('pluginId') == 'tiber@ai-plugins' and item.get('installed') is True]
    if len(matches) != 1 or matches[0].get('enabled') is not True:
        raise ValueError('plugin_not_enabled')
    version = matches[0]['version']
    if not isinstance(version, str) or not re.fullmatch(r'[A-Za-z0-9_+.-]+', version) or version in ('.', '..'):
        raise ValueError('invalid_installed_version')
    cache = home / 'plugins/cache/ai-plugins/tiber'
    root = cache / version
    if root.is_symlink() or root.resolve().parent != cache.resolve() or not (root / 'bin/tiber').is_file():
        raise ValueError('installed_launcher_unavailable')
    if json.loads((root / 'plugin.json').read_text())['version'] != version and version != 'local':
        raise ValueError('installed_manifest_mismatch')
    return root.resolve()


def main():
    home = Path(os.environ['CODEX_HOME'])
    if not home.is_absolute():
        raise ValueError('invalid_codex_home')
    root = installed_root(os.environ['TIBER_CODEX_EXECUTABLE'], home)
    env = dict(os.environ, PLUGIN_ROOT=str(root), TIBER_AGENT_FORWARDING='current-session')
    launcher = str(root / 'bin/tiber')
    os.execve(launcher, [launcher, *sys.argv[1:]], env)


if __name__ == '__main__':
    try:
        main()
    except (ValueError, KeyError, TypeError, OSError, subprocess.SubprocessError) as error:
        allowed = {'codex_executable_unavailable', 'inventory_unavailable', 'plugin_not_enabled',
                   'invalid_installed_version', 'installed_launcher_unavailable', 'installed_manifest_mismatch',
                   'invalid_codex_home'}
        reason = str(error) if isinstance(error, ValueError) and str(error) in allowed else 'inventory_unavailable'
        print(f'tiber.mcp_dispatch_failed reason={reason} remediation=check_enabled_Tiber_installation_or_rerun_installed_configuration_helper', file=sys.stderr)
        sys.exit(1)
