# Environment image format

Workflow images use `formatVersion: 2`. The archive is a portable description of a guest filesystem and its virtual attributes; host extraction permissions are not the guest's authority. The runtime validates the entire representation before committing an installed generation. The [environment document](environment.md) explains how the Server uses those generations.

## Archive and index

`image.tar.zst` contains one zstd stream, possibly with multiple frames. Every frame carries a content checksum and has a window no larger than 128 MiB (`windowLog <= 27`), so the decoder needs no exceptional window configuration. Decompressed contents are POSIX tar using ustar and local PAX extensions.

`image.json` accompanies the archive and has `sha256`, `size`, and `metadata` fields. Its metadata copies the archive's `metadata.json` so packaging and bootstrap can inspect it before extraction. In an installed APK, the APK signature authenticates this index and therefore the expected archive digest.

Tar member order is fixed: `metadata.json`, `attributes.tsv`, then the rootfs members in attribute-table order. No other members are accepted. Metadata is limited to 64 KiB and the attribute table to 64 MiB.

Only ordinary files (`0`), directories (`5`), symbolic links (`2`), and a local PAX header (`x`) modifying the next member are allowed. Global PAX (`g`), GNU long-name/link/sparse extensions (`L`, `K`, `S`), hard links (`1`), and special files (`3`, `4`, `6`) are rejected. Hard links and special files are represented in the attribute table instead. PAX permits only `path`, `linkpath`, `size`, and `mtime`, with UTF-8 values.

The root member is named `rootfs`; other member names are `rootfs/` followed by the guest path without its initial slash. Reading removes one trailing slash from directory names. Paths must be valid UTF-8 without empty, `.` or `..` components. Tar uid/gid are zero and uname/gname are empty. Tar mode is `permissions & 0777` for inspection only; it must not control installed guest attributes. Integer-second guest mtime is preserved because bytecode caches and build tools depend on it. Symlink targets remain verbatim, including guest-absolute targets, and extraction never follows them.

## Metadata

`metadata.json` is a UTF-8 object. Readers ignore unknown fields; additive fields do not require a version increase, while incompatible changes do. Version 1 images are rejected rather than migrated.

| Field | Value or meaning |
| --- | --- |
| `format` | `"workflow-image"` |
| `formatVersion` | `2` |
| `type` | `"debian-trixie"`, selecting the runtime/environment variant |
| `typeVersion` | `1` for the variant defined below |
| `profile` | `"workspace"` for a deliverable environment, or `"base"` for builds and runtime tests only |
| `architecture` | `"arm64"` or `"amd64"`, matching Android `arm64-v8a` or `x86_64` |
| `createdAt` | RFC 3339 UTC timestamp |
| `base` | `{reference,indexDigest,manifestDigest,platform}` for the pinned OCI base |
| `rootfs` | `{entries,members,regularBytes}`: attribute rows, rootfs tar members, and regular bytes excluding hard-link duplicates |
| `attributes` | `{path:"attributes.tsv",version:1,rows,size,sha256}` |
| `requires` | Required runtime capabilities; unknown or unavailable entries cause rejection before extraction |
| `stores` | Workspace-only map `{guestPath:{store,seed}}`; seed is `if-absent` or `merge` |
| `user` | Workspace-only `{name,uid,gid,home,shell}` |
| `environment` | Workspace-only default variables for non-login processes |
| `toolchains` | Workspace-only `{uv,nvm,python,node,profile}` describing seeded versions and profile |
| `defaults` | Workspace-only `{python,node}` default language specifications |
| `provision` | Workspace-only `{sha256,builder}`, where builder is `podman` or `engine` |

Every image requires `virtual-ownership` and `virtual-mode`. `hardlink-emulation` is required when the table contains `h` rows, `virtual-special-files` for `c`, `b`, or `p` rows, and `store-seeds` when `stores` is present. Ownership and full guest mode bits live in the virtual attribute representation rather than relying on host filesystem support.

## Attribute table

`attributes.tsv` is ASCII with newline-terminated rows. Its first line is `#workflow-attributes 1`; each following row has six fields separated by single tabs:

```text
path    type    uid    gid    mode    extra
```

| Field | Encoding |
| --- | --- |
| `path` | Absolute guest path, `/` for root; bytes `0x21..0x7E` except `%` remain literal, and every other byte is uppercase `%XX` |
| `type` | `d` directory, `f` regular file, `l` symlink, `h` hard link, `c` character device, `b` block device, or `p` FIFO |
| `uid`, `gid` | Decimal integers in `0..4294967294` |
| `mode` | Four octal permission digits `0000..7777` without type bits; symlinks always use `0777` |
| `extra` | Encoded primary path for `h`; decimal `major,minor` for `c` or `b`; otherwise `-` |

