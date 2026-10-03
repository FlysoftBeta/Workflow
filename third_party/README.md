# Pinned upstream inputs

Each package directory records an exact upstream version, download locations, integrity hashes and
license terms. Preserve those records when changing a dependency. Downloaded executables and
archives belong only in the ignored `.cache/`; Gradle and image tools verify the pinned hashes
before use. A cache mismatch is an error to inspect, not permission to accept different bytes.

`codex/` and `mihomo/` describe executables staged into generated Android native-library assets.
`claude-code/` describes the pinned CLI downloaded on demand inside the environment, with its
upstream license terms. `jetbrains-mono/` and `material-symbols/` record the font and icon inputs.
`qemu-user/` supplies only the image builder's cross-architecture runner; its executable enters
neither an image nor an APK. See its [README](qemu-user/README.md) for the namespace boundary.

The web terminal dependencies are pinned separately in `web/package.json` and `web/package-lock.json`;
[`web/vendor.mjs`](../app/web/vendor.mjs) writes their copied licenses and digest manifest alongside the
offline assets. Agent protocol schema snapshots belong in `agent/src/main/resources/protocol/`,
where adapter coverage is checked against their inventories.

Update a manifest, required license/notice files and the consuming implementation together. Keep
download caches and generated packaging output out of the source tree. The
[dependency guide](../docs/development/dependencies.md) is the repository-wide version inventory;
the [testing guide](../docs/development/testing.md) covers packaging and release verification.
