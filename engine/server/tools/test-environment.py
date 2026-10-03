#!/usr/bin/env python3
"""Real runtime + customized image lifecycle. No mock executables and no host commands."""
import argparse, base64, importlib, json, pathlib, sys, tempfile, time
sys.path.insert(0,str(pathlib.Path(__file__).parent));Client=importlib.import_module('test-protocol').Client

def poll(c, expected, seconds=240):
    deadline=time.monotonic()+seconds;last=None
    while time.monotonic()<deadline:
        status=c.call('environment.status',timeout=30)
        stage=(status['phase'],status.get('stage'),status.get('step'))
        if stage!=last:print('environment:',stage,flush=True);last=stage
        if status['phase'] in expected:return status
        if status['phase']=='failed':raise AssertionError(status)
        time.sleep(.2)
    raise TimeoutError(status)

def guest(c,argv,expected=0):
    p=c.call('process.spawn',{'argv':argv,'cwd':'/workspace','terminal':False})['processId']
    status=c.call('process.wait',{'processId':p,'timeoutMs':30000},timeout=35)
    assert status=={'running':False,'exitCode':expected},status
    result={}
    for stream in ['stdout','stderr']:
        data=b'';offset=0
        for _ in range(50):
            r=c.call('process.read',{'processId':p,'stream':stream,'offset':offset,'maxBytes':65536,'waitMs':1000});data+=base64.b64decode(r['data']);offset=r['nextOffset']
            if r['eof']:break
        result[stream]=data.decode(errors='replace')
    return result

def run(a,root):
    private=root/'.workspace';private.mkdir(parents=True)
    spec={'version':1,'python':['3.14'],'node':['24'],'packages':[],'env':{},'post_scripts':[{'id':'home-config','user':'work','run':'mkdir -p "$HOME/.config/workflow-test"; printf "script-live\\n" > "$HOME/.config/workflow-test/result"; python3 --version; node --version'}]}
    (private/'env.json').write_text(json.dumps(spec))
    c=Client(a.engine,root,['--runtime',a.runtime,'--loader',a.loader,'--image',a.image,'--image-index',a.index]);c.hello()
    start=time.monotonic();c.call('environment.reconcile');c.command('createSession',{'name':'files stay responsive during install'});assert time.monotonic()-start<2
    status=poll(c,{'ready'});first=status['active']['generation']
    out=guest(c,['/bin/sh','-c','cat "$HOME/.config/workflow-test/result"; python3 --version; node --version'])
    assert 'script-live' in out['stdout'] and 'Python 3.14.' in out['stdout'] and 'v24.' in out['stdout'],out
    assert not list((private/'environment/generations').glob('*/post-home'))
    print('real customized image/default toolchains/staged work-home activation:',out['stdout'].splitlines(),flush=True)
    # A failed script wrote only to its private staged home.
    spec['post_scripts']=[{'id':'failure','user':'work','run':'printf "must-not-publish" > "$HOME/.config/workflow-test/result"; exit 7'}]
    (private/'env.json').write_text(json.dumps(spec));c.call('environment.reconcile')
    status=poll(c,{'failed'});assert status['usable'] and status['active']['generation']==first,status
    assert (private/'environment/stores/home/work/.config/workflow-test/result').read_text()=='script-live\n'
    assert not list((private/'environment/generations').glob('*/post-home'))
    # An empty language list really disables that language; cached installs remain available for later profiles.
    spec['post_scripts']=[];spec['node']=[];(private/'env.json').write_text(json.dumps(spec));c.call('environment.reconcile');status=poll(c,{'ready'})
    assert status['active']['verified']['node']==[],status
    out=guest(c,['/bin/sh','-c','python3 --version; node --version'],expected=127)
    assert 'Python 3.14.' in out['stdout'],out
    # A live process pins the current environment until an explicit restart.
    p=c.call('process.spawn',{'argv':['/bin/sleep','300']})['processId'];old=status['active']['generation']
    spec['env']={'WORKFLOW_TEST_VALUE':'new-value'};(private/'env.json').write_text(json.dumps(spec));c.call('environment.reconcile');status=poll(c,{'needs_restart'})
    assert status['active']['generation']==old and c.call('process.wait',{'processId':p})['running'],status
    status=c.call('environment.restart',timeout=30);assert status['phase']=='ready' and status['active']['generation']!=old,status
    assert not c.call('process.wait',{'processId':p})['running']
    out=guest(c,['/bin/sh','-c','printf "%s" "$WORKFLOW_TEST_VALUE"; cat "$HOME/.config/workflow-test/result"'])
    assert out['stdout']=='new-valuescript-live\n',out
    current=status['active']['generation'];c.close()
    c=Client(a.engine,root,['--runtime',a.runtime,'--loader',a.loader,'--image',a.image,'--image-index',a.index]);c.hello()
    assert c.call('environment.status')['active']['generation']==current
    assert guest(c,['/bin/sh','-c','test -f "$HOME/.config/workflow-test/result"'])['stderr']==''
    c.close()
    print('Real lifecycle PASS: script failure rollback, cache/home preservation, disabled Node, pending generation, explicit process restart, server restart.',flush=True)

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('engine',type=pathlib.Path);p.add_argument('runtime',type=pathlib.Path);p.add_argument('loader',type=pathlib.Path);p.add_argument('image',type=pathlib.Path);p.add_argument('index',type=pathlib.Path);p.add_argument('--root',type=pathlib.Path);a=p.parse_args()
    for name in ['engine','runtime','loader','image','index']:setattr(a,name,getattr(a,name).resolve())
    if a.root:run(a,a.root.resolve())
    else:
        with tempfile.TemporaryDirectory(prefix='wf-real-environment-') as root:run(a,pathlib.Path(root))
