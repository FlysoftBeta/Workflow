#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../../.."
mkdir -p artifacts/workspace-engine/oracle
stdlib=$(rg --files "$HOME/.gradle/caches/modules-2/files-2.1/org.jetbrains.kotlin/kotlin-stdlib/2.4.20" | rg '/kotlin-stdlib-2.4.20\.jar$' | head -1)
junit=$(rg --files "$HOME/.gradle/caches/modules-2/files-2.1/junit/junit/4.13.2" | rg '\.jar$' | head -1)
cp="core/build/classes/kotlin/main:core/build/classes/kotlin/test:$stdlib:$junit"
javac -cp "$cp" -d artifacts/workspace-engine/oracle engine/server/tools/LayoutOracle.java
java -cp "artifacts/workspace-engine/oracle:$cp" LayoutOracle "${1:-400}" > artifacts/workspace-engine/layout-oracle.jsonl
printf 'Oracle fixtures: %s\n' "$(wc -l < artifacts/workspace-engine/layout-oracle.jsonl)"
