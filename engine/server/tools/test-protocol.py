#!/usr/bin/env python3
"""Black-box JSONL contract, restart and concurrent build tests; no Android or host settings."""
import argparse, base64, json, pathlib, queue, subprocess, tempfile, threading, time, os

class Client:
    def __init__(self, engine, root, extra=()):
        self.p = subprocess.Popen([str(engine), 'serve', '--root', str(root), *map(str, extra)], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        self.q = queue.Queue(); self.pending = {}; self.sequence = 0
        def reader():
            for line in self.p.stdout: self.q.put(json.loads(line))
        threading.Thread(target=reader, daemon=True).start()
    def send(self, method, params=None, id=None):
        if id is None: self.sequence += 1; id = self.sequence
        self.p.stdin.write(json.dumps({'jsonrpc':'2.0','id':id,'method':method,'params':params or {}}, ensure_ascii=False)+'\n'); self.p.stdin.flush(); return id
    def receive(self, id, timeout=10):
        if id in self.pending: return self.pending.pop(id)
        deadline=time.monotonic()+timeout
        while True:
            r=self.q.get(timeout=max(0.01,deadline-time.monotonic()))
            if r['id']==id:return r
            self.pending[r['id']]=r
    def call(self, method, params=None, error=None, timeout=10):
        r=self.receive(self.send(method,params),timeout)
        if error:
            assert r['error']['data']['kind']==error,r
            return r['error']
        assert 'result' in r,r
        return r['result']
    def hello(self):
        return self.call('hello',{'protocol':'workflow.workspace/1','clientId':'host-contract'})
    def command(self,name,args=None):return self.call('workspace.command',{'name':name,'args':args or {}})['value']
    def close(self):
        self.p.stdin.close(); self.p.wait(timeout=10)
        stderr=self.p.stderr.read(); assert self.p.returncode==0,stderr
        assert not stderr,stderr


def protocol(engine, root):
    bad=Client(engine,root)
    bad.call('workspace.snapshot',error='protocol_mismatch'); bad.close()
    bad=Client(engine,root)
    bad.call('hello',{'protocol':'old/v0','clientId':'x'},error='protocol_mismatch'); bad.close()
    c=Client(engine,root); assert c.hello()['engineVersion']=='1.0.0'
    unknown=c.call('unknown.vendor/method',{},error='unknown_method');assert unknown['code']==-32601
    special='opaque 🦀 18446744073709551615'; assert c.receive(c.send('workspace.snapshot',id=special))['id']==special
    huge=2**120+37;assert c.receive(c.send('workspace.snapshot',id=huge))['id']==huge
    snap=c.call('workspace.snapshot'); wait=c.send('workspace.watch',{'afterRevision':snap['revision'],'timeoutMs':30000})
    begin=time.monotonic(); session=c.command('createSession',{'name':'协议测试'}); assert time.monotonic()-begin<2
    assert c.receive(wait)['result']['revision']>snap['revision']
    assert c.command('createFile',{'path':'a.txt','data':base64.b64encode(b'base').decode()})['kind']=='done'
    disk=c.command('openFile',{'path':'a.txt'})['disk']
    c.command('editFile',{'path':'a.txt','text':'draft 🦀','shown':disk})
    c.command('applyLayout',{'sessionId':session,'op':{'type':'open','target':{'kind':'file','path':'a.txt'}}})
    assert c.command('archiveSession',{'id':session})['kind']=='needsDecision'
    payload=bytes(range(256))*600; u=c.call('files.upload.begin',{'path':'media/data.bin','size':len(payload)})['uploadId']
    for offset in range(0,len(payload),65536):
        data=payload[offset:offset+65536]
        assert c.call('files.upload.chunk',{'uploadId':u,'offset':offset,'data':base64.b64encode(data).decode()})['nextOffset']==offset+len(data)
    assert c.call('files.upload.commit',{'uploadId':u})['kind']=='done'
    result=b'';offset=0
    while True:
        data=c.call('files.read',{'path':'media/data.bin','offset':offset,'length':65536}); result+=base64.b64decode(data['data']);offset=data['nextOffset']
        if data['eof']:break
    assert result==payload
    u=c.call('files.upload.begin',{'path':'.workspace/services/proxy/assets/provider.dat','size':3})['uploadId']
    c.call('files.upload.chunk',{'uploadId':u,'offset':0,'data':base64.b64encode(b'\x00\xffx').decode()})
    assert c.call('files.upload.commit',{'uploadId':u})['kind']=='done'
    assert base64.b64decode(c.call('files.read',{'path':'.workspace/services/proxy/assets/provider.dat'})['data'])==b'\x00\xffx'

    u=c.call('files.upload.begin',{'path':'race','size':0})['uploadId'];(root/'race').write_text('never replace')
    assert c.call('files.upload.commit',{'uploadId':u})['kind']=='failed';assert(root/'race').read_text()=='never replace'
    c.call('files.upload.cancel',{'uploadId':u})
    c.call('files.read',{'path':'../escape'},error='invalid_params')
    c.call('files.read',{'path':'.workspace/state/workspace.json'},error='invalid_params')
    c.call('documents.write',{'namespace':'chat','key':'conversations','document':'{"unknownField":42}','expectedRevision':0})
    c.call('documents.write',{'namespace':'chat','key':'conversations','document':'stale','expectedRevision':0},error='conflict')
    assert c.call('documents.read',{'namespace':'chat','key':'conversations'})['document']=='{"unknownField":42}'
    c.call('documents.read',{'namespace':'../x','key':'y'},error='invalid_params')
    service=c.call('documents.write',{'namespace':'services.proxy','key':'config.yaml','document':'mode: direct\n','expectedRevision':0})
    assert (root/'.workspace/services/proxy/config.yaml').read_text()=='mode: direct\n'
    snap=c.command('openFile',{'path':'.workspace/services/proxy/config.yaml'})
    c.command('editFile',{'path':'.workspace/services/proxy/config.yaml','text':'mode: rule\n','shown':snap['disk']})
    assert c.command('saveFile',{'path':'.workspace/services/proxy/config.yaml'})['kind']=='saved'
    c.call('documents.write',{'namespace':'services.proxy','key':'config.yaml','document':'stale','expectedRevision':service['revision']},error='conflict')
    assert c.call('documents.read',{'namespace':'services.proxy','key':'config.yaml'})['document']=='mode: rule\n'

    c.call('services.report',{'serviceId':'network','state':{'dnsServers':['127.0.0.1','::1'],'connected':False}})
    assert (root/'.workspace/environment/network/resolv.conf').read_text()=='nameserver 127.0.0.1\nnameserver ::1\n'
    c.call('services.report',{'serviceId':'network','state':{'dnsServers':['bad\nsearch example.invalid'],'connected':True}},error='invalid_params')
    c.call('services.report',{'serviceId':'proxy','state':{'running':False,'reason':'permission_missing'}})
    assert c.call('services.status')['services']['proxy']['state']['running'] is False
    c.call('process.spawn',{'argv':['/bin/echo','must not run on host']},error='environment_unavailable')
    revision=c.call('workspace.snapshot')['revision'];c.close()
    c=Client(engine,root);c.hello();snap=c.call('workspace.snapshot');assert snap['revision']>=revision
    assert snap['state']['drafts']['a.txt']['text']=='draft 🦀';assert c.call('documents.read',{'namespace':'chat','key':'conversations'})['revision']==1
    c.close()
    print('JSONL: strict hello, opaque IDs, unknown methods, concurrent watch, upload/read limits, no overwrite, document CAS, service reports, restart drafts: PASS')

MOCK_RUNTIME=r'''#!/usr/bin/env python3
import sys,os,json,pathlib,time
args=sys.argv[1:]
if args[0]=='install':
 time.sleep(1.5)
 g=pathlib.Path(args[args.index('--target')+1]);(g/'rootfs').mkdir(parents=True);sys.exit(0)
if args[0]=='verify':sys.exit(0)
if args[0]=='run':
 command=args[args.index('--')+1:]
 if command[0]=='/usr/bin/env':
  command=command[1:]
  while command and '=' in command[0]:command=command[1:]
 if command[0].endswith('envctl') and command[1]=='verify-many':
  py=json.loads(command[2]);node=json.loads(command[3]);print(json.dumps({'profile':'fixture-profile','python':py,'node':node,'packages':{p:'1.0' for p in command[4:]}}));sys.exit(0)
 if command[0]=='/bin/sleep':time.sleep(float(command[1]));sys.exit(0)
 if command[-1]=='FAIL_BUILD':sys.exit(42)
 sys.exit(0)
sys.exit(125)
'''
def lifecycle(engine,root):
    runtime=root/'fixture-runtime.py';runtime.write_text(MOCK_RUNTIME);runtime.chmod(0o755)
    index=root/'image.json';index.write_text(json.dumps({'sha256':'a'*64,'metadata':{'format':'workflow-image','formatVersion':2,'profile':'workspace','type':'debian-trixie','typeVersion':1,'defaults':{'python':['3.14'],'node':['24']},'environment':{},'stores':{}}}))
    image=root/'image.tar.zst';image.write_bytes(b'fixture - runtime mocks only installation')
    c=Client(engine,root,['--runtime',runtime,'--image',image,'--image-index',index]);c.hello()
    begin=time.monotonic();status=c.call('environment.reconcile');assert status['phase'] in ('installing','building')
    c.command('createSession',{'name':'responsive during image install'});assert time.monotonic()-begin<1.0
    for _ in range(100):
        status=c.call('environment.status')
        if status['phase'] not in ('installing','building'):break
        time.sleep(.05)
    assert status['phase']=='ready',status;generation=status['active']['generation']
    p=c.call('process.spawn',{'argv':['/bin/sleep','20']})['processId']
    c.call('environment.restart');assert c.call('process.wait',{'processId':p})['running']
    (root/'.workspace/env.json').write_text(json.dumps({'version':1,'env':{'CHANGE':'1'}}))
    c.call('environment.reconcile')
    for _ in range(100):
        status=c.call('environment.status')
        if status['phase']=='needs_restart':break
        time.sleep(.05)
    assert status['phase']=='needs_restart' and status['active']['generation']==generation,status
    spawning=c.send('process.spawn',{'argv':['/bin/sleep','20']})
    restarting=c.send('environment.restart')
    c.command('createSession',{'name':'responsive during restart'})
    spawned=c.receive(spawning);assert 'result' in spawned,spawned
    status=c.receive(restarting)['result'];assert status['phase']=='ready',status
    c.call('process.stop',{'processId':spawned['result']['processId'],'force':True})
    c.call('process.wait',{'processId':spawned['result']['processId'],'timeoutMs':1000})
    assert status['active']['generation']!=generation and not c.call('process.wait',{'processId':p})['running']
    generation=status['active']['generation']
    spec={'version':1,'python':['3.14','3.13'],'node':[],'packages':[],'env':{},'post_scripts':[{'id':'fails','run':'FAIL_BUILD','user':'root'}]}
    (root/'.workspace/env.json').write_text(json.dumps(spec));c.call('environment.reconcile')
    for _ in range(100):
        status=c.call('environment.status')
        if status['phase']=='failed':break
        time.sleep(.05)
    assert status['phase']=='failed' and status['usable'] and status['active']['generation']==generation,status
    assert '42' in status['error'];old_job=status['jobId'];c.call('environment.reconcile');assert c.call('environment.status')['jobId']==old_job
    c.call('environment.reconcile',{'retry':True});assert c.call('environment.status')['jobId']!=old_job
    # Do not terminate midway through fixture build.
    for _ in range(100):
        if c.call('environment.status')['phase']=='failed':break
        time.sleep(.05)
    c.close();print('Lifecycle fixture: nonblocking build, language arrays, script failure retains usable generation, explicit retry, pending restart stops managed jobs: PASS')


def processes(engine,root,runtime,loader,rootfs):
    # The real Rust runtime executes every tested command. No host shell fallback.
    private=root/'.workspace/environment';g=private/'generations/fixture';g.mkdir(parents=True)
    (g/'rootfs').symlink_to(rootfs.resolve(),target_is_directory=True)
    (private/'environment.json').write_text(json.dumps({'format':1,'status':'ready','active':{'generation':'fixture','profile':'fixture','config':{'env':{}}},'pending':None,'failure':None}))
    c=Client(engine,root,['--runtime',runtime,'--loader',loader]);c.hello()
    vendor='{"jsonrpc":"2.0","id":"approval-opaque-🚀","method":"vendor/unknown/request","params":{"opaque":true}}'
    p=c.call('process.spawn',{'argv':['/bin/cat'],'cwd':'/workspace','terminal':False})['processId']
    raw=(vendor+'\n').encode();c.call('process.write',{'processId':p,'data':base64.b64encode(raw).decode()})
    actual=b'';offset=0
    for _ in range(10):
        r=c.call('process.read',{'processId':p,'stream':'stdout','offset':offset,'waitMs':1000});actual+=base64.b64decode(r['data']);offset=r['nextOffset']
        if actual==raw:break
    assert actual==raw,(actual,raw)
    # cat remains alive: forwarding an approval-shaped frame generated no engine response.
    assert c.call('process.wait',{'processId':p,'timeoutMs':0})['running']
    c.call('process.stop',{'processId':p,'force':True});assert not c.call('process.wait',{'processId':p,'timeoutMs':30000})['running']
    p=c.call('process.spawn',{'argv':['/bin/sh','-c','stty size; printf "work:%s\\n" "$PWD"; test ! -e /workspace/.workspace/state/workspace.json'],'terminal':True,'rows':37,'columns':101})['processId']
    assert c.call('process.wait',{'processId':p,'timeoutMs':30000})['exitCode']==0
    r=c.call('process.read',{'processId':p,'offset':0,'waitMs':1000});text=base64.b64decode(r['data']);assert b'37 101' in text and b'work:/workspace' in text,text
    p=c.call('process.spawn',{'argv':['/bin/sh','-c','read line; stty size'],'terminal':True})['processId']
    c.call('process.resize',{'processId':p,'rows':39,'columns':99})
    c.call('process.write',{'processId':p,'data':base64.b64encode(b'go\n').decode()})
    assert c.call('process.wait',{'processId':p,'timeoutMs':30000})['exitCode']==0
    r=c.call('process.read',{'processId':p,'offset':0,'waitMs':1000});assert b'39 99' in base64.b64decode(r['data']),r
    p=c.call('process.spawn',{'argv':['/bin/sh','-c','head -c 5242880 /dev/zero'],'terminal':False})['processId']
    assert c.call('process.wait',{'processId':p,'timeoutMs':30000})['exitCode']==0
    r=c.call('process.read',{'processId':p,'offset':0,'maxBytes':65536,'waitMs':1000});assert r['startOffset']>0 and len(base64.b64decode(r['data']))==65536,r
    c.close();print('Real Rust runtime: stdio/UTF-8/vendor approval bytes preserved, PTY size/cwd, private state masking, bounded output clipping, stop/wait: PASS')

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('engine',type=pathlib.Path);p.add_argument('--runtime',type=pathlib.Path);p.add_argument('--loader',type=pathlib.Path);p.add_argument('--rootfs',type=pathlib.Path);a=p.parse_args();a.engine=a.engine.resolve()
    with tempfile.TemporaryDirectory(prefix='workflow-server-') as path:protocol(a.engine,pathlib.Path(path))
    with tempfile.TemporaryDirectory(prefix='workflow-env-') as path:lifecycle(a.engine,pathlib.Path(path))
    if a.runtime and a.loader and a.rootfs:
        with tempfile.TemporaryDirectory(prefix='workflow-process-') as path:processes(a.engine,pathlib.Path(path),a.runtime.resolve(),a.loader.resolve(),a.rootfs.resolve())
