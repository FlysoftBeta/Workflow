# QEMU build-only user emulator

The pinned Debian `qemu-user` package provides the statically linked x86_64
`qemu-aarch64` used by `image/with-cross.sh`. Its SHA256 is verified before every
extraction. The package and executable live only in ignored `third_party/.cache`;
neither enters an image or APK. `LICENSE` preserves Debian's upstream/component
copyright record; `COPYING` contains GPL version 2.

The runner creates a rootless Podman user namespace with a private mount namespace
and mounts `binfmt_misc` there (Linux 6.7 or later). AArch64 registration has OCF
flags and is destroyed with that namespace. It never registers an interpreter in
the host's `/proc/sys/fs/binfmt_misc`, and does not require device root or change
host settings. Other build operations retain Podman's ordinary isolated runtime.
