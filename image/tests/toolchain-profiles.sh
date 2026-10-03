#!/bin/bash
# Destructive tests inside an explicitly disposable, already provisioned podman container.
# Usage: image/tests/toolchain-profiles.sh CONTAINER
set -euo pipefail
container=${1:?a disposable provisioned container is required}
E=/usr/local/libexec/workflow/envctl
W() { podman exec -u work "$container" "$@"; }
R() { podman exec "$container" "$@"; }
field() { jq -er "$1"; }
assert() { "$@" || { echo "FAIL: $*" >&2; exit 1; }; }
initial=$(W "$E" state)
default_py=$(field .python <<< "$initial")
default_node=$(field .node <<< "$initial")
default_profile=$(field .profile <<< "$initial")
assert test "$(W "$E" python "$default_py" | field .installed)" = false
assert test "$(W "$E" node "$default_node" | field .installed)" = false
py_extra=$(W "$E" python 3.13 | field .python)
node_extra=$(W "$E" node 22 | field .node)
assert test "$(W "$E" python 3.13 | field .installed)" = false
assert test "$(W "$E" node 22 | field .installed)" = false
assert test "$(W "$E" state | field .profile)" = "$default_profile"
py_specs=$(jq -cn --arg first "$default_py" --arg second "$py_extra" '[$first,$second]')
node_specs=$(jq -cn --arg first "$default_node" --arg second "$node_extra" '[$first,$second]')
many=$(W "$E" verify-many "$py_specs" "$node_specs" git)
profile=$(field .profile <<< "$many")
assert test "$(jq -c .python <<< "$many")" = "$py_specs"
assert test "$(jq -c .node <<< "$many")" = "$node_specs"
assert test "$(W "$E" verify-many "$py_specs" "$node_specs" git | field .profile)" = "$profile"
assert test "$(W "$E" state | field .profile)" = "$default_profile"
W "$E" activate "$profile"
assert test "$(W bash -lc 'python3 -c "import platform; print(platform.python_version())"')" = "$default_py"
assert test "$(W bash -lc 'node --version')" = "v$default_node"
assert test "$(W "/opt/toolchains/active/pythons/$py_extra/bin/python3" -c 'import platform; print(platform.python_version())')" = "$py_extra"
assert test "$(W "/opt/toolchains/active/nodes/$node_extra/bin/node" --version)" = "v$node_extra"
changed=$(W "$E" verify-many "$py_specs" "[\"$default_node\"]" git | field .profile)
assert test "$changed" != "$profile"
assert test "$(W "$E" state | field .profile)" = "$profile"
# Changing order is meaningful, and the previous immutable profile remains intact.
reverse=$(W "$E" verify-many "[\"$py_extra\",\"$default_py\"]" "$node_specs" | field .profile)
assert test "$reverse" != "$profile"
W "$E" activate "$reverse"
assert test "$(W bash -lc 'python3 -c "import platform; print(platform.python_version())"')" = "$py_extra"
# Supply executable system fallbacks to prove disabled defaults cannot silently reach them.
R bash -c 'printf "#!/bin/sh\necho SYSTEM_FALLBACK\n" > /usr/local/bin/python3; cp /usr/local/bin/python3 /usr/local/bin/node; chmod +x /usr/local/bin/python3 /usr/local/bin/node'
for variants in both python node; do
    py=$py_specs node=$node_specs
    case $variants in both) py='[]'; node='[]' ;; python) py='[]' ;; node) node='[]' ;; esac
    disabled=$(W "$E" verify-many "$py" "$node" | field .profile)
    W "$E" activate "$disabled"
    if [ "$py" = '[]' ]; then
        set +e
        result=$(W bash -lc 'python3 --version' 2>&1); code=$?
        set -e
        assert test "$code" = 127
        assert test "${result#*SYSTEM_FALLBACK}" = "$result"
        assert test "$(W "$E" state | jq .python)" = null
    fi
    if [ "$node" = '[]' ]; then
        set +e
        result=$(W bash -lc 'node --version' 2>&1); code=$?
        set -e
        assert test "$code" = 127
        assert test "${result#*SYSTEM_FALLBACK}" = "$result"
        assert test "$(W "$E" state | jq .node)" = null
    fi
done
R rm /usr/local/bin/python3 /usr/local/bin/node
W "$E" activate "$default_profile"
for invalid in '{}' '["3.14","3.14"]' '[3]' '["2.7"]' '["3.14"'; do
    if W "$E" verify-many "$invalid" '[]' >/dev/null 2>&1; then echo "accepted invalid $invalid" >&2; exit 1; fi
done
if W "$E" verify-many "$py_specs" '["24","24"]' >/dev/null 2>&1; then exit 1; fi
if W "$E" verify-many '["3.999.999"]' '[]' >/dev/null 2>&1; then exit 1; fi
if W "$E" verify-many "$py_specs" "$node_specs" workflow-no-such-package >/dev/null 2>&1; then exit 1; fi
assert test "$(W "$E" state | field .profile)" = "$default_profile"
state=$(W "$E" state)
assert jq -e --arg py "$py_extra" --arg node "$node_extra" '.installedPython | index($py) != null' <<< "$state"
assert jq -e --arg node "$node_extra" '.installedNode | index($node) != null' <<< "$state"
assert test "$(R readlink /usr/local/bin/codex)" = /opt/workflow/bundled/libcodex.so
printf 'PASS: installs and verification preserve active; all selected versions run; secondary/order changes yield new profiles; empty languages return 127 with no fallback; malformed/missing inputs fail; rollback and Codex alias verified.\n'
