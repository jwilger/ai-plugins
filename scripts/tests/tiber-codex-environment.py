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


def main():
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
