import getpass,json,os,pathlib,shlex,shutil,socket,subprocess,sys,tempfile,time,tomllib

# Disposable loopback SSH transport; never touches the caller's agent or keys.
source=pathlib.Path(sys.argv[1]).resolve()
binary=pathlib.Path(sys.argv[2]).resolve()
root=pathlib.Path(tempfile.mkdtemp(prefix='tiber-ssh-')); root.chmod(0o700)
processes=[]
def run(args,env=None,check=True,cwd=None,input=None):
 r=subprocess.run(list(map(str,args)),env=env,cwd=cwd,text=True,capture_output=True,timeout=30,input=input)
 if check and r.returncode: raise RuntimeError(f'{args[0]} failed: '+r.stderr)
 return r
try:
 for name in ['host','client']: run(['ssh-keygen','-q','-t','ed25519','-N','','-f',root/name])
 agent=subprocess.Popen(['ssh-agent','-D','-a',str(root/'agent.sock')],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL); processes.append(agent)
 for _ in range(100):
  if (root/'agent.sock').exists():break
  time.sleep(.02)
 env=dict(os.environ,SSH_AUTH_SOCK=str(root/'agent.sock'),GIT_CONFIG_GLOBAL='/dev/null',GIT_CONFIG_NOSYSTEM='1')
 run(['ssh-add',root/'client'],env=env)
 (root/'client').unlink()
 with socket.socket() as reservation: reservation.bind(('127.0.0.1',0)); port=reservation.getsockname()[1]
 (root/'sshd.conf').write_text(f'Port {port}\nListenAddress 127.0.0.1\nHostKey {root}/host\nAuthorizedKeysFile {root}/client.pub\nStrictModes no\nUsePAM no\nPasswordAuthentication no\nKbdInteractiveAuthentication no\nPidFile {root}/sshd.pid\nLogLevel ERROR\nAllowAgentForwarding no\nDisableForwarding yes\nPermitTTY no\nPermitUserEnvironment no\n')
 with (root/'sshd.log').open('w') as log:
  server=subprocess.Popen([shutil.which('sshd'),'-D','-e','-f',str(root/'sshd.conf')],stdout=log,stderr=log);processes.append(server)
 for _ in range(100):
  try:
   with socket.create_connection(('127.0.0.1',port),timeout=.1):break
  except OSError:time.sleep(.02)
 if server.poll() is not None: raise RuntimeError((root/'sshd.log').read_text())
 (root/'known_hosts').write_text(f'[127.0.0.1]:{port} '+(root/'host.pub').read_text())
 ssh=[shutil.which('ssh'),'-F','/dev/null','-o','BatchMode=yes','-o','IdentitiesOnly=yes','-i',str(root/'client.pub'),'-o','StrictHostKeyChecking=yes','-o',f'UserKnownHostsFile={root}/known_hosts','-p',str(port)]
 env['GIT_SSH_COMMAND']=shlex.join(ssh)
 noagent={k:v for k,v in env.items() if k!='SSH_AUTH_SOCK'}
 assert run([*ssh,f'{getpass.getuser()}@127.0.0.1','true'],env=noagent,check=False).returncode!=0
 run([*ssh,f'{getpass.getuser()}@127.0.0.1','true'],env=env)
 run(['git','init','--bare',root/'origin.git'])
 version=json.loads((source/'plugins/tiber/plugin.json').read_text())['version']
 cache=root/'codex/plugins/cache/ai-plugins/tiber'/version
 for directory in ['bin','lib']:shutil.copytree(source/'plugins/tiber'/directory,cache/directory)
 shutil.copy2(source/'plugins/tiber/plugin.json',cache/'plugin.json')
 (cache/'scripts').mkdir()
 shutil.copy2(source/'plugins/tiber/scripts/configure-codex-mcp.py',cache/'scripts/configure-codex-mcp.py')
 data=root/'data';host=os.uname().sysname.lower()+'-'+os.uname().machine
 install=data/'ai-plugins/tiber'/version/host;install.mkdir(parents=True)
 shutil.copy2(binary,install/'tiber');(install/'.plugin-version').write_text(version)
 config_env=dict(env,CODEX_HOME=str(root/'codex'))
 run(['python3',cache/'scripts/configure-codex-mcp.py','--plugin-data',data,'--apply'],env=config_env)
 config=tomllib.loads((root/'codex/config.toml').read_text())['mcp_servers']['tiber']
 def transport(parent):
  result={k:v for k,v in parent.items() if k!='SSH_AUTH_SOCK'}
  result.update(config['env'])
  for key in config['env_vars']:
   if key in parent:result[key]=parent[key]
  return result
 def mcp(repo,name,arguments,parent):
  request={'jsonrpc':'2.0','id':1,'method':'tools/call','params':{'name':name,'arguments':arguments,'_meta':{'codex/sandbox-state-meta':{'sandboxCwd':repo.as_uri()}}}}
  response=run([config['command'],*config['args']],env=transport(parent),cwd=cache,input=json.dumps(request)+'\n')
  return json.loads(response.stdout)
 for index in range(2):
  origin=root/f'origin-{index}.git';run(['git','init','--bare',origin])
  repo=root/f'repo-{index}';run(['git','init',repo])
  for key,value in [('user.name','Synthetic Fixture'),('user.email','fixture@example.invalid')]:run(['git','-C',repo,'config',key,value])
  run(['git','-C',repo,'commit','--allow-empty','--no-gpg-sign','-m','Fixture'])
  remote=f'ssh://{getpass.getuser()}@127.0.0.1:{port}{origin}'
  run(['git','-C',repo,'remote','add','origin',remote])
  run(['git','-C',repo,'push','origin','HEAD:main'],env=env)
  run([binary,'init'],env=env,cwd=repo)
  for key,value in [('gpg.format','ssh'),('user.signingkey',str(root/'client.pub')),('commit.gpgsign','true')]:run(['git','-C',repo,'config',key,value])
  missing=mcp(repo,'tiber.list',{},noagent)
  diagnostic=json.dumps(missing)
  assert 'tiber.ssh_authentication_failed' in diagnostic,diagnostic
  assert 'SSH_AUTH_SOCK' in diagnostic and 'source_redacted=true' in diagnostic,diagnostic
  for sensitive in [str(root),remote,'Permission denied (publickey)']:
   assert sensitive not in diagnostic,diagnostic
  before=run(['git','--git-dir',origin,'rev-parse','refs/heads/tiber']).stdout.strip()
  created=mcp(repo,'tiber.create',{'title':f'SSH fixture {index}'},env)
  assert 'error' not in created and not created.get('result',{}).get('isError'),created
  after=run(['git','--git-dir',origin,'rev-parse','refs/heads/tiber']).stdout.strip()
  assert before!=after
  assert 'gpgsig ' in run(['git','--git-dir',origin,'cat-file','-p',after]).stdout
  listed=mcp(repo,'tiber.list',{},env)
  assert f'SSH fixture {index}' in json.dumps(listed),listed
 print(json.dumps({'missing_agent':'safe_actionable_failure','forwarded_agent':'signed_SSH_fetch_push_passed','caller_repositories':2,'private_client_key_removed_after_agent_load':True}))

except Exception:
 if (root/'sshd.log').exists():print((root/'sshd.log').read_text()[-2000:])
 raise
finally:
 for process in reversed(processes):
  process.terminate()
  try:process.wait(timeout=3)
  except subprocess.TimeoutExpired:process.kill();process.wait()
 shutil.rmtree(root)
