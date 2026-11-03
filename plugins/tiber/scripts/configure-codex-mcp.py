#!/usr/bin/env python3
"""Converge the user-level Codex transport without persisting agent socket values."""

import argparse
import fcntl
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import tomllib

BEGIN = '# BEGIN ai-plugins:tiber current-session SSH-agent forwarding'
END = '# END ai-plugins:tiber current-session SSH-agent forwarding'


def fail(code, remediation):
    raise ValueError(f'tiber.mcp_configuration_failed reason={code} remediation={remediation}')


def configuration(original, root, data):
    parsed = tomllib.loads(original)
    starts, ends = original.count(BEGIN), original.count(END)
    if (starts, ends) not in ((0, 0), (1, 1)):
        fail('malformed_managed_block', 'repair_managed_block_markers')
    if starts:
        start, end = original.index(BEGIN), original.index(END) + len(END)
        if end <= start:
            fail('malformed_managed_block', 'repair_managed_block_markers')
        # Preserve every byte outside our explicitly owned block.
        prefix, suffix = original[:start], original[end:]
        if suffix.startswith('\n'):
            suffix = suffix[1:]
        remaining = prefix + suffix
        if 'tiber' in tomllib.loads(remaining).get('mcp_servers', {}):
            fail('unmanaged_registration', 'add_SSH_AUTH_SOCK_to_existing_env_vars_or_remove_duplicate')
    else:
        if 'tiber' in parsed.get('mcp_servers', {}):
            fail('unmanaged_registration', 'add_SSH_AUTH_SOCK_to_existing_env_vars_or_remove_duplicate')
        prefix, suffix = original + ('\n' if original and not original.endswith('\n') else ''), ''
    quote = lambda value: json.dumps(str(value), ensure_ascii=False)
    block = '\n'.join([
        BEGIN,
        '[mcp_servers.tiber]',
        f'command = {quote(root / "bin/tiber")}',
        'args = ["mcp", "stdio"]',
        'env_vars = ["SSH_AUTH_SOCK"]',
        'env = { PLUGIN_ROOT = ' + quote(root) + ', PLUGIN_DATA = ' + quote(data)
        + ', TIBER_AGENT_FORWARDING = "current-session" }',
        END,
        '',
    ])
    result = prefix + block + suffix
    tomllib.loads(result)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--apply', action='store_true', help='update the shared user configuration')
    parser.add_argument('--plugin-data', type=Path, help='absolute Tiber PLUGIN_DATA from Codex inventory')
    args = parser.parse_args()
    home = Path(os.environ.get('CODEX_HOME', str(Path.home() / '.codex')))
    root = Path(__file__).resolve().parents[1]
    version = json.loads((root / 'plugin.json').read_text())['version']
    expected = home / 'plugins/cache/ai-plugins/tiber' / version
    if not home.is_absolute() or root != expected.resolve() or not (root / 'bin/tiber').is_file():
        fail('untrusted_launcher_location', 'run_this_helper_from_the_installed_Tiber_plugin_cache')
    data = args.plugin_data
    if data is None:
        inventory = subprocess.run(['codex', '-C', '/tmp', 'mcp', 'list', '--json'], capture_output=True, text=True, timeout=30)
        if inventory.returncode:
            fail('inventory_unavailable', 'check_codex_mcp_list')
        servers = json.loads(inventory.stdout)
        data = next((Path((item['transport'].get('env') or {}).get('PLUGIN_DATA', '')) for item in servers if item.get('name') == 'tiber'), Path())
    if not data.is_absolute():
        fail('plugin_data_unavailable', 'supply_absolute_plugin_data_from_codex_mcp_list')
    target = home / 'config.toml'
    if target.is_symlink():
        fail('configuration_symlink', 'configure_env_vars_in_the_source_of_the_user_config')

    def prepare():
        original = target.read_text() if target.exists() else ''
        return original, configuration(original, root, data)

    if not args.apply:
        original, updated = prepare()
        if original != updated:
            print('tiber.mcp_configuration_needed remediation=rerun_installed_helper_with_--apply')
            return 1
        print('tiber.mcp_configuration_current agent_source=parent_env_vars')
        return 0
    home.mkdir(parents=True, exist_ok=True)
    # Serialize our own repairs; unrelated configuration writers must not run concurrently.
    with (home / '.tiber-mcp-config.lock').open('a') as lock:
        os.chmod(lock.name, 0o600)
        fcntl.flock(lock, fcntl.LOCK_EX)
        original, updated = prepare()
        if original != updated:
            pending = None
            try:
                with tempfile.NamedTemporaryFile(mode='w', dir=home, prefix='.tiber-config-', delete=False) as temporary:
                    pending = Path(temporary.name)
                    temporary.write(updated)
                    temporary.flush()
                    os.fsync(temporary.fileno())
                os.replace(pending, target)
                directory = os.open(home, os.O_RDONLY | os.O_DIRECTORY)
                try:
                    os.fsync(directory)
                finally:
                    os.close(directory)
            finally:
                if pending is not None:
                    pending.unlink(missing_ok=True)
    print('tiber.mcp_configuration_ready agent_source=parent_env_vars remediation=restart_Codex_from_current_agent_shell')
    return 0


if __name__ == '__main__':
    try:
        sys.exit(main())
    except (ValueError, KeyError, OSError, subprocess.SubprocessError):
        # No config content, paths, environment values or subprocess stderr escape here.
        error = sys.exc_info()[1]
        if isinstance(error, ValueError) and str(error).startswith('tiber.mcp_configuration_failed '):
            print(error, file=sys.stderr)
        else:
            print('tiber.mcp_configuration_failed reason=configuration_unavailable remediation=check_user_config_and_installed_plugin', file=sys.stderr)
        sys.exit(1)
