#!/usr/bin/env bash
# Builds both ABI-specific 1.0.0 releases and verifies their signatures and runtime assets.
set -euo pipefail
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd -- "$repo"
if [[ ${WORKFLOW_BUILD_LOCK_HELD:-0} != 1 ]]; then
  exec "$repo/tools/with-build-lock.sh" "$0" "$@"
fi
version=1.0.0
output="$repo/artifacts/delivery/$version"
sdk=${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}
if [[ -z "$sdk" && -f local.properties ]]; then
  sdk=$(sed -n 's/^sdk.dir=//p' local.properties | head -1)
fi
signer="$sdk/build-tools/37.0.0/apksigner"
if [[ ! -x "$signer" ]]; then
  printf 'Android SDK build-tools 37.0.0 required (set ANDROID_HOME).\n' >&2
  exit 1
fi
if [[ ! -f artifacts/signing/Workflow-release.p12 ]]; then
  printf 'Run tools/init-release-key.sh once before building a release.\n' >&2
  exit 1
fi
mkdir -p -- "$output"
# Keep both build and immutable copies under the same lock; image-profile builds share output paths.
./gradlew :app:android:assembleRelease \
  > "$output/build.log" 2>&1
for spec in arm64:arm64-v8a x86_64:x86_64; do
  flavor=${spec%%:*}
  abi=${spec#*:}
  apk="$output/Workflow-$version-$abi-release.apk"
  cp -- "app/android/build/outputs/apk/$flavor/release/android-$flavor-release.apk" "$apk"
  # Verification starting at API 21 exercises v1 as well as v2/v3; the manifest still requires API 28.
  "$signer" verify --verbose --print-certs --min-sdk-version 21 "$apk" > "$output/$abi-signature.txt"
  for scheme in v1 v2 v3; do
    if ! grep -Eq "Verified using $scheme scheme .*: true" "$output/$abi-signature.txt"; then
      printf '%s signature verification failed for %s\n' "$scheme" "$abi" >&2
      exit 1
    fi
  done
  "$sdk/build-tools/37.0.0/aapt" dump badging "$apk" > "$output/$abi-package.txt"
done
python3 - "$output" <<'PY'
import hashlib, io, json, sys, zipfile
from pathlib import Path
out = Path(sys.argv[1])
artifacts = []
for abi, arch in [('arm64-v8a', 'arm64'), ('x86_64', 'amd64')]:
    apk = out / f'Workflow-1.0.0-{abi}-release.apk'
    package = (out / f'{abi}-package.txt').read_text()
    assert "versionName='1.0.0'" in package and "versionCode='10000'" in package, abi
    assert "sdkVersion:'28'" in package and 'application-debuggable' not in package, abi
    with zipfile.ZipFile(apk) as z:
        names = set(z.namelist())
        for binary in ('workflow-engine', 'workflow-runtime', 'workflow-loader', 'mihomo', 'proxyguard'):
            # The guardian's exact packaged name is declared in its native build script.
            if binary == 'proxyguard':
                assert any(n.startswith(f'lib/{abi}/lib') and 'guard' in n for n in names), abi
            else:
                assert f'lib/{abi}/lib{binary}.so' in names, (abi, binary)
        assert {n.split('/')[1] for n in names if n.startswith('lib/')} == {abi}
        index = json.loads(z.read('assets/environment/image.json'))
        assert index['metadata']['architecture'] == arch
        with z.open('assets/environment/image.tar.zst') as image:
            digest = hashlib.file_digest(image, 'sha256').hexdigest()
        assert digest == index['sha256']
        assert z.getinfo('assets/environment/image.tar.zst').file_size == index['size']
        assert index['metadata']['profile'] == 'workspace'
        assert index['metadata']['format'] == 'workflow-image'
        assert index['metadata']['formatVersion'] == 2
        assert not any(n.endswith('/libcodex.so') for n in names)
        tools = json.loads(z.read('assets/environment/tools/tools.json'))
        assert tools['format'] == 1 and tools['architecture'] == arch
        payload = z.read('assets/environment/tools/tools.zip')
        assert len(payload) == tools['size'] and hashlib.sha256(payload).hexdigest() == tools['sha256']
        with zipfile.ZipFile(io.BytesIO(payload)) as contents:
            assert set(contents.namelist()) == {item['path'] for item in tools['files']}
            for item in tools['files']:
                data = contents.read(item['path'])
                assert len(data) == item['size'] and hashlib.sha256(data).hexdigest() == item['sha256']
            for required in ['codex/bin/codex', 'notices/codex-LICENSE', 'notices/claude-code-LICENSE']:
                assert required in contents.namelist()
        assert 'assets/web/terminal.html' in names
        assert not any(n.startswith('assets/web/chat/') for n in names)
        assert not any(n.startswith('assets/environment/bootstrap/') for n in names)
    with apk.open('rb') as stream:
        digest = hashlib.file_digest(stream, 'sha256').hexdigest()
    artifacts.append({'file': apk.name, 'sha256': digest, 'bytes': apk.stat().st_size,
                      'abi': abi, 'imageProfile': index['metadata']['profile'], 'imageSha256': index['sha256']})
(out / 'SHA256SUMS').write_text(''.join(f"{a['sha256']}  {a['file']}\n" for a in artifacts))
(out / 'release.json').write_text(json.dumps({'version': '1.0.0', 'versionCode': 10000,
    'signatures': ['v1', 'v2', 'v3'], 'artifacts': artifacts}, ensure_ascii=False, indent=2) + '\n')
print(f'Verified releases: {out}')
PY
