# Customized environment images

`image/` builds the complete Debian-based workspace environment for amd64 and arm64. Both release
ABIs use the customized `workspace` profile; the `base` profile exists only for image and low-level
runtime fixtures. It is not an app fallback. The Engine verifies and installs these images, then
owns environment generations, home stores and process lifetime.

[`versions.env`](versions.env) pins the base image digests, downloaded toolchain inputs and default
language versions. `build.sh` coordinates a rootless Podman build. `guest/provision.sh` installs the
workspace tools, `guest/envctl` applies environment declarations, and the other `guest/` scripts
capture and pack the canonical filesystem. The prune/canonical lists and placeholder network
files are source inputs. `with-cross.sh` supplies build-only QEMU inside a private namespace for
the other architecture; it does not register a global host interpreter.

`wfimage/` is the Python image-format implementation. It separates manifest/tree capture, attribute
tables, tar framing, packing, verification and store handling. Its `fixture` command deliberately
generates valid and invalid conformance inputs for installer tests. Those generated archives are
test data, never application images. `tests/` covers the format and includes opt-in Podman and
toolchain-profile acceptance scripts.

Run format checks from the repository root without building an image:

```sh
PYTHONPATH=image python3 -m unittest discover -s image/tests -v
```

Build a customized image explicitly with `image/build.sh --profile workspace --arch amd64` or
`--arch arm64`. Images, indexes, package inventories and build logs go into ignored
`artifacts/image/<arch>/`; `--out` selects another output directory. Do not put generated archives
or container exports in source control. The
[environment guide](../docs/implementation/environment.md) defines the image/runtime contract,
and the [testing guide](../docs/development/testing.md) records the required acceptance matrix.
