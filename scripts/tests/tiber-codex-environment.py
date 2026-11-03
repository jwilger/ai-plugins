#!/usr/bin/env python3
"""Bounded real-Codex MCP environment probe. No auth copies or model turns.

Run manually with Python 3.11+ and Codex on PATH. All homes, configuration,
servers and synthetic socket values are disposable. Requires app-server startup
network access on Codex versions that refresh plugin metadata at initialization.
"""
import json
import os
from pathlib import Path
import selectors
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import tomllib

SOURCE = Path(__file__).resolve().parents[2]
FAKE = '''#!/usr/bin/env python3
import json,os,pathlib,sys
pathlib.Path(os.environ['PROBE_OUTPUT']).write_text(json.dumps(os.environ.get('SSH_AUTH_SOCK')))
for line in sys.stdin:
 try: r=json.loads(line)
 except ValueError: continue
 if 'id' not in r: continue
 result={'protocolVersion':r.get('params',{}).get('protocolVersion','2024-11-05'),'capabilities':{'tools':{}},'serverInfo':{'name':'probe','version':'1'}} if r.get('method')=='initialize' else {'tools':[]} if r.get('method')=='tools/list' else {}
 print(json.dumps({'jsonrpc':'2.0','id':r['id'],'result':result}),flush=True)
'''


