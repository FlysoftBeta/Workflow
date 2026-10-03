# ABI build names and packaging verification

The Android flavors now identify executable compatibility: `arm64` selects `arm64-v8a` libraries and the customized arm64 image, while `x86_64` selects x86_64 libraries and the customized amd64 image. Standard `assembleDebug` and `assembleRelease` each build both flavors. Release delivery, test-suite commands and frozen instrumentation APK paths use the same naming. The [build guide](../development/building.md) documents host preparation, image inputs, signing and output locations.

Implementation commit `daa7198` was integrated with the independently checked documentation and checkout fixes at `9ce8dc9618daaa4bca4a4b39d03de41c5ea126db`. On Linux x86_64 with JDK 25, Rust 1.93.1 and Android Build-Tools 37.0.0, the following command passed in 7 minutes 51 seconds:

```sh
tools/with-build-lock.sh ./gradlew \
  :app:assembleDebug :app:assembleRelease :app:assembleX86_64DebugAndroidTest
```

All four app APKs were inspected under the build lease. Each contained exactly its declared native ABI, the Engine/runtime/loader and other required executables, the matching customized `workspace` image with valid size and SHA-256, and offline terminal assets. Version remained 1.0.0/versionCode 10000 with minSdk 28. Debug signatures were verified at API 28; both Release signatures passed v1, v2 and v3 verification starting at API 21, and Release packages were not debuggable. The raw build log, package inspection, signature results, APK sizes and hashes are retained locally under `artifacts/abi-build-names/`. Existing release-delivery evidence was not overwritten.

The infrastructure suite passed all 17 tests in coordinator run `20261003T065351Z-fe5b0c49`. After integration, `android-apk` passed in run `20261003T070502Z-e63ea70a` and saved the real app/test pair from the new x86_64 paths. Documentation passed independently in contributor run `20261003T070134Z-4c3fadbb` and after integration in coordinator run `20261003T070454Z-7bdcbb67`. Source fingerprints and outputs accompany those runs under `artifacts/workflow/runs/`.

A fresh task checkout also established that the cache symlink is ignored and the Windows Gradle wrapper retains CRLF working bytes without an immediate Git diff. The wrapper's script content is unchanged; only its stored line endings were normalized. Both temporary task checkouts were archived after integration or verification, preserving their branches and records.

These results establish build naming, packaging and coordination behavior. No emulator or physical device was used for this change, so this report makes no new claim about startup, connection or runtime acceptance. Historical reports retain their original task names and evidence.
