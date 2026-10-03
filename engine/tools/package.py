#!/usr/bin/env python3
"""Create a verified, architecture-specific Engine payload. No Android runtime dependency."""
import argparse, hashlib, json, os, pathlib, shutil, subprocess, tarfile, tempfile, zipfile
ROOT = pathlib.Path(__file__).resolve().parents[2]
def digest(path):
    with open(path, 'rb') as stream: return hashlib.file_digest(stream, 'sha256').hexdigest()
def fetch(entry, cache):
    cache.parent.mkdir(parents=True, exist_ok=True)
    if not cache.exists():
        partial = cache.with_suffix('.part')
        try:
            subprocess.run(['curl','-fsSL','--retry','3','--connect-timeout','30','--max-time','1800','-o',str(partial),entry['url']],check=True)
            if digest(partial) != entry['archiveSha256'] or partial.stat().st_size != entry['archiveBytes']: raise ValueError('Downloaded archive identity mismatch')
            partial.replace(cache)
        finally: partial.unlink(missing_ok=True)
    if digest(cache) != entry['archiveSha256'] or cache.stat().st_size != entry['archiveBytes']: raise ValueError(f'Invalid cached archive: {cache}; inspect before removal')
    return cache

def safe(name):
    p=pathlib.PurePosixPath(name)
    if p.is_absolute() or not p.parts or any(x in ('..','.') for x in name.split('/')) or '\\' in name: raise ValueError('Unsafe payload member')
    return p

def build(arch, jar, output):
    abi={'amd64':'x86_64','arm64':'arm64-v8a'}[arch]
    codex=json.loads((ROOT/'third_party/codex/manifest.json').read_text()); jre=json.loads((ROOT/'third_party/jre/manifest.json').read_text()); claude=json.loads((ROOT/'third_party/claude-code/manifest.json').read_text())
    output.mkdir(parents=True,exist_ok=True)
    with tempfile.TemporaryDirectory(dir=output) as temp:
        tree=pathlib.Path(temp); files=[]
        def put(name, data, executable=False):
            safe(name); p=tree/name;p.parent.mkdir(parents=True,exist_ok=True)
            with p.open('wb') as dest:
                if isinstance(data,bytes): dest.write(data)
                else: shutil.copyfileobj(data,dest)
            files.append({'path':name,'size':p.stat().st_size,'sha256':digest(p),'executable':executable})
        c=codex['abis'][abi]; archive=fetch(c,ROOT/f'third_party/.cache/engine/codex/{codex["version"]}/{arch}.tar.gz')
        with tarfile.open(archive) as t:
            member=t.getmember(c['member'])
            if not member.isfile(): raise ValueError('Codex must be regular file')
            put('codex/bin/codex',t.extractfile(member),True)
        if files[-1]['sha256']!=c['sha256'] or files[-1]['size']!=c['bytes']: raise ValueError('Codex binary identity mismatch')
        j=jre['architectures'][arch]; archive=fetch(j,ROOT/f'third_party/.cache/engine/jre/{jre["version"]}/{arch}.tar.gz')
        with tarfile.open(archive) as t:
            # Resolve upstream license symlinks as ordinary files. Extraction never creates links.
            for m in t.getmembers():
                path=safe(m.name); relative=path.relative_to(j['root'])
                if not relative.parts or m.isdir(): continue
                if not (m.isfile() or m.issym() or m.islnk()): raise ValueError('Unexpected JRE archive type')
                if m.issym():
                    resolved=path.parent.joinpath(m.linkname)
                    normalized=os.path.normpath(str(resolved));safe(normalized)
                    if not normalized.startswith(j['root']+'/'): raise ValueError('Escaping JRE symlink')
                put('jre/'+str(relative),t.extractfile(m),bool(m.mode & 0o111) if m.isfile() else False)
        with jar.open('rb') as stream: put('chat/workflow-chat.jar',stream)
        for name in ['codex','jre','claude-code']:
            for file in ['manifest.json','LICENSE']:
                put(f'notices/{name}-{file}',(ROOT/f'third_party/{name}/{file}').read_bytes())
        tools=[{'id':'codex','version':codex['version'],'binary':'/opt/workflow/tools/codex/bin/codex'}, {'id':'jre','version':jre['version'],'binary':'/opt/workflow/tools/jre/bin/java'}, {'id':'chat','version':'1.0.0','binary':'/opt/workflow/tools/chat/workflow-chat.jar'}]
        metadata={'format':1,'architecture':arch,'tools':tools,'optional':{'claude':{'version':claude['version'],**claude['abis'][abi]}},'files':sorted(files,key=lambda x:x['path'])}
        archive=output/'tools.zip.part'
        with zipfile.ZipFile(archive,'w',compression=zipfile.ZIP_DEFLATED,compresslevel=6) as z:
            for item in metadata['files']:
                info=zipfile.ZipInfo(item['path'],date_time=(2026,1,1,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED;info.external_attr=0o100644<<16
                with (tree/item['path']).open('rb') as source,z.open(info,'w') as dest: shutil.copyfileobj(source,dest)
        archive.replace(output/'tools.zip');metadata['sha256']=digest(output/'tools.zip');metadata['size']=(output/'tools.zip').stat().st_size
        (output/'tools.json').write_text(json.dumps(metadata,sort_keys=True,separators=(',',':'))+'\n')
if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--architecture',choices=['amd64','arm64'],required=True);parser.add_argument('--jar',type=pathlib.Path,required=True);parser.add_argument('--output',type=pathlib.Path,required=True);a=parser.parse_args();build(a.architecture,a.jar,a.output)