def launch(home, root, socket, names):
    for name in names:
        (root / (name + '.json')).unlink(missing_ok=True)
    env = dict(os.environ, CODEX_HOME=str(home))
    env.pop('SSH_AUTH_SOCK', None)
    if socket is not None:
        env['SSH_AUTH_SOCK'] = socket
    with (root / 'stderr.log').open('w') as err:
        process = subprocess.Popen(['codex', 'app-server', '--stdio'], env=env,
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=err,
            text=True, start_new_session=True)
        selector = selectors.DefaultSelector()
        selector.register(process.stdout, selectors.EVENT_READ)
        def send(value):
            process.stdin.write(json.dumps(value) + '\n')
            process.stdin.flush()
        try:
            send({'id': 1, 'method': 'initialize', 'params': {'clientInfo': {'name': 'tiber-env-probe', 'version': '1'}, 'capabilities': {'experimentalApi': True}}})
            deadline = time.monotonic() + 30
            while time.monotonic() < deadline and process.poll() is None:
                for key, _ in selector.select(.2):
                    line = key.fileobj.readline()
                    if not line:
                        continue
                    event = json.loads(line)
                    if event.get('id') == 1:
                        send({'method': 'initialized', 'params': {}})
                        send({'id': 2, 'method': 'thread/start', 'params': {'cwd': str(root), 'approvalPolicy': 'never', 'sandbox': 'workspace-write', 'ephemeral': True}})
                if all((root / (name + '.json')).exists() for name in names):
                    return {name: json.loads((root / (name + '.json')).read_text()) for name in names}
            raise RuntimeError('MCP startup probe timed out; no model turn was sent')
        finally:
            selector.close()
            try:
                os.killpg(process.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
            try:
                process.wait(timeout=4)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait()


def portable_bootstrap_upgrade():
    with tempfile.TemporaryDirectory(prefix='tiber-portable-bootstrap-') as temporary:
        root = Path(temporary)
        home = root / 'home'
        home.mkdir()
        (home / 'config.toml').write_text('[features]\napps=false\n')
        market = root / 'market'
        plugin = market / 'plugins/tiber'
        plugin.mkdir(parents=True)
        for name in ['bin', 'lib', 'scripts']:
            shutil.copytree(SOURCE / 'plugins/tiber' / name, plugin / name)
        for name in ['plugin.json', 'mcp.json']:
            shutil.copy2(SOURCE / 'plugins/tiber' / name, plugin / name)
        manifest = json.loads((plugin / 'plugin.json').read_text())
        version = manifest['version']
        metadata = market / '.agents/plugins'
        metadata.mkdir(parents=True)
        catalog = {'name': 'ai-plugins', 'interface': {'displayName': 'Synthetic probe'},
                   'plugins': [{'name': 'tiber', 'version': version,
                                'source': {'source': 'local', 'path': './plugins/tiber'}}]}
        (metadata / 'marketplace.json').write_text(json.dumps(catalog))
        env = dict(os.environ, CODEX_HOME=str(home))
        env.pop('SSH_AUTH_SOCK', None)
        for args in [['plugin', 'marketplace', 'add', str(market), '--json'],
                     ['plugin', 'add', 'tiber@ai-plugins', '--json']]:
            subprocess.run(['codex', *args], env=env, cwd=root, capture_output=True, check=True)
        inventory = json.loads(subprocess.check_output(
            ['codex', '-C', '/tmp', 'mcp', 'list', '--json'], env=env, text=True))
        data = Path(next(item for item in inventory if item['name'] == 'tiber')['transport']['env']['PLUGIN_DATA'])
        native = data / 'ai-plugins/tiber' / version / (os.uname().sysname.lower() + '-' + os.uname().machine)
        native.mkdir(parents=True)
        executable = native / 'tiber'
        executable.write_text(FAKE.replace("os.environ['PROBE_OUTPUT']", repr(str(root / 'tiber.json'))))
        executable.chmod(0o700)
        (native / '.plugin-version').write_text(version)
        first = launch(home, root, '/synthetic/current', ['tiber'])
        config = tomllib.loads((home / 'config.toml').read_text())
        assert config['mcp_servers']['tiber']['command'] == str(data / 'mcp-dispatch.py'), config
        assert first['tiber'] is None, first
        second = launch(home, root, '/synthetic/current', ['tiber'])
        assert second['tiber'] == '/synthetic/current', second
        old_cache = home / 'plugins/cache/ai-plugins/tiber' / version
        manifest['version'] = '99.0.0'
        (plugin / 'plugin.json').write_text(json.dumps(manifest, indent=2))
        catalog['plugins'][0]['version'] = '99.0.0'
        (metadata / 'marketplace.json').write_text(json.dumps(catalog))
        subprocess.run(['codex', 'plugin', 'add', 'tiber@ai-plugins', '--json'],
                       env=env, cwd=root, capture_output=True, check=True)
        new_native = native.parent.parent / '99.0.0' / native.name
        shutil.copytree(native, new_native)
        (new_native / '.plugin-version').write_text('99.0.0')
        # Codex may already have removed the old version during the upgrade.
        if old_cache.exists():
            shutil.rmtree(old_cache)
        third = launch(home, root, '/synthetic/rotated', ['tiber'])
        assert third['tiber'] == '/synthetic/rotated', third
        assert tomllib.loads((home / 'config.toml').read_text()) == config
        print(json.dumps({'portable_bootstrap': 'passed', 'real_upgrade_deleted_cache': 'passed',
                          'initial_filtered': True, 'next_parent_launch_forwarded': True, 'model_turns': 0}))


def main():
    portable_bootstrap_upgrade()
    version = subprocess.check_output(['codex', '--version'], text=True).strip()
    with tempfile.TemporaryDirectory(prefix='tiber-codex-environment-') as directory:
        root = Path(directory)
        home = root / 'home'
        home.mkdir()
        fake = root / 'fake'
        fake.write_text(FAKE)
        fake.chmod(0o700)
        q = json.dumps
        config = '[analytics]\nenabled = false\n[features]\napps = false\n'
        for name, extra in [('inherited', ''), ('literal', 'SSH_AUTH_SOCK = "${SSH_AUTH_SOCK}"'), ('allowlist', '')]:
            config += f'\n[mcp_servers.{name}]\ncommand = {q(str(fake))}\n'
            if name == 'allowlist':
                config += 'env_vars = ["SSH_AUTH_SOCK"]\n'
            config += f'env = {{ PROBE_OUTPUT = {q(str(root / (name + ".json")))}'
            config += (', ' + extra if extra else '') + ' }\n'
        (home / 'config.toml').write_text(config)
        names = ['inherited', 'literal', 'allowlist']
        observed = launch(home, root, '/synthetic/current-agent', names)
        assert observed == {'inherited': None, 'literal': '${SSH_AUTH_SOCK}', 'allowlist': '/synthetic/current-agent'}, observed
        # Exercise the actual installed-helper configuration across socket changes.
        manifest = json.loads((SOURCE / 'plugins/tiber/plugin.json').read_text())
        installed = home / 'plugins/cache/ai-plugins/tiber' / manifest['version']
        (installed / 'scripts').mkdir(parents=True)
        (installed / 'bin').mkdir()
        for script in (SOURCE / 'plugins/tiber/scripts').glob('*.py'):
            shutil.copy2(script, installed / 'scripts')
        (installed / 'plugin.json').write_text(json.dumps(manifest))
        launcher = installed / 'bin/tiber'
        launcher.write_text(FAKE.replace("os.environ['PROBE_OUTPUT']", repr(str(root / 'tiber.json'))))
        launcher.chmod(0o700)
        # Register the fixture through real Codex so dispatcher inventory includes
        # actual installed/enabled state, not a handwritten approximation.
        market = root / 'market'
        (market / '.agents/plugins').mkdir(parents=True)
        shutil.copytree(installed, market / 'plugins/tiber')
        (market / '.agents/plugins/marketplace.json').write_text(json.dumps({
            'name': 'ai-plugins', 'interface': {'displayName': 'Synthetic probe'},
            'plugins': [{'name': 'tiber', 'version': manifest['version'],
                         'source': {'source': 'local', 'path': './plugins/tiber'}}]}))
        fixture_env = dict(os.environ, CODEX_HOME=str(home))
        fixture_env.pop('SSH_AUTH_SOCK', None)
        for command in [['codex', 'plugin', 'marketplace', 'add', str(market), '--json'],
                        ['codex', 'plugin', 'add', 'tiber@ai-plugins', '--json']]:
            subprocess.run(command, env=fixture_env, cwd=root, check=True, capture_output=True)
        subprocess.run([sys.executable, str(installed / 'scripts/configure-codex-mcp.py'), '--plugin-data', str(root / 'data'), '--apply'], env=dict(os.environ, CODEX_HOME=str(home)), check=True, capture_output=True)
        for socket in ['/synthetic/current-agent', '/synthetic/rotated-agent', None]:
            result = launch(home, root, socket, names + ['tiber'])
            assert result['tiber'] == socket, result
        print(json.dumps({'codex': version, 'default_filtered': True, 'interpolation_literal': True, 'helper_current_rotated_unset': 'passed', 'model_turns': 0}))


if __name__ == '__main__':
    main()
