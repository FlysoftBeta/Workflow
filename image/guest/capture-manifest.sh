#!/bin/bash
# Print the ownership manifest of a guest root as NUL-terminated records (format: image/wfimage/manifest.py):
#   <raw st_mode hex> <uid> <gid> <dev>:<ino> <nlink> <major hex>:<minor hex> <./path>
# Runs inside the guest as (virtual) root, so ownership and mode are what the guest sees. Needs only
# find(1) and stat(1). Usage: capture-manifest.sh [ROOT [PRUNE_LIST]] > manifest
set -euo pipefail
root=${1:-/}
prune_list=${2:-$(dirname "${BASH_SOURCE[0]}")/prune.list}
patterns=() exceptions=()
while IFS= read -r line || [ -n "$line" ]; do
    case $line in
        '' | '#'*) ;;
        '!'*) exceptions+=(-o -path "${line#!}") ;;
        *) patterns+=(-o -path "$line") ;;
    esac
done < "$prune_list"
prune=(-false "${patterns[@]}")
if [ ${#exceptions[@]} -gt 0 ]; then prune=(\( "${prune[@]}" \) ! \( -false "${exceptions[@]}" \)); fi
cd "$root"
exec find . \( "${prune[@]}" \) -prune -o ! -type s -exec stat --printf '%f %u %g %d:%i %h %t:%T %n\0' {} +