Rows are strictly ordered by the decoded path's byte sequence, with no duplicates. The first row is `/` of type `d`, and each non-root entry has a preceding directory parent. The `d`, `f`, and `l` rows correspond one-to-one with rootfs tar members in the same order and with matching types. `h`, `c`, `b`, and `p` have no tar member.

A hard-link row references a preceding regular-file row, which is the group's lexicographically first path. All group members repeat the same uid, gid, and mode and share its contents. Hard links to directories, symlinks, or special files are invalid. Counts, byte length, and SHA256 must match metadata.

Every `stores` prefix is a directory row. Entries below it, excluding the prefix itself, may only be directories, regular files, or symlinks; they belong to the default user and have no setuid, setgid, or sticky bits. No hard link can lie below or point into a store. Sockets cannot be represented and are omitted when packing. This attribute-table version has no general extended-attribute representation; file capabilities such as `security.capability` are unsupported and must report an honest error.

## Safe installation and commit

Before extracting rootfs contents, the installer validates the format, version, known variant, supported variant version, intended profile, matching architecture, and every required capability. It reads and verifies the attribute-table header, path/parent ordering, hard-link relationships, and metadata digest/counts. Space and size checks use declared regular bytes and entries.

Extraction streams into a staging generation and matches each tar member against the corresponding table row. Missing, extra, reordered, or mismatched members fail the entire generation. All filesystem operations are relative to already opened directory descriptors, using `openat` with `O_NOFOLLOW|O_EXCL`, `mkdirat`, and `symlinkat`. Archive-created symlinks are never followed. Host device nodes and FIFOs are not created, and host files never receive setuid, setgid, or sticky bits. Host modes ensure runtime readability, writability, and directory traversal; virtual attributes express guest permissions.

The installer records uid, gid, mode, and special-file information in its `user.workflow.*` xattrs and journal, and routes hard-link rows through hard-link emulation. It preserves mtime. Entries beneath store prefixes go to `<generation>/seeds/<store>/<relative-path>` instead of the rootfs and attribute store. Their host modes use `mode & 0777`, with owner rwx added to directories. Prefix directories remain rootfs mount points. The Server later applies the declared seed policy to persistent stores.

Only after the complete stream digest matches `image.json` can the installer commit the generation. Any validation or extraction failure removes staging and leaves the active generation untouched.

## Debian trixie variant, version 1

The base is `docker.io/library/debian:13-slim`, pinned by OCI digest in `image/versions.env`. Provisioning installs sudo, curl, wget, git, nano, unzip, zip, rsync, ca-certificates, gnupg, locales, tzdata, build-essential, and cmake, plus xz-utils for nvm downloads, zstd for guest-side packing, and jq for envctl JSON handling. It generates `en_US.UTF-8`.

The default user is `work` at 1000:1000 with `/home/work` and `/bin/bash`. `/etc/sudoers.d/workflow` grants `NOPASSWD: ALL`, so `sudo su` enters virtual guest root without a password. uv and uvx are under `/usr/local/bin`, and nvm's program is `/usr/local/lib/nvm/nvm.sh`, loaded on demand in interactive shells.

`/opt/toolchains` belongs to `work`. Python installations use `uv/python/cpython-X.Y.Z-*` through `UV_PYTHON_INSTALL_DIR`; Node uses `nvm/versions/node/vX.Y.Z` with `NVM_DIR=/opt/toolchains/nvm`. Profiles contain default `python` and `node` entries and complete `pythons/<version>` and `nodes/<version>` selections. `active` points to the selected profile. `UV_PYTHON_PREFERENCE=system` makes uv use the active PATH Python unless a project's `.python-version` or `requires-python` chooses otherwise.

The profile script, sudo `secure_path`, and metadata environment agree on this PATH:

```text
/opt/toolchains/active/python/bin:/opt/toolchains/active/node/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
```

`/home/work` maps to store `home/work` with `if-absent` seeding; `/opt/toolchains` maps to `toolchains` with `merge`. The guest step interface is `/usr/local/libexec/workflow/envctl`. `/usr/local/bin/codex` links to `/opt/workflow/bundled/libcodex.so`, which the signed APK supplies and the Server mounts. The image includes localhost hosts entries, hostname `workflow`, and an empty resolver file for the Server's runtime binding.
