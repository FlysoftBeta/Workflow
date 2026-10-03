#!/usr/bin/env bash
# Create the local, passwordless PKCS#12 signing key once. Never replaces an existing key.
set -euo pipefail
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
target=${1:-"$repo/artifacts/signing/Workflow-release.p12"}
umask 077
mkdir -p -- "$(dirname -- "$target")"
if [[ -e "$target" ]]; then
  printf 'Signing key already exists: %s\n' "$target"
  exit 0
fi
temporary=$(mktemp -d "$(dirname -- "$target")/.key.XXXXXX")
trap 'rm -rf -- "$temporary"' EXIT
openssl req -x509 -newkey rsa:3072 -sha256 -nodes -days 10000 \
  -subj '/CN=Workflow/O=Workflow' \
  -keyout "$temporary/key.pem" -out "$temporary/cert.pem" > /dev/null 2>&1
openssl pkcs12 -export -name workflow -passout pass: \
  -inkey "$temporary/key.pem" -in "$temporary/cert.pem" -out "$temporary/release.p12"
# ln fails if another invocation created the target in the meantime; no key is overwritten.
ln -- "$temporary/release.p12" "$target"
printf 'Created local signing key: %s\n' "$target"
